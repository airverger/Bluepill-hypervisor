use crate::amd::guest;
use alloc::boxed::Box;

use core::ptr::addr_of;
use kernelutils::{physical_address, PhysicalAllocator, Registers};

use x86::controlregs::{cr0, cr3, cr4};

use x86::msr::{rdmsr, wrmsr};
use x86::segmentation;
use crate::amd::guest::support::{get_segment_access_right, get_segment_limit, sgdt, sidt, SVM_NP_ENABLE_NP_ENABLE};

#[derive(derive_deref::Deref, derive_deref::DerefMut)]
pub struct HostStateArea {
    data: Box<guest::HostStateAreaRaw, PhysicalAllocator>,
}

impl HostStateArea {
    pub fn new() -> Self {
        let dada = unsafe { Box::new_zeroed_in(PhysicalAllocator).assume_init() };
        Self { data: dada }
    }
    #[allow(dead_code)]
    pub fn pa(&self) -> u64 {
        physical_address(addr_of!(*self.data.as_ref()) as _).as_u64()
    }
}


#[derive(derive_deref::Deref, derive_deref::DerefMut)]
#[derive(Debug)]
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
        unsafe {
            wrmsr(SVM_MSR_VM_CR, rdmsr(SVM_MSR_VM_CR) | R_INIT);
        }
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

        self.state_save_area.es_attrib =
            get_segment_access_right(guest_gdt, segmentation::es().bits());
        self.state_save_area.cs_attrib =
            get_segment_access_right(guest_gdt, segmentation::cs().bits());
        self.state_save_area.ss_attrib =
            get_segment_access_right(guest_gdt, segmentation::ss().bits());
        self.state_save_area.ds_attrib =
            get_segment_access_right(guest_gdt, segmentation::ds().bits());

        self.state_save_area.es_limit = get_segment_limit(guest_gdt, segmentation::es().bits());
        self.state_save_area.cs_limit = get_segment_limit(guest_gdt, segmentation::cs().bits());
        self.state_save_area.ss_limit = get_segment_limit(guest_gdt, segmentation::ss().bits());
        self.state_save_area.ds_limit = get_segment_limit(guest_gdt, segmentation::ds().bits());

        self.state_save_area.gdtr_base = gdtr.base as _;
        self.state_save_area.gdtr_limit = u32::from(gdtr.limit);
        self.state_save_area.idtr_base = idtr.base as _;
        self.state_save_area.idtr_limit = u32::from(idtr.limit);

        unsafe {
            self.state_save_area.efer = rdmsr(x86::msr::IA32_EFER) | EFER_SVME;
        }
        unsafe {
            self.state_save_area.cr0 = cr0().bits() as _;
        }
        unsafe {
            self.state_save_area.cr3 = cr3();
        }
        unsafe {
            self.state_save_area.cr4 = cr4().bits() as _;
        }
        self.state_save_area.rip = registers.rip;
        self.state_save_area.rsp = registers.rsp;
        self.state_save_area.rflags = registers.rflags;
        self.state_save_area.rax = registers.rax;
        unsafe {
            self.state_save_area.gpat = rdmsr(x86::msr::IA32_PAT);
        }

        // VMSAVE copies some of the current register values into VMCB. Take
        // advantage of it.
    }
}
