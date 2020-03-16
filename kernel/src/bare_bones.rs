//! This module contains some basic functionality that libstd would normally
//! otherwise provide. Most importantly, it defines `rust_begin_unwind` which is
//! used by `panic!`.

/// This function is used by `panic!` to display an error message.
#[cfg(not(test))] // rust-analyzer is confused by a 'duplicate definition' in std via test
#[panic_handler]
fn rust_begin_panic(pi: &core::panic::PanicInfo) -> ! {
    use crate::debug::Debug;
    use core::fmt::Write;
    use x86_64::instructions::{hlt, interrupts};

    // we should no be interrupting any more
    interrupts::disable();

    printk!("\n========={{ PANIC }}=========\n");

    // Print location if its there
    if let Some(loc) = pi.location() {
        printk!("{}:{}:{}\n", loc.file(), loc.line(), loc.column());
    } else {
        printk!("<no location info>\n");
    }

    printk!("...........................\n");

    // Print the message
    if let Some(msg) = pi.message() {
        let _ = Debug.write_fmt(*msg);
    } else {
        printk!("<no message>");
    }

    printk!("\n===========================\n");

    loop {
        hlt(); // Don't just spin... wait a bit
    }
}
