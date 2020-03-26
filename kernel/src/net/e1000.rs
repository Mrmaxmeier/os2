// this is mostly a port from redox/drivers/e1000d. some implementation details are borrowed from serenity/E1000NetworkAdapter.cpp

fn wrap_ring(index: usize, ring_size: usize) -> usize {
    (index + 1) & (ring_size - 1)
}

use crate::net::Dma;
use tinypci::PciDeviceInfo;

const CTRL: u32 = 0x00;
const CTRL_LRST: u32 = 1 << 3;
const CTRL_ASDE: u32 = 1 << 5;
const CTRL_SLU: u32 = 1 << 6;
const CTRL_ILOS: u32 = 1 << 7;
const CTRL_RST: u32 = 1 << 26;
const CTRL_VME: u32 = 1 << 30;
const CTRL_PHY_RST: u32 = 1 << 31;

const STATUS: u32 = 0x08;

const FCAL: u32 = 0x28;
const FCAH: u32 = 0x2C;
const FCT: u32 = 0x30;
const FCTTV: u32 = 0x170;

const ICR: u32 = 0xC0;

const IMS: u32 = 0xD0;
const IMS_TXDW: u32 = 1;
const IMS_TXQE: u32 = 1 << 1;
const IMS_LSC: u32 = 1 << 2;
const IMS_RXSEQ: u32 = 1 << 3;
const IMS_RXDMT: u32 = 1 << 4;
const IMS_RX: u32 = 1 << 6;
const IMS_RXT: u32 = 1 << 7;

const RCTL: u32 = 0x100;
const RCTL_EN: u32 = 1 << 1;
const RCTL_UPE: u32 = 1 << 3;
const RCTL_MPE: u32 = 1 << 4;
const RCTL_LPE: u32 = 1 << 5;
const RCTL_LBM: u32 = 1 << 6 | 1 << 7;
const RCTL_BAM: u32 = 1 << 15;
const RCTL_BSIZE1: u32 = 1 << 16;
const RCTL_BSIZE2: u32 = 1 << 17;
const RCTL_BSEX: u32 = 1 << 25;
const RCTL_SECRC: u32 = 1 << 26;

const RDBAL: u32 = 0x2800;
const RDBAH: u32 = 0x2804;
const RDLEN: u32 = 0x2808;
const RDH: u32 = 0x2810;
const RDT: u32 = 0x2818;

const RAL0: u32 = 0x5400;
const RAH0: u32 = 0x5404;

#[derive(Debug, Copy, Clone)]
#[repr(packed)]
struct Rd {
    buffer: u64,
    length: u16,
    checksum: u16,
    status: u8,
    error: u8,
    special: u16,
}
const RD_DD: u8 = 1;
const RD_EOP: u8 = 1 << 1;

const TCTL: u32 = 0x400;
const TCTL_EN: u32 = 1 << 1;
const TCTL_PSP: u32 = 1 << 3;

const TDBAL: u32 = 0x3800;
const TDBAH: u32 = 0x3804;
const TDLEN: u32 = 0x3808;
const TDH: u32 = 0x3810;
const TDT: u32 = 0x3818;

#[derive(Debug, Copy, Clone)]
#[repr(packed)]
struct Td {
    buffer: u64,
    length: u16,
    cso: u8,
    command: u8,
    status: u8,
    css: u8,
    special: u16,
}
const TD_CMD_EOP: u8 = 1;
const TD_CMD_IFCS: u8 = 1 << 1;
const TD_CMD_RS: u8 = 1 << 3;
const TD_DD: u8 = 1;

struct E1000Com {
    mmio_base: *mut u8,
    mmio_size: usize,
}

impl E1000Com {
    fn read_mac_address(&self) -> [u8; 6] {
        let mac_low = self.in32(RAL0);
        let mac_high = self.in32(RAH0);
        [
            mac_low as u8,
            (mac_low >> 8) as u8,
            (mac_low >> 16) as u8,
            (mac_low >> 24) as u8,
            mac_high as u8,
            (mac_high >> 8) as u8,
        ]
    }

    fn out32<T: Into<u64>>(&self, address: T, data: u32) {
        let address = address.into() as usize;
        assert!(address < self.mmio_size);
        unsafe { core::ptr::write_volatile(self.mmio_base.add(address) as *mut u32, data) };
    }

    fn in32<T: Into<u64>>(&self, address: T) -> u32 {
        let address = address.into() as usize;
        assert!(address < self.mmio_size);
        unsafe { core::ptr::read_volatile(self.mmio_base.add(address) as *mut u32) }
    }

    fn flag<T: Into<u64>>(&self, register: T, flag: u32, value: bool) {
        let register = register.into();
        if value {
            self.out32(register, self.in32(register) | flag);
        } else {
            self.out32(register, self.in32(register) & !flag);
        }
    }
}

pub struct E1000Net {
    com: E1000Com,
    receive_buffer: [Dma<[u8; 16384]>; 16],
    receive_ring: Dma<[Rd; 16]>,
    receive_index: usize,
    transmit_buffer: [Dma<[u8; 16384]>; 16],
    transmit_ring: Dma<[Td; 16]>,
    transmit_ring_free: usize,
    transmit_index: usize,
    transmit_clean_index: usize,
}

