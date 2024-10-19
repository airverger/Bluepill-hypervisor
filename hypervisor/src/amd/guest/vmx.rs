use alloc::boxed::Box;
use core::arch::{asm, global_asm};
use core::ptr::addr_of;
use x86::bits64::paging::BASE_PAGE_SHIFT;
use x86::controlregs::{cr0, cr3, cr4};
use x86::dtables::DescriptorTablePointer;
use x86::msr::{rdmsr, wrmsr};
use x86::segmentation;

use kernelutils::{physical_address, PhysicalAllocator, Registers};
use crate::amd::guest;

#[derive(derive_deref::Deref, derive_deref::DerefMut)]
pub struct Vmcb {
    data: Box<guest::VmcbRaw, PhysicalAllocator>,

}
#[derive(derive_deref::Deref, derive_deref::DerefMut)]
struct HostStateArea {
    data: Box<guest::HostStateAreaRaw, PhysicalAllocator>,
}

impl HostStateArea {
    pub fn new() -> Self {
        let dada = unsafe { Box::new_zeroed_in(PhysicalAllocator).assume_init() };
        Self {
            data: dada
        }
    }
    pub fn pa(&self) -> u64 {
        physical_address(addr_of!(*self.data.as_ref()) as _).as_u64()
    }
}

/// Returns the access rights of the given segment for SVM.
pub fn get_segment_access_right(table_base: u64, selector: u16) -> u16 {
    let descriptor_value = get_segment_descriptor_value(table_base, selector);

    // First, get the AVL, L, D/B and G bits, while excluding the "Seg. Limit 19:16"
    // bits. Then, get the Type, S, DPL and P bits. Finally, return those bits
    // without the "Seg. Limit 19:16" bits.
    // See: Figure 3-8. Segment Descriptor
    let ar = (descriptor_value >> 40) as u16;
    let upper_ar = (ar >> 4) & 0b1111_0000_0000;
    let lower_ar = ar & 0b1111_1111;
    lower_ar | upper_ar
}

/// Returns the segment descriptor casted as a 64bit integer for the given
/// selector.
pub fn get_segment_descriptor_value(table_base: u64, selector: u16) -> u64 {
    let sel = x86::segmentation::SegmentSelector::from_raw(selector);
    let descriptor_addr = table_base + u64::from(sel.index() * 8);
    let ptr = descriptor_addr as *const u64;
    unsafe { *ptr }
}

/// Returns the limit of the given segment.
pub fn get_segment_limit(table_base: u64, selector: u16) -> u32 {
    let sel = x86::segmentation::SegmentSelector::from_raw(selector);
    if sel.index() == 0 && (sel.bits() >> 2) == 0 {
        return 0; // unusable
    }
    let descriptor_value = get_segment_descriptor_value(table_base, selector);
    let limit_low = descriptor_value & 0xffff;
    let limit_high = (descriptor_value >> (32 + 16)) & 0xF;
    let mut limit = limit_low | (limit_high << 16);
    if ((descriptor_value >> (32 + 23)) & 0x01) != 0 {
        limit = ((limit + 1) << BASE_PAGE_SHIFT) - 1;
    }
    limit as u32
}


impl Vmcb {
    pub fn new() -> Self {
        let dada = unsafe { Box::new_zeroed_in(PhysicalAllocator).assume_init() };
        Self {
            data: dada
        }
    }
    pub fn pa(&self) -> u64 {
        physical_address(addr_of!(*self.data.as_ref()) as _).as_u64()
    }


    pub fn control_intercept(&mut self, ctl1_code: u32, ctl2_code: u32) {
        self.control_area.intercept_misc1 = ctl1_code;
        self.control_area.intercept_misc2 = ctl2_code;
        self.control_area.pause_filter_count = u16::MAX;
    }

    pub fn control_asid(&mut self) {
        self.control_area.guest_asid = 1;
    }

    pub fn control_pml4(&mut self, pml4_pa: u64) {
        const SVM_NP_ENABLE_NP_ENABLE: u64 = 1 << 0;
        self.control_area.np_enable = SVM_NP_ENABLE_NP_ENABLE;
        self.control_area.ncr3 = pml4_pa;
    }
    pub fn control_msr_init(&mut self) {
        const SVM_MSR_VM_CR: u32 = 0xc001_0114;
        const R_INIT: u64 = 1 << 1;
        unsafe { wrmsr(SVM_MSR_VM_CR, rdmsr(SVM_MSR_VM_CR) | R_INIT); };
    }


    pub fn control_exception(&mut self, except_code: u32) {
        self.control_area.intercept_exception = except_code;
    }
    pub fn control_basic(&mut self) {
        const SVM_INTERCEPT_MISC1_CPUID: u32 = 1 << 18;
        const SVM_INTERCEPT_MISC2_VMRUN: u32 = 1 << 0;
        self.control_intercept(SVM_INTERCEPT_MISC1_CPUID, SVM_INTERCEPT_MISC2_VMRUN);
        self.control_asid();
        self.control_pml4(0);
        self.control_msr_init();
        const SECURITY_EXCEPTION: u32 = 1 << 30;
        self.control_exception(SECURITY_EXCEPTION);
    }

    pub fn state_selector(&mut self) {
        self.state_save_area.es_selector = segmentation::es().bits();
        self.state_save_area.cs_selector = segmentation::cs().bits();
        self.state_save_area.ss_selector = segmentation::ss().bits();
        self.state_save_area.ds_selector = segmentation::ds().bits();
    }
    pub fn state_attrib(&mut self) {
        let gdtr = sgdt();
        let guest_gdt = gdtr.base as u64;
        self.state_save_area.es_attrib = get_segment_access_right(guest_gdt, segmentation::es().bits());
        self.state_save_area.cs_attrib = get_segment_access_right(guest_gdt, segmentation::cs().bits());
        self.state_save_area.ss_attrib = get_segment_access_right(guest_gdt, segmentation::ss().bits());
        self.state_save_area.ds_attrib = get_segment_access_right(guest_gdt, segmentation::ds().bits());
    }

    pub fn state_limit(&mut self) {
        let gdtr = sgdt();
        let guest_gdt = gdtr.base as u64;
        self.state_save_area.es_limit = get_segment_limit(guest_gdt, segmentation::es().bits());
        self.state_save_area.cs_limit = get_segment_limit(guest_gdt, segmentation::cs().bits());
        self.state_save_area.ss_limit = get_segment_limit(guest_gdt, segmentation::ss().bits());
        self.state_save_area.ds_limit = get_segment_limit(guest_gdt, segmentation::ds().bits());
    }

    pub fn state_tr_table(&mut self) {
        let idtr = sidt();
        let gdtr = sgdt();
        // let guest_gdt = gdtr.base as u64;
        self.state_save_area.gdtr_base = gdtr.base as _;
        self.state_save_area.gdtr_limit = u32::from(gdtr.limit);
        self.state_save_area.idtr_base = idtr.base as _;
        self.state_save_area.idtr_limit = u32::from(idtr.limit);
    }
    pub fn state_msr(&mut self) {
        const EFER_SVME: u64 = 1 << 12;
        unsafe { self.state_save_area.efer = rdmsr(x86::msr::IA32_EFER) | EFER_SVME; }
        unsafe { self.state_save_area.gpat = rdmsr(x86::msr::IA32_PAT); }
    }
    pub fn state_cr(&mut self) {
        unsafe { self.state_save_area.cr0 = cr0().bits() as _; }
        unsafe { self.state_save_area.cr3 = cr3(); }
        unsafe { self.state_save_area.cr4 = cr4().bits() as _; }
    }
    pub fn state_register(&mut self, registers: &Registers) {
        self.state_save_area.rip = registers.rip;
        self.state_save_area.rsp = registers.rsp;
        self.state_save_area.rflags = registers.rflags;
        self.state_save_area.rax = registers.rax;
    }
}
pub(crate) fn vmsave(vmcb_pa: u64) {
    unsafe {
        asm!(
        "mov rax, {}",
        "vmsave rax",
        in(reg) vmcb_pa, options(nostack, preserves_flags),
        )
    };
}
pub(crate) fn sidt() -> DescriptorTablePointer<u64> {
    let mut idtr = DescriptorTablePointer::<u64>::default();
    unsafe { x86::dtables::sidt(&mut idtr) };
    idtr
}
#[allow(dead_code)]
pub(crate) fn sgdt() -> DescriptorTablePointer<u64> {
    let mut gdtr = DescriptorTablePointer::<u64>::default();
    unsafe { x86::dtables::sgdt(&mut gdtr) };
    gdtr
}
pub struct Vmx {
    #[allow(dead_code)]
    id: usize,
    guest_vmcb: Vmcb,

