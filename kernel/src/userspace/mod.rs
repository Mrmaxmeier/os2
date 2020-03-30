pub mod user;

/// The head of the current stack
static mut CURRENT_STACK_HEAD: u64 = 0;

pub use user::{start_user_task};

pub enum UserspaceExit {
    Segfault,
    Exception,
    Syscall
}

#[derive(Debug, Default)]
#[repr(C)]
pub struct SavedRegs {
    pub rax: u64,
    pub rbx: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rdi: u64,
    pub rsi: u64,
    pub rbp: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,

    pub rflags: u64,
    pub rip: u64,

    pub rsp: u64,
}
