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
#[allow(dead_code)]
const SVM_DEBUG_CONTROL_0: u16 = 1 << 0;
#[allow(dead_code)]
const SVM_DEBUG_CONTROL_6: u16 = 1 << 6;
#[allow(dead_code)]

const SVM_DEBUG_CONTROL_7: u16 = 1 << 7;
#[allow(dead_code)]
#[repr(u32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum TlbControl {
    DoNotFlush = 0x0,
    FlushAll = 0x1,
    FlushGuests = 0x3,
    FlushGuestsNonGlobal = 0x7,
}
extern "C" {
    /// Runs the guest until #VMEXIT occurs.
    pub fn run_svm_guest(registers: &mut Registers, vmcb_pa: u64, host_vmcb_pa: u64);
}

global_asm!(include_str!("run_guest.s"));
pub fn vmsave(vmcb_pa: u64) {
    unsafe {
        asm!(
        "mov rax, {}",
        "vmsave rax",
        in(reg) vmcb_pa, options(nostack, preserves_flags),
        )
    };
}
pub fn sidt() -> DescriptorTablePointer<u64> {
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

#[derive(derive_deref::Deref, derive_deref::DerefMut)]
pub struct Vmcb {
    data: Box<guest::VmcbRaw, PhysicalAllocator>,
}


impl Vmcb {
    pub fn new() -> Self {
        let dada = unsafe { Box::new_zeroed_in(PhysicalAllocator).assume_init() };
        Self { data: dada }
    }
    pub fn pa(&self) -> u64 {
        physical_address(addr_of!(*self.data.as_ref()) as _).as_u64()
    }
}
#[allow(dead_code)]
pub const SVM_NP_ENABLE_NP_ENABLE: u64 = 1 << 0;
#[allow(dead_code)]
pub const SECURITY_EXCEPTION: u32 = 1 << 30;
impl Vmcb {
    pub fn initialize_npt(&mut self, nested_pml4_addr: u64) {
        // let nested_pml4_addr = SHARED_GUEST_DATA.npt.read().as_ref() as *const _;
        self.control_area.np_enable = SVM_NP_ENABLE_NP_ENABLE;
        self.control_area.ncr3 = nested_pml4_addr;
    }
    pub fn initialize_excepiton(&mut self, code: u32) {
        self.control_area.intercept_exception = code;
    }

    pub fn initialize_control(&mut self) {
        const SVM_INTERCEPT_MISC1_CPUID: u32 = 1 << 18;
        const SVM_INTERCEPT_MISC2_VMRUN: u32 = 1 << 0;

        self.control_area.intercept_misc1 = SVM_INTERCEPT_MISC1_CPUID;
        self.control_area.intercept_misc2 = SVM_INTERCEPT_MISC2_VMRUN;
        self.control_area.pause_filter_count = u16::MAX;
        self.control_area.guest_asid = 1;

        const SVM_MSR_VM_CR: u32 = 0xc001_0114;
        const R_INIT: u64 = 1 << 1;
        unsafe { wrmsr(SVM_MSR_VM_CR, rdmsr(SVM_MSR_VM_CR) | R_INIT); }
    }

    pub fn initialize_guest(&mut self, registers: &Registers) {
        const EFER_SVME: u64 = 1 << 12;

        let idtr = sidt();
        let gdtr = sgdt();
        let guest_gdt = gdtr.base as u64;

        self.state_save_area.es_selector = segmentation::es().bits();
        self.state_save_area.cs_selector = segmentation::cs().bits();
        self.state_save_area.ss_selector = segmentation::ss().bits();
        self.state_save_area.ds_selector = segmentation::ds().bits();

        self.state_save_area.es_attrib = get_segment_access_right(guest_gdt, segmentation::es().bits());
        self.state_save_area.cs_attrib = get_segment_access_right(guest_gdt, segmentation::cs().bits());
        self.state_save_area.ss_attrib = get_segment_access_right(guest_gdt, segmentation::ss().bits());
        self.state_save_area.ds_attrib = get_segment_access_right(guest_gdt, segmentation::ds().bits());

        self.state_save_area.es_limit = get_segment_limit(guest_gdt, segmentation::es().bits());
        self.state_save_area.cs_limit = get_segment_limit(guest_gdt, segmentation::cs().bits());
        self.state_save_area.ss_limit = get_segment_limit(guest_gdt, segmentation::ss().bits());
        self.state_save_area.ds_limit = get_segment_limit(guest_gdt, segmentation::ds().bits());

        self.state_save_area.gdtr_base = gdtr.base as _;
        self.state_save_area.gdtr_limit = u32::from(gdtr.limit);
        self.state_save_area.idtr_base = idtr.base as _;
        self.state_save_area.idtr_limit = u32::from(idtr.limit);

        unsafe { self.state_save_area.efer = rdmsr(x86::msr::IA32_EFER) | EFER_SVME; }
        unsafe { self.state_save_area.cr0 = cr0().bits() as _; }
        unsafe { self.state_save_area.cr3 = cr3(); }
        unsafe { self.state_save_area.cr4 = cr4().bits() as _; }
        self.state_save_area.rip = registers.rip;
        self.state_save_area.rsp = registers.rsp;
        self.state_save_area.rflags = registers.rflags;
        self.state_save_area.rax = registers.rax;
        unsafe { self.state_save_area.gpat = rdmsr(x86::msr::IA32_PAT); }

        // VMSAVE copies some of the current register values into VMCB. Take
        // advantage of it.

    }
}