impl E1000Net {
    fn initialize(com: E1000Com) -> Self {
        #[rustfmt::skip]
        let receive_buffer: [Dma<[u8; 16384]>; 16] = [
            Dma::zeroed(), Dma::zeroed(), Dma::zeroed(), Dma::zeroed(),
            Dma::zeroed(), Dma::zeroed(), Dma::zeroed(), Dma::zeroed(),
            Dma::zeroed(), Dma::zeroed(), Dma::zeroed(), Dma::zeroed(),
            Dma::zeroed(), Dma::zeroed(), Dma::zeroed(), Dma::zeroed(),
        ];
        #[rustfmt::skip]
        let transmit_buffer: [Dma<[u8; 16384]>; 16] = [
            Dma::zeroed(), Dma::zeroed(), Dma::zeroed(), Dma::zeroed(),
            Dma::zeroed(), Dma::zeroed(), Dma::zeroed(), Dma::zeroed(),
            Dma::zeroed(), Dma::zeroed(), Dma::zeroed(), Dma::zeroed(),
            Dma::zeroed(), Dma::zeroed(), Dma::zeroed(), Dma::zeroed(),
        ];
        let receive_index = 0;
        let mut receive_ring: Dma<[Rd; 16]> = Dma::zeroed();
        let mut transmit_ring: Dma<[Td; 16]> = Dma::zeroed();
        let transmit_ring_free = 16;
        let transmit_index = 0;
        let transmit_clean_index = 0;

        // assert!(com.detect_eeprom());
        let mac_addr = com.read_mac_address();
        printk!("[e1000] Link address: {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}\n", mac_addr[0], mac_addr[1], mac_addr[2], mac_addr[3], mac_addr[4], mac_addr[5]);

        com.flag(CTRL, CTRL_RST, true);
        while com.in32(CTRL) & CTRL_RST == CTRL_RST {
            printk!("[e1000] Waiting for reset: {:X}\n", com.in32(CTRL));
        }

        // Enable auto negotiate, link, clear reset, do not Invert Loss-Of Signal
        com.flag(CTRL, CTRL_ASDE | CTRL_SLU, true);
        com.flag(CTRL, CTRL_LRST | CTRL_PHY_RST | CTRL_ILOS, false);

        // No flow control
        com.out32(FCAH, 0);
        com.out32(FCAL, 0);
        com.out32(FCT, 0);
        com.out32(FCTTV, 0);

        // Do not use VLANs
        com.flag(CTRL, CTRL_VME, false);

        for i in 0..receive_ring.len() {
            receive_ring[i].buffer = receive_buffer[i].physical();
        }

        com.out32(RDBAH, (receive_ring.physical() >> 32) as u32);
        com.out32(RDBAL, receive_ring.physical() as u32);
        com.out32(
            RDLEN,
            (receive_ring.len() * core::mem::size_of::<Rd>()) as u32,
        );
        com.out32(RDH, 0);
        com.out32(RDT, receive_ring.len() as u32 - 1);

        // Transmit Buffer
        for i in 0..transmit_ring.len() {
            transmit_ring[i].buffer = transmit_buffer[i].physical();
        }
        com.out32(TDBAH, (transmit_ring.physical() >> 32) as u32);
        com.out32(TDBAL, transmit_ring.physical() as u32);
        com.out32(
            TDLEN,
            (transmit_ring.len() * core::mem::size_of::<Td>()) as u32,
        );
        com.out32(TDH, 0);
        com.out32(TDT, 0);

        com.out32(IMS, IMS_RXT | IMS_RX | IMS_RXDMT | IMS_RXSEQ); // | IMS_LSC | IMS_TXQE | IMS_TXDW

        com.flag(RCTL, RCTL_EN, true);
        com.flag(RCTL, RCTL_UPE, true);
        // com.flag(RCTL, RCTL_MPE, true);
        com.flag(RCTL, RCTL_LPE, true);
        com.flag(RCTL, RCTL_LBM, false);
        // RCTL.RDMTS = Minimum threshold size ???
        // RCTL.MO = Multicast offset
        com.flag(RCTL, RCTL_BAM, true);
        com.flag(RCTL, RCTL_BSIZE1, true);
        com.flag(RCTL, RCTL_BSIZE2, false);
        com.flag(RCTL, RCTL_BSEX, true);
        com.flag(RCTL, RCTL_SECRC, true);

        com.flag(TCTL, TCTL_EN, true);
        com.flag(TCTL, TCTL_PSP, true);

        // TCTL.CT = Collision threshold
        // TCTL.COLD = Collision distance
        // TIPG Packet Gap
        // TODO ...

        while com.in32(STATUS) & 2 != 2 {
            printk!("[e1000] Waiting for link up: {:X}\n", com.in32(STATUS));
        }
        printk!(
            "[e1000] Link is up with speed {}\n",
            match (com.in32(STATUS) >> 6) & 0b11 {
                0b00 => "10 Mb/s",
                0b01 => "100 Mb/s",
                _ => "1000 Mb/s",
            }
        );

        E1000Net {
            com,
            receive_buffer,
            receive_ring,
            receive_index,
            transmit_buffer,
            transmit_ring,
            transmit_ring_free,
            transmit_index,
            transmit_clean_index,
        }
    }


