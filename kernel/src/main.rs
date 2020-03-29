#![feature(
    lang_items,
    asm,
    alloc_error_handler,
    box_syntax,
    abi_x86_interrupt,
    panic_info_message,
    naked_functions
)]
// Compile without libstd
#![no_std]
#![no_main]
#![crate_type = "staticlib"]
#![crate_name = "kernel"]

extern crate alloc;

#[macro_use]
mod debug;
mod bare_bones;
mod interrupts;
mod memory;
mod net;
mod sched;
mod snapshot;
mod time;

use bootloader::BootInfo;

pub const PAGING_DEBUG: bool = false;

/// The kernel heap
#[global_allocator]
static mut ALLOCATOR: memory::KernelAllocator = memory::KernelAllocator::new();

bootloader::entry_point!(kernel_main);

/// This is the entry point to the kernel. It is the first rust code that runs.
#[no_mangle]
fn kernel_main(boot_info: &'static BootInfo) -> ! {
    // At this point we are still in the provisional environment with
    // - the temporary page tables (first 2MiB of memory direct mapped)
    // - no IDT
    // - no current task

    // Make sure interrupts are off
    x86_64::instructions::interrupts::disable();

    printk!("   ~=> kernel_main\n");

    // Initialize memory
    // make the kernel heap 1MiB - 4KiB starting at 1MiB + 4KiB. This extra page will be unmapped
    // later to protect against heap overflows (unlikely as that is)...
    if PAGING_DEBUG {
        printk!("[ .. ] Memory\n");
    } else {
        printk!("[    ] Memory ...\r");
    }
    memory::init(unsafe { &mut ALLOCATOR }, boot_info);
    // remove stack range from allocatable memory
    unsafe { memory::VirtualMemoryRegion::take_range(0x100_0000_0000, 0x100_003f_ffff) };
    printk!("[DONE] Memory    \n");

    // Set up interrupt/exception handling
    printk!("[    ] Interrupts ...\r");
    interrupts::init();
    sched::user::init();
    printk!("[DONE] Interrupts    \n");


    // We can turn on interrupts now.
    x86_64::instructions::interrupts::enable();

    if false {
        printk!("[    ] Time ...\r");
        let start = time::SysTime::now();
        for target in 0..50 {
            while time::SysTime::now() < start.after_ms(target * 20) {
                x86_64::instructions::hlt();
            }
            printk!("[{:04}] Time ...\r", (target + 1) * 20);
        }
        printk!("[DONE] Time    \n");
    }

    let mut netdev = None;
    for dev in tinypci::brute_force_scan() {
        /*
        printk!(
            "PCI: {:x}:{:x}: {:?}\n",
            dev.vendor_id,
            dev.device_id,
            dev.full_class
        );
        */
        if dev.vendor_id == 0x8086 && dev.device_id == 0x100e {
            // printk!("{}\n", dev);
            netdev = Some(net::setup_1000e(&dev));
        }
    }

    /*
    log::set_logger(&crate::debug::Logger).unwrap();
    log::set_max_level(log::LevelFilter::Trace);
    */

    printk!("[DONE] Enumerating PCI devices\n");
    printk!("[    ] Network ...\r");
    let mut network = net::init(netdev.unwrap());
    printk!("[IDLE] Waiting for TCP connection on :1234 ...\r");
    network.wait_for_connection()
        .expect("network error");
    printk!("[DONE] Got connection on :1234                \n");

    printk!("[****] Starting Session ...\n");
    let mut session = snapshot::Session::new(network);
    session.recv_snapshot();
    session.setup_pages();
    session.run();
}