    #[allow(dead_code)]
    host_vmcb: Vmcb,
    guest_vmcb_pa: u64,
    host_vmcb_pa: u64,
    // #[derivative(Debug = "ignore")]
    host_state: HostStateArea,
    registers: Registers,

    // activity_state: &'static AtomicU8,

}
extern "C" {
    /// Runs the guest until #VMEXIT occurs.
   pub fn run_svm_guest(registers: &mut Registers, vmcb_pa: u64, host_vmcb_pa: u64);
}

global_asm!(include_str!("run_guest.S"));
impl Vmx {
    #[allow(dead_code)]
    pub(crate) fn run(&mut self) {
        const VMEXIT_EXCEPTION_SX: u64 = 0x5e;
        const VMEXIT_CPUID: u64 = 0x72;
        const VMEXIT_NPF: u64 = 0x400;
        self.guest_vmcb.state_save_area.rax = self.registers.rax;
        self.guest_vmcb.state_save_area.rip = self.registers.rip;
        self.guest_vmcb.state_save_area.rsp = self.registers.rsp;
        self.guest_vmcb.state_save_area.rflags = self.registers.rflags;
        log::trace!("Entering the guest");
        unsafe { run_svm_guest(&mut self.registers, self.guest_vmcb_pa, self.host_vmcb_pa) };
        log::trace!("Exited the guest");
        unsafe { asm!("int 3") }

    }

}
impl Vmx {
    #[allow(dead_code)]
    pub fn save_guest(&self) {
        vmsave(self.guest_vmcb_pa);
    }
    #[allow(dead_code)]
    pub fn save_host(&self) {
        vmsave(self.host_vmcb_pa);
    }
    #[allow(dead_code)]
    pub fn enable() {
        const EFER_SVME: u64 = 1 << 12;

        // Enable SVM. We assume the processor is compatible with this.
        // See: 15.4 Enabling SVM
        unsafe { wrmsr(x86::msr::IA32_EFER, rdmsr(x86::msr::IA32_EFER) | EFER_SVME); }
    }
    #[allow(dead_code)]
    pub fn new(registers: &Registers) -> Self {
        let mut guest_vmcb = Vmcb::new();
        guest_vmcb.control_basic();
        const SVM_INTERCEPT_MISC1_CPUID: u32 = 1 << 18;
        const SVM_INTERCEPT_MISC2_VMRUN: u32 = 1 << 0;
        guest_vmcb.control_intercept(SVM_INTERCEPT_MISC1_CPUID, SVM_INTERCEPT_MISC2_VMRUN);
        const SECURITY_EXCEPTION: u32 = 1 << 30;
        guest_vmcb.control_exception(SECURITY_EXCEPTION);
        guest_vmcb.state_selector();
        guest_vmcb.state_attrib();
        guest_vmcb.state_limit();
        guest_vmcb.state_tr_table();
        guest_vmcb.state_msr();
        guest_vmcb.state_cr();
        guest_vmcb.state_register(registers);
        let host_vmcb = Vmcb::new();


        let guest_vmcb_pa = guest_vmcb.pa();
        let host_vmcb_pa = host_vmcb.pa();
        let host_state = HostStateArea::new();
        Self {
            id: 0,
            guest_vmcb,
            host_vmcb,
            guest_vmcb_pa,
            host_vmcb_pa,
            host_state,
            registers: *registers,
        }
    }

    #[allow(dead_code)]
  pub  fn activate(&mut self) {
        const SVM_MSR_VM_HSAVE_PA: u32 = 0xc001_0117;
        let pa = self.host_state.pa();
        unsafe { wrmsr(SVM_MSR_VM_HSAVE_PA, pa); }
    }
    #[allow(dead_code)]
    fn regs(&mut self) -> &mut Registers {
        &mut self.registers
    }
}