#![feature(
    lang_items,
    asm,
    alloc_error_handler,
    box_syntax,
    abi_x86_interrupt,
    panic_info_message,
    drain_filter,
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
mod sched;
mod snapshot;
mod time;

use bootloader::BootInfo;
use memory::{map_region, VirtualMemoryRegion};

use x86_64::structures::paging::PageTableFlags;

pub const PAGING_DEBUG: bool = true;

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
    printk!("[DONE] Memory    \n");

    // Set up interrupt/exception handling
    printk!("[    ] Interrupts ...\r");
    interrupts::init();
    sched::user::init();
    printk!("[DONE] Interrupts    \n");

    // We can turn on interrupts now.
    x86_64::instructions::interrupts::enable();

    printk!("[    ] Time ...\r");
    let start = time::SysTime::now();
    for target in 0..50 {
        while time::SysTime::now() < start.after_ms(target * 20) {
            x86_64::instructions::hlt();
        }
        printk!("[{:04}] Time ...\r", (target + 1) * 20);
    }
    printk!("[DONE] Time    \n");

    let code = unsafe { VirtualMemoryRegion::take_range(0x13370000, 0x13370fff) };
    {
        map_region(
            code.clone(),
            PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE | PageTableFlags::WRITABLE,
        );
        let data = [0x90, 0x90, 0x0f, 0x05, 0xcc];
        for i in 0..data.len() {
            unsafe { code.start().add(i).write_volatile(data[i]) };
        }
    }
    let stack = sched::user::allocate_user_stack();
    printk!("[    ] Userspace ...\r");
    sched::user::start_user_task(code.start() as usize, stack);
    // printk!("[DONE] Userspace    \n");
    // todo!();
}
