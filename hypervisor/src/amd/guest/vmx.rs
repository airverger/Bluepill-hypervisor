use alloc::boxed::Box;
use core::arch::{asm, global_asm};
use core::ptr::addr_of;
use core::sync::atomic::Ordering;
use bit_field::BitField;
use x86::bits64::paging::BASE_PAGE_SHIFT;
use x86::controlregs::{cr0, cr3, cr4};
use x86::dtables::DescriptorTablePointer;
use x86::msr::{rdmsr, wrmsr};
use x86::segmentation;

use crate::amd::{guest, InstructionInfo, VmExitReason};
use kernelutils::{physical_address, PhysicalAllocator, Registers};
use crate::amd::guest::{apic_id, paging, NestedPageTables};
use crate::amd::guest::vmm::{get_segment_access_right, get_segment_limit, run_svm_guest, sgdt, sidt, vmsave, Vmcb};


#[derive(derive_deref::Deref, derive_deref::DerefMut)]
struct HostStateArea {
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


pub struct Vmx {
    #[allow(dead_code)]
    id: usize,
    pub guest_vmcb: Vmcb,

    #[allow(dead_code)]
    host_vmcb: Vmcb,
    guest_vmcb_pa: u64,
    host_vmcb_pa: u64,

    pub npt: NestedPageTables,

    // #[derivative(Debug = "ignore")]
    host_state: HostStateArea,
    registers: Registers,
    // activity_state: &'static AtomicU8,
}

impl Vmx {
    fn initialize_control(&mut self) {
        self.guest_vmcb.initialize_control();
    }
    fn initialize_guest(&mut self) {
        self.guest_vmcb.initialize_guest(&self.registers);


        vmsave(self.guest_vmcb_pa);
    }
    fn initialize_host(&mut self) {
        vmsave(self.host_vmcb_pa);
    }
    pub(crate) fn initialize(&mut self, registers: &Registers) {
        self.initialize_control();
        self.initialize_guest();
        self.initialize_host();
    }
    pub(crate) fn activate(&mut self) {
        const SVM_MSR_VM_HSAVE_PA: u32 = 0xc001_0117;

        // Need to specify the address of the host state-save area before executing
        // the VMRUN instruction. The host state-save area is where the processor
        // saves the host (ie, current) register values on execution of `VMRUN`.
        //
        // "The VMRUN instruction saves some host processor state information in
        //  the host state-save area in main memory at the physical address
        //  specified in the VM_HSAVE_PA MSR".
        // See: 15.5.1 Basic Operation
        let pa = physical_address(addr_of!(*self.host_state.as_ref()) as _).as_u64();
        unsafe { wrmsr(SVM_MSR_VM_HSAVE_PA, pa); }
    }
}
impl Vmx {
    #[allow(dead_code)]
    fn intercept_apic_write(&mut self, enable: bool) {
        let apic_base_raw = unsafe { rdmsr(x86::msr::IA32_APIC_BASE) };
        let apic_base = apic_base_raw & !0xfff;
        let pt_index = apic_base.get_bits(12..=20) as usize; // [20:12]

        let  npt = &mut self.npt;
        let pt = npt.apic_pt();
        pt.0.entries[pt_index].set_writable(!enable);

        // Other processors will have stale TLB entries as we do not do TLB
        // shootdown. It is fine because APIC writes we want to see are done by
        // this processors. We need to handle #VMEXIT(NFP) on other processors
        // if it happens.
        self.guest_vmcb.control_area.tlb_control = 1;
    }

    fn handle_nested_page_fault(&mut self) {
        // if self.id == apic_id::PROCESSOR_COUNT.load(Ordering::Relaxed) - 1 {
        //     log::debug!("Stopping APIC write interception");
        //     self.intercept_apic_write(false);
        // }
        unsafe { asm!("int 3") }
    }


    #[allow(dead_code)]
    pub(crate) fn run(&mut self) -> VmExitReason {
        const VMEXIT_EXCEPTION_SX: u64 = 0x5e;
        const VMEXIT_CPUID: u64 = 0x72;
        const VMEXIT_NPF: u64 = 0x400;
        self.guest_vmcb.state_save_area.rax = self.registers.rax;
        self.guest_vmcb.state_save_area.rip = self.registers.rip;
        self.guest_vmcb.state_save_area.rsp = self.registers.rsp;
        self.guest_vmcb.state_save_area.rflags = self.registers.rflags;
        self.guest_vmcb.control_area.tlb_control = 0 as _;
        self.guest_vmcb.control_area.vmcb_clean = u32::MAX;

        log::trace!("Entering the guest");
        unsafe { asm!("int 3") }
        unsafe { run_svm_guest(&mut self.registers, self.guest_vmcb_pa, self.host_vmcb_pa) };
        log::trace!("Exited the guest");
        let reason = self.guest_vmcb.control_area.exit_code;
        match reason {
            VMEXIT_CPUID => VmExitReason::Cpuid(InstructionInfo {
                next_rip: self.guest_vmcb.control_area.nrip,
            }),
            VMEXIT_NPF => {
                unsafe { asm!("int 3") };
                self.handle_nested_page_fault();
                VmExitReason::NestedPageFault
            }
            _ => {
                unsafe { asm!("int 3") }
                VmExitReason::Unknown
            }
        }
    }
}

impl Vmx {
    #[allow(dead_code)]
    pub fn new(id: usize, registers: &Registers) -> Self {
        let guest_vmcb = Vmcb::new();

        let host_vmcb = Vmcb::new();

        let guest_vmcb_pa = guest_vmcb.pa();
        let host_vmcb_pa = host_vmcb.pa();
        let host_state = HostStateArea::new();
        let mut npt = NestedPageTables::new();
        npt.build_identity();
        npt.split_apic_page();
        Self {
            id,
            guest_vmcb,
            host_vmcb,
            guest_vmcb_pa,
            host_vmcb_pa,
            host_state,
            npt,
            registers: *registers,
        }
    }
}

impl Vmx {

    #[allow(dead_code)]
    pub(crate) fn regs(&mut self) -> &mut Registers {
        &mut self.registers
    }
}
