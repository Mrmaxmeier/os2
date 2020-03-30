//! System calls and kernel <-> user mode switching...

use x86_64::{
    registers::{
        model_specific::{Efer, EferFlags, Star, LStar, SFMask},
        rflags::{self, RFlags},
    },
    VirtAddr,
};

use crate::{
    interrupts::SELECTORS,
    userspace::SavedRegs,
};


/// Set some MSRs, registers to enable syscalls and user/kernel context switching.
pub fn init() {
    unsafe {
        // Need to set IA32_EFER.SCE
        Efer::update(|flags| *flags |= EferFlags::SYSTEM_CALL_EXTENSIONS);

        // STAR: Ring 0 and Ring 3 segments
        // - Kernel mode CS is bits 47:32
        // - Kernel mode SS is bits 47:32 + 8
        // - User mode CS is bits 63:48 + 16
        // - User mode SS is bits 63:48 + 8
        //
        // Each entry in the GDT is 8B...
        let selectors = SELECTORS.lock();
        let kernel_base: u16 = selectors.kernel_cs.index() * 8;
        let user_base: u16 = (selectors.user_ss.index() - 1) * 8;
        Star::write_raw(kernel_base, user_base);
        /*
        Star::write(
            selectors.kernel_cs, 
            selectors.kernel_ss, 
            selectors.user_cs, 
            selectors.user_ss
        ).unwrap();
        */

        // LSTAR: Syscall Entry RIP
        LStar::write(VirtAddr::new(syscall::entry as u64));

        // FMASK: rflags mask: any set bits are cleared on syscall
        //
        // Want to disable interrupt until we switch to the kernel stack.
        SFMask::write(RFlags::INTERRUPT_FLAG);
    }
}

pub fn start_user_task(regs: SavedRegs) -> ! {
    // Compute new register values
    /*
    let rsp = {
        let start = stack.start();
        let len = stack.len();
        unsafe { start.offset(len as isize) }
    };
    */

    // Enable interrupts for user mode.
    let rflags = (rflags::read() | rflags::RFlags::INTERRUPT_FLAG).bits();

    let registers = SavedRegs {
        rflags,
        ..regs
    };

    syscall::switch_to_user(&registers)
}

mod syscall {
    //! System call handling.

    use super::SavedRegs;

    /// Handle a `syscall` instruction from userspace.
    ///
    /// This is not to be called from kernel mode! And it should never be called more than once at a
    /// time.
    ///
    /// Interrupts are disabled on entry.
    ///
    /// Contract with userspace (beyond what the ISA does):
    /// - System call argument is passed in %rax
    /// - We may clobber %rdx
    /// - We will save and restore all other registers, including the stack pointer
    /// - We will return values in %rax
    #[naked]
    pub(super) unsafe extern "C" fn entry() {
        // Switch to tmp stack, save user regs
        asm!(
            "
            # save the user stack pointer to %rdx before we switch stacks.
            mov %rsp, %rdx

            # switch to the tmp stack
            mov $0, %rsp
            mov (%rsp), %rsp

            # start saving stuff
            pushq %rdx # user rsp
            pushq %rcx # user rip
            pushq %r11 # user rflags

            pushq %r15
            pushq %r14
            pushq %r13
            pushq %r12
            pushq %r11
            pushq %r10
            pushq %r9
            pushq %r8
            pushq %rbp
            pushq %rsi
            pushq %rdi
            pushq %rdx
            pushq %rcx
            pushq %rbx
            pushq %rax

            # handle the system call. The saved registers are passed at the top of the stack where
            # we just pushed them.
            mov %rsp, %rdi
            call handle_syscall
            "
            : /* no outputs */
            : "i"(&super::super::CURRENT_STACK_HEAD)
            : "memory", "rax", "rbx", "rcx", "rdx", "rdi", "rsi", "r8", "r9", "r10", "r11", "r12",
              "r13", "r14", "r15", "rbp", "stack"
            : "volatile"
        );

        unreachable!();
    }

    /// Does the actual work of handling a syscall. Should only be called by `syscall_entry`. This
    /// assumes we are still running on the tmp stack. It switches to the saved kernel stack.
    #[no_mangle]
    unsafe extern "C" fn handle_syscall(saved_regs: &mut SavedRegs) {
        // TODO: can probably enable interrupts here...

        // Handle the system call. The syscall number is passed in %rax.
        match saved_regs.rax {
            n => printk!("syscall #{:#x?}\n", n),
        }

        // Return to usermode
        switch_to_user(saved_regs)
    }

    /// Switch to user mode with the given registers.
    pub(super) fn switch_to_user(registers: &SavedRegs) -> ! {
        // https://software.intel.com/sites/default/files/managed/39/c5/325462-sdm-vol-1-2abcd-3abcd.pdf#G43.25974
        //
        // Set the following and execute the `sysret` instruction:
        // - user rip: load into rcx before sysret
        // - rflags: load into r11 before sysret
        // - also want to set any register values to be given to the user
        //      - user rsp
        //      - clear all other regs
        unsafe {
            asm!(
                "
                # save kernel stack
                mov $16, %rcx
                mov %rsp, (%rcx)

                # restore registers
                movq $0, %rax
                movq $1, %rbx
                movq $2, %rdx
                movq $3, %rdi
                movq $4, %rsi
                movq $5, %rbp
                movq $6, %r8
                movq $7, %r9
                movq $8, %r10
                movq $9, %r12
                movq $10, %r13
                movq $11, %r14
                movq $12, %r15

                # user rflags
                movq $13, %r11

                # user rip
                movq $14, %rcx

                # disable interrupts before loading the user stack; otherwise, an interrupt may be
                # serviced on the wrong stack.
                cli

                # no more stack refs until sysret
                movq $15, %rsp

                # return to usermode (ring 3)
                sysretq
                "
                : /* no outputs */
                : "m"(registers.rax)
                , "m"(registers.rbx)
                , "m"(registers.rdx)
                , "m"(registers.rdi)
                , "m"(registers.rsi)
                , "m"(registers.rbp)
                , "m"(registers.r8)
                , "m"(registers.r9)
                , "m"(registers.r10)
                , "m"(registers.r12)
                , "m"(registers.r13)
                , "m"(registers.r14)
                , "m"(registers.r15)
                , "m"(registers.rflags)
                , "m"(registers.rip)
                , "m"(registers.rsp)
                , "i"(&super::super::CURRENT_STACK_HEAD)
                : "memory", "rax", "rbx", "rcx", "rdx", "rdi", "rsi", "r8", "r9", "r10", "r11", "r12",
                  "r13", "r14", "r15", "rbp"
                : "volatile"
            );
        }

        unreachable!();
    }
}
