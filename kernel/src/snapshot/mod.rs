use shared::{AdminToNode, NodeToAdmin};
use crate::net::Network;
use crate::memory::VirtualMemoryRegion;
use crate::map_region;
use crate::sched;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use x86_64::structures::paging::PageTableFlags;

pub struct Session<'a, 'b, 'c> {
    net: Network<'a, 'b, 'c>,
    page_cache: BTreeMap<u32, [u8; 4096]>,
    regs: shared::SavedRegs,
    mappings: Vec<shared::Mapping>,
    vmem_ranges: BTreeMap<u64, VirtualMemoryRegion>,
}

impl<'a, 'b, 'c> Session<'a, 'b, 'c> {
    pub fn new(net: Network<'a, 'b, 'c>) -> Self {
        Session {
            net,
            page_cache: BTreeMap::new(),
            vmem_ranges: BTreeMap::new(),
            regs: shared::SavedRegs::default(),
            mappings: Vec::new(),
        }
    }

    fn recv(&mut self) -> AdminToNode {
        let mut size_buf = [0; 4];
        self.net.recv(&mut size_buf).unwrap();
        let len = u32::from_le_bytes(size_buf);
        let mut buf = Vec::new();
        buf.resize(len as usize, 0);
        self.net.recv(&mut buf).unwrap();
        shared::serde_cbor::from_slice(&buf).expect("serde_cbor::from_slice failed")
    }

    fn send(&mut self, msg: NodeToAdmin) {
        let message = shared::serde_cbor::to_vec(&msg).unwrap();
        let len_buf = (message.len() as u32).to_le_bytes();
        self.net.send(&len_buf).unwrap();
        self.net.send(&message).unwrap();
    }

    pub fn recv_snapshot(&mut self) {
        self.send(NodeToAdmin::Ready);
        let mut pages = 0;
        loop {
            printk!("[{:04}] Collecting pages\r", pages);
            match self.recv() {
                AdminToNode::PushPage { page_id, page} => {
                    assert_eq!(page.len(), 4096);
                    let mut _page = [0; 4096];
                    _page.clone_from_slice(&page);
                    pages += 1;
                    self.page_cache.insert(page_id, _page);
                }
                AdminToNode::PushSnapshot { regs, mappings } => {
                    self.regs = regs;
                    self.mappings = mappings;
                    break;
                }
            }
        }
        printk!("[DONE] Received snapshot\n");
    }

    fn user_range(&mut self, start: u64, end: u64) -> VirtualMemoryRegion {
        let region = self.vmem_ranges.entry(start).or_insert_with(|| {
            unsafe { VirtualMemoryRegion::take_range(start, end) }
        });
        assert_eq!(region.start() as u64, start);
        assert_eq!(region.len(), end-start+1);
        region.clone()
    }

    pub fn setup_pages(&mut self) {
        let mappings = self.mappings.clone();
        for mapping in &mappings {
            let region = self.user_range(mapping.page_start, mapping.page_end);
            let mut flags = PageTableFlags::PRESENT;
            flags.set(PageTableFlags::USER_ACCESSIBLE, mapping.perm_r);
            flags.set(PageTableFlags::WRITABLE, mapping.perm_w);
            flags.set(PageTableFlags::NO_EXECUTE, !mapping.perm_x);
            let mut data = Vec::new();
            for page_id in &mapping.pages {
                data.extend_from_slice(self.page_cache.get(page_id).expect("mapping references unknown page hash"));
            }
            crate::memory::map_snapshot_region(region, flags, data);
        }
    }

    pub fn run(&mut self) -> ! {
        let shared::SavedRegs {
            rax,
            rbx,
            rcx,
            rdx,
            rdi,
            rsi,
            rbp,
            r8,
            r9,
            r10,
            r11,
            r12,
            r13,
            r14,
            r15,
            rflags,
            rip,
            rsp,
        } = self.regs;

        sched::user::start_user_task(
            crate::sched::user::SavedRegs {
                rax,
                rbx,
                rcx,
                rdx,
                rdi,
                rsi,
                rbp,
                r8,
                r9,
                r10,
                r11,
                r12,
                r13,
                r14,
                r15,
                rflags,
                rip,
                rsp,
            }
        );
    }
}