    fn read(&mut self, buf: &mut [u8]) -> Option<usize> {
        let desc = unsafe { &mut *(self.receive_ring.as_ptr().add(self.receive_index) as *mut Rd) };

        if desc.status & RD_DD == RD_DD {
            desc.status = 0;

            let data = &self.receive_buffer[self.receive_index][..desc.length as usize];

            let i = core::cmp::min(buf.len(), data.len());
            buf[..i].copy_from_slice(&data[..i]);

            self.com.out32(RDT, self.receive_index as u32);
            self.receive_index = wrap_ring(self.receive_index, self.receive_ring.len());

            Some(i)
        } else {
            None
        }
    }

    fn write(&mut self, buf: &[u8]) -> Option<usize> {
        use core::cmp;

        if self.transmit_ring_free == 0 {
            loop {
                unsafe { asm!("nop" ::: "memory":"volatile")};
                let desc = unsafe {
                    &*(self.transmit_ring.as_ptr().add(self.transmit_clean_index) as *const Td)
                };

                if desc.status != 0 {
                    self.transmit_clean_index =
                        wrap_ring(self.transmit_clean_index, self.transmit_ring.len());
                    self.transmit_ring_free += 1;
                } else if self.transmit_ring_free > 0 {
                    break;
                }

                if self.transmit_ring_free >= self.transmit_ring.len() {
                    break;
                }
            }
        }

        let desc =
            unsafe { &mut *(self.transmit_ring.as_ptr().add(self.transmit_index) as *mut Td) };

        let data = unsafe {
            alloc::slice::from_raw_parts_mut(
                self.transmit_buffer[self.transmit_index].as_ptr() as *mut u8,
                cmp::min(buf.len(), self.transmit_buffer[self.transmit_index].len()) as usize,
            )
        };

        let i = cmp::min(buf.len(), data.len());
        data[..i].copy_from_slice(&buf[..i]);

        desc.cso = 0;
        desc.command = TD_CMD_EOP | TD_CMD_IFCS | TD_CMD_RS;
        desc.status = 0;
        desc.css = 0;
        desc.special = 0;

        desc.length = (cmp::min(
            buf.len(),
            self.transmit_buffer[self.transmit_index].len() - 1,
        )) as u16;

        self.transmit_index = wrap_ring(self.transmit_index, self.transmit_ring.len());
        self.transmit_ring_free -= 1;

        self.com.out32(TDT, self.transmit_index as u32);

        Some(i)
    }
}

pub fn setup_1000e(dev: &PciDeviceInfo) -> E1000Net {
    // a bunch of crude checks that work for the qemu e1000 implementation:
    assert!(dev.bars[0] != 0);
    assert!(dev.bars[1] != 0);
    assert_eq!(dev.bars[0] & 1, 0);
    assert_eq!(dev.bars[1] & 1, 1);
    let mem_space_type = dev.bars[0] & 0b110;
    assert_eq!(mem_space_type, 0); // => 32 bit
    let memory_space_bar = dev.bars[0] & (!0xff);
    // let io_space_bar = dev.bars[1] & (!0xf);
    // printk!("memory_space_bar = {:#x}\n", memory_space_bar);
    // printk!("io_space_bar = {:#x}\n", io_space_bar);

    let mmio = crate::memory::allocate_and_map_specific_phys_region(
        x86_64::PhysAddr::new(memory_space_bar as u64),
        dev.bar_sizes[0] as u64,
    );

    dev.enable_bus_mastering();
    let mut dev = E1000Net::initialize(E1000Com {
        mmio_base: mmio.start(),
        mmio_size: mmio.len() as usize,
    });

    let mut buf = alloc::vec::Vec::new();
    for _ in 0..8096 {
        buf.push(0u8);
    }
    loop {
        // printk!("dev.transmit_ring_free: {}\n", dev.transmit_ring_free);
        let res = dev.read(&mut buf);
        if let Some(x) = res {
            printk!("dev.read: {:#x?}\n", &buf[..x]);
        }
        let mut packet = alloc::vec::Vec::new();
        packet.extend_from_slice(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
        packet.extend_from_slice(&dev.com.read_mac_address());
        packet.extend_from_slice(&[0x08, 0x00]); // Type: IPv4
        packet.extend_from_slice(&[0x45, 0x00, 0x00, 0x24, 0x23, 0x47, 0x40, 0x00, 0x40, 0x11, 0xa4, 0xb2, 0xc0, 0xa8, 0xb2, 0x27, 0xff, 0xff, 0xff, 0xff]); // IP Header
        packet.extend_from_slice(&[0xe9, 0xd2, 0x5d, 0xc0, 0x00, 0x10, 0x72, 0xf1]); // UDP Header
        packet.extend_from_slice(b"TESTING"); // Payload

        let res = dev.write(&packet);
        // printk!("dev.write: {:?}\n", res);
    }

    dev
}
