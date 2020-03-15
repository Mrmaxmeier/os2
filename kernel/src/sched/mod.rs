//! The scheduler

pub mod user;

use alloc::{boxed::Box, collections::linked_list::LinkedList, vec, vec::Vec};

use core::{borrow::Borrow, mem};

/// The size of a stack in words
const STACK_WORDS: usize = 1 << 12; // 16KB

/// The head of the current stack
// TODO: maybe there is some race condition here? not really sure... I think the scheduler and the
// syscall handler are the only ones using this, and by construction at most one of them can be
// running at a time...
static mut CURRENT_STACK_HEAD: u64 = 0;

/// An stack for execution of continuations
struct Stack(Box<[usize; STACK_WORDS]>);

impl Stack {
    /// Returns a new clean stack
    pub fn new() -> Self {
        Stack(box [0; STACK_WORDS]) // initialize in place
    }

    /// Returns the stack pointer to use for this stack
    pub fn first_rsp(&self) -> usize {
        /// Add a little padding in case a bug causes us to unwind too far.
        const PADDING: usize = 400; // words

        // The end of the array is the "bottom" (highest address) in the stack.
        let stack: &[usize; STACK_WORDS] = self.0.borrow();
        let bottom = stack.as_ptr();
        unsafe { bottom.add(STACK_WORDS - PADDING) as usize }
    }

    /// Clear the contents of this stack
    pub fn clear(&mut self) {
        for word in self.0.iter_mut() {
            *word = 0xDEADBEEF_DEADBEEF;
        }
    }
}

pub fn init() {

}

/// Run the scheduler to choose a task. Then switch to that task, discarding the current task as
/// complete. This should be called after all clean up has been completed. If no next task exists,
/// the idle continuation is used.
pub fn sched() -> ! {
    todo!();

    // switch to clean stack.
    /*
    let rsp = to.first_rsp();

    unsafe {
        CURRENT_STACK_HEAD = rsp as u64;
    }

    unsafe {
        sched_part_2_thunk(rsp);
    }
    */
}

/// Part 2 of `sched`. This actually switches to the new stack. Then, it calls `part_3`, having
/// already switched to the new stack. This is done so that the compiler knows that no state should
/// be carried over, so we cannot lose any important stack variables (e.g. locks).
unsafe fn sched_part_2_thunk(rsp: usize) -> ! {
    asm! {
        "
        movq $0, %rsp
        movq $0, %rbp
        "
         : /* no outputs */
         : "r"(rsp)
         : "rbp", "rsp"
         : "volatile"
    };
    todo!();
}
