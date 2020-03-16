//! The scheduler

pub mod user;

/// The head of the current stack
// TODO: maybe there is some race condition here? not really sure... I think the scheduler and the
// syscall handler are the only ones using this, and by construction at most one of them can be
// running at a time...
static mut CURRENT_STACK_HEAD: u64 = 0;
