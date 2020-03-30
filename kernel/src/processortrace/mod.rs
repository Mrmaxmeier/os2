use x86_64::registers::model_specific::Msr;


const MSR_IA32_VMX_MISC: Msr = Msr::new(0x00000485);
const MSR_IA32_VMX_MISC_INTEL_PT: u64 = 1 << 14;
const MSR_IA32_VMX_PROCBASED_CTLS2: Msr = Msr::new(0x0000048b);
const VM_EXIT_CLEAR_IA32_RTIT_CTL: u64 = 0x02000000;
const VM_ENTRY_LOAD_IA32_RTIT_CTL: u64 = 0x00040000;
const VMX_FEATURE_PT_USE_GPA: u64 = ( 2*32+ 24) & 0x1f; /* "" Processor Trace logs GPAs */

pub fn test() {
    let x = unsafe { MSR_IA32_VMX_PROCBASED_CTLS2.read() };
    printk!("MSR_IA32_VMX_PROCBASED_CTLS2: {:#x}\n", x);
    let vmx_misc = unsafe { MSR_IA32_VMX_MISC.read() };
    assert!(vmx_misc & MSR_IA32_VMX_MISC_INTEL_PT != 0);
    printk!("[I-PT] Looks good!\n");
}