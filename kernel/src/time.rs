//! A module for dealing with system time and the passage of time.

use core::sync::atomic::{AtomicUsize, Ordering};

use crate::interrupts::PIT_HZ;

/// Counts interrupts. This can be used as a source of time.
static TICKS: AtomicUsize = AtomicUsize::new(0);

/// Opaquely represents a system time
#[derive(Copy, Clone, Eq, Ord, PartialEq, PartialOrd)]
pub struct SysTime(usize);

impl SysTime {
    /// Get the system time without synchronizing. This has better performance but potentially misses a
    /// tick every once in a while.
    pub fn now() -> Self {
        // safe because we are only reading and we don't mind missing some synchronous op
        let time = unsafe {
            // we are guaranteed by the standard library that `AtomicUsize` has the same memory layout
            // as `usize`.
            *(&TICKS as *const AtomicUsize as *const usize)
        };

        SysTime(time)
    }

    /// Get the time `millis` millis after `self`.
    pub fn after_ms(self, millis: usize) -> Self {
        SysTime(self.0 + millis * PIT_HZ / 1000)
    }
}

/// Tick the clock atomically.
///
/// # NOTE
///
/// This should only be called from the timer interrupt handler.
pub fn tick() {
    TICKS.fetch_add(1, Ordering::Relaxed);
}
