use crate::amd::Vmx;
use core::arch::asm;
use x86::cpuid::cpuid;

pub enum VmExitReason {
    Cpuid(InstructionInfo),
    Rdmsr(InstructionInfo),
    Wrmsr(InstructionInfo),
    XSetBv(InstructionInfo),
    InitSignal,
    StartupIpi,
    NestedPageFault,
    Unknown,
}

pub struct InstructionInfo {
    /// The next RIP of the guest in case the current instruction is emulated.
    pub(crate) next_rip: u64,
}

pub fn handle_cpuid(guest: &mut Vmx, info: &InstructionInfo) {
    let leaf = guest.regs().rax as u32;
    let sub_leaf = guest.regs().rcx as u32;
    log::trace!("CPUID {leaf:#x?} {sub_leaf:#x?}");
    let mut cpuid_result = cpuid!(leaf, sub_leaf);
    if leaf == 1 {
        cpuid_result.ecx &= !(1 << 5);
    }
    guest.regs().rax = u64::from(cpuid_result.eax);
    guest.regs().rbx = u64::from(cpuid_result.ebx);
    guest.regs().rcx = u64::from(cpuid_result.ecx);
    guest.regs().rdx = u64::from(cpuid_result.edx);
    guest.regs().rip = info.next_rip;

    unsafe { asm!("int 3") }
}
