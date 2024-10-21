use alloc::boxed::Box;
use alloc::vec::Vec;
use bit_field::BitField;
use core::arch::{asm, global_asm};
use core::mem::zeroed;
use core::ptr::addr_of;
use core::sync::atomic::{AtomicU8, Ordering};
use spin::{Lazy, Once, RwLock};
use x86::bits64::paging::BASE_PAGE_SHIFT;
use x86::controlregs::{cr0, cr3, cr3_write, cr4};
use x86::cpuid::cpuid;
use x86::current::rflags::RFlags;
use x86::dtables::{ DescriptorTablePointer};
use x86::msr::{rdmsr, wrmsr};
use x86::segmentation;
use x86::segmentation::{cs, ds, es, ss};

use crate::amd::guest::{apic_id, paging, NestedPageTables};
use crate::amd::{guest, InstructionInfo, VmExitReason};
use kernelutils::{physical_address, PhysicalAllocator, Registers};
use kernelutils::nt::platform_ops;
use crate::amd::guest::gdt_tss::GdtTss;
use crate::amd::guest::interrupt_handlers::InterruptDescriptorTable;
use crate::amd::guest::paging::PagingStructures;
use crate::amd::guest::support::{get_segment_access_right, get_segment_limit, run_svm_guest, sgdt, sidt, vmsave, GuestActivityState, TlbControl};
use crate::amd::guest::vmm::{HostStateArea, Vmcb};

pub struct Vmx {
    #[allow(dead_code)]
    id: usize,
    pub guest_vmcb: Vmcb,

    #[allow(dead_code)]
    host_vmcb: Vmcb,
    guest_vmcb_pa: u64,
    host_vmcb_pa: u64,

    // #[derivative(Debug = "ignore")]
    host_state: HostStateArea,
    registers: Registers,
    activity_state: &'static AtomicU8,
}
struct SharedGuestData {
    npt: RwLock<NestedPageTables>,
    activity_states: [AtomicU8; 0xff],
}

impl SharedGuestData {
    fn new() -> Self {
        let mut npt = NestedPageTables::new();
        npt.build_identity();
        npt.split_apic_page();

        Self {
            npt: RwLock::new(npt),
            activity_states: core::array::from_fn(|_| {
                AtomicU8::new(GuestActivityState::Active as u8)
            }),
        }
    }
}
static SHARED_GUEST_DATA: Lazy<SharedGuestData> = Lazy::new(SharedGuestData::new);

/// A collection of data that the host depends on for its entire lifespan.
#[derive(Debug, Default)]
pub struct SharedHostData {
    /// The paging structures for the host. If `None`, the current paging
    /// structure is used for both the host and the guest.
    pub pt: Option<PagingStructures>,

    /// The IDT for the host. If `None`, the current IDT is used for both the
    /// host and the guest.
    pub idt: Option<InterruptDescriptorTable>,

    /// The GDT and TSS for the host for each logical processor. If `None`,
    /// the current GDTs and TSSes are used for both the host and the guest.
    pub gdts: Option<Vec<GdtTss>>,
}

static SHARED_HOST_DATA: Once<SharedHostData> = Once::new();
pub(crate) fn lidt(idtr: &DescriptorTablePointer<u64>) {
    unsafe { x86::dtables::lidt(idtr) };
}
impl Vmx {
    fn handle_security_exception(&mut self) {
        assert!(self.id != 0);
        self.handle_init_signal();
        self.handle_sipi(self.wait_for_sipi());
    }

    fn handle_init_signal(&mut self) {
        const EFER_SVME: u64 = 1 << 12;

        assert!(self.id != 0);

        // Update the state to Wait-for-SIPI as soon as possible since we are
        // racing against BSP sending SIPI.
        assert!(
            self.activity_state
                .swap(GuestActivityState::WaitForSipi as u8, Ordering::Relaxed)
                == GuestActivityState::Active as u8
        );

        log::debug!("INIT");

        // Extension Type
        // Not Write-through
        // Cache Disabled
        let previous_cr0 = unsafe { cr0() }.bits();
        let new_cr0 = 1u64 << 4
            | (previous_cr0.get_bit(29) as u64) << 29
            | (previous_cr0.get_bit(30) as u64) << 30;
        self.guest_vmcb.state_save_area.cr0 = new_cr0;
        self.guest_vmcb.state_save_area.cr2 = 0;
        self.guest_vmcb.state_save_area.cr3 = 0;
        self.guest_vmcb.state_save_area.cr4 = 0;
        self.guest_vmcb.state_save_area.rflags = RFlags::FLAGS_A1.bits();
        self.guest_vmcb.state_save_area.efer = EFER_SVME;
        self.guest_vmcb.state_save_area.rip = 0xfff0;
        self.guest_vmcb.state_save_area.cs_selector = 0xf000;
        self.guest_vmcb.state_save_area.cs_base = 0xffff0000;
        self.guest_vmcb.state_save_area.cs_limit = 0xffff;
        self.guest_vmcb.state_save_area.cs_attrib = 0x9b;
        self.guest_vmcb.state_save_area.ds_selector = 0;
        self.guest_vmcb.state_save_area.ds_base = 0;
        self.guest_vmcb.state_save_area.ds_limit = 0xffff;
        self.guest_vmcb.state_save_area.ds_attrib = 0x93;
        self.guest_vmcb.state_save_area.es_selector = 0;
        self.guest_vmcb.state_save_area.es_base = 0;
        self.guest_vmcb.state_save_area.es_limit = 0xffff;
        self.guest_vmcb.state_save_area.es_attrib = 0x93;
        self.guest_vmcb.state_save_area.fs_selector = 0;
        self.guest_vmcb.state_save_area.fs_base = 0;
        self.guest_vmcb.state_save_area.fs_limit = 0xffff;
        self.guest_vmcb.state_save_area.fs_attrib = 0x93;
        self.guest_vmcb.state_save_area.gs_selector = 0;
        self.guest_vmcb.state_save_area.gs_base = 0;
        self.guest_vmcb.state_save_area.gs_limit = 0xffff;
        self.guest_vmcb.state_save_area.gs_attrib = 0x93;
        self.guest_vmcb.state_save_area.ds_selector = 0;
        self.guest_vmcb.state_save_area.ds_base = 0;
        self.guest_vmcb.state_save_area.ds_limit = 0xffff;
        self.guest_vmcb.state_save_area.ds_attrib = 0x93;
        self.guest_vmcb.state_save_area.gdtr_base = 0;
        self.guest_vmcb.state_save_area.gdtr_limit = 0xffff;
        self.guest_vmcb.state_save_area.idtr_base = 0;
        self.guest_vmcb.state_save_area.idtr_limit = 0xffff;
        self.guest_vmcb.state_save_area.ldtr_selector = 0;
        self.guest_vmcb.state_save_area.ldtr_base = 0;
        self.guest_vmcb.state_save_area.ldtr_limit = 0xffff;
        self.guest_vmcb.state_save_area.ldtr_attrib = 0x82;
        self.guest_vmcb.state_save_area.tr_selector = 0;
        self.guest_vmcb.state_save_area.tr_base = 0;
        self.guest_vmcb.state_save_area.tr_limit = 0xffff;
        self.guest_vmcb.state_save_area.tr_attrib = 0x8b;
        self.registers.rax = 0;
        self.registers.rdx = cpuid!(0x1).eax as _;
        self.registers.rbx = 0;
        self.registers.rcx = 0;
        self.registers.rbp = 0;
        self.guest_vmcb.state_save_area.rsp = 0;
        self.registers.rdi = 0;
        self.registers.rsi = 0;
        self.registers.r8 = 0;
        self.registers.r9 = 0;
        self.registers.r10 = 0;
        self.registers.r11 = 0;
        self.registers.r12 = 0;
        self.registers.r13 = 0;
        self.registers.r14 = 0;
        self.registers.r15 = 0;
        unsafe {
            x86::debugregs::dr0_write(0);
            x86::debugregs::dr1_write(0);
            x86::debugregs::dr2_write(0);
            x86::debugregs::dr3_write(0);
        };
        self.guest_vmcb.state_save_area.dr6 = 0xffff0ff0;
        self.guest_vmcb.state_save_area.dr7 = 0x400;

        self.guest_vmcb.control_area.tlb_control = TlbControl::FlushAll as _;
        self.guest_vmcb.control_area.vmcb_clean = 0;
    }

    fn wait_for_sipi(&self) -> u8 {
        assert!(self.id != 0);

        // Wait for SIPI sent from BSP.
        while self.activity_state.load(Ordering::Relaxed) == GuestActivityState::WaitForSipi as u8 {
            core::hint::spin_loop();
        }

        // Received SIPI. Fetch the vector value and get out of the Wait-for-SIPI state.
        self.activity_state
            .swap(GuestActivityState::Active as u8, Ordering::Relaxed)
    }

    fn handle_sipi(&mut self, vector: u8) {
        assert!(self.id != 0);
        assert!(self.activity_state.load(Ordering::Relaxed) == GuestActivityState::Active as u8);
        log::debug!("SIPI vector {vector:#x?}");

        self.guest_vmcb.state_save_area.cs_selector = (vector as u16) << 8;
        self.guest_vmcb.state_save_area.cs_base = (vector as u64) << 12;
        self.guest_vmcb.state_save_area.rip = 0;
        self.registers.rip = 0;
    }

    fn intercept_apic_write(&mut self, enable: bool) {
        let apic_base_raw = unsafe { rdmsr(x86::msr::IA32_APIC_BASE) };
        let apic_base = apic_base_raw & !0xfff;
        let pt_index = apic_base.get_bits(12..=20) as usize; // [20:12]

        let mut npt = SHARED_GUEST_DATA.npt.write();
        let pt = npt.apic_pt();
        pt.0.entries[pt_index].set_writable(!enable);

        // Other processors will have stale TLB entries as we do not do TLB
        // shootdown. It is fine because APIC writes we want to see are done by
        // this processors. We need to handle #VMEXIT(NFP) on other processors
        // if it happens.
        self.guest_vmcb.control_area.tlb_control = TlbControl::FlushAll as _;
    }

    fn handle_nested_page_fault(&mut self) {
        unsafe { asm!("int 3") }
        if self.id == apic_id::PROCESSOR_COUNT.load(Ordering::Relaxed) - 1 {
            log::debug!("Stopping APIC write interception");
            self.intercept_apic_write(false);
        }

        let instructions = unsafe {
            core::slice::from_raw_parts(
                self.guest_vmcb.control_area.guest_instruction_bytes.as_ptr(),
                self.guest_vmcb.control_area.num_of_bytes_fetched as _,
            )
        };

        // This one is by far the most frequent one. Micro-optimize this path by
        // checking this pattern first.
        let (value, instr_len) = if instructions
            .starts_with(&[0xc7, 0x80, 0xb0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00])
        {
            // MOV DWORD PTR [RAX+000000B0],00000000
            (0u32, 10u64)
        } else {
            match instructions {
                [0x45, 0x89, 0x65, 0x00, ..] => {
                    // MOV DWORD PTR [R13],R12D
                    (self.registers.r12 as _, 4)
                }
                [0x41, 0x89, 0x14, 0x00, ..] => {
                    // MOV DWORD PTR [R8+RAX],EDX
                    (self.registers.rdx as _, 4)
                }
                [0xc7, 0x81, 0xb0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, ..] => {
                    // MOV DWORD PTR [RCX+000000B0],00000000
                    (0, 10)
                }
                [0xa3, 0x00, 0x03, 0xe0, 0xfe, 0x00, 0x00, 0x00, 0x00, ..] => {
                    // MOV DWORD PTR [00000000FEE00300],EAX
                    (self.registers.rax as _, 9)
                }
                [0xa3, 0x10, 0x03, 0xe0, 0xfe, 0x00, 0x00, 0x00, 0x00, ..] => {
                    // MOV DWORD PTR [00000000FEE00310],EAX
                    (self.registers.rax as _, 9)
                }
                [0x89, 0x90, 0x00, 0x03, 0x00, 0x00, ..] => {
                    // MOV DWORD PTR [RAX+00000300],EDX
                    (self.registers.rdx as _, 6)
                }
                [0x89, 0x88, 0x10, 0x03, 0x00, 0x00, ..] => {
                    // MOV DWORD PTR [RAX+00000310],ECX
                    (self.registers.rcx as _, 6)
                }
                _ => {
                    log::error!("{:#x?}", self.registers);
                    log::error!("{:#x?}", self.guest_vmcb);
                    panic!("Unhandled APIC access instructions: {:02x?}", instructions);
                }
            }
        };

        self.registers.rip += instr_len;

        let message_type = value.get_bits(8..=10);
        let faulting_gpa = self.guest_vmcb.control_area.exit_info2;
        let apic_register = faulting_gpa & 0xfff;
        if apic_register != 0xb0 && self.id == 0 {
            log::trace!("APIC reg:{apic_register:#x} <= {value:#x}");
        }

        // If the faulting access is not because of sending Startup IPI (0b110)
        // via the Interrupt Command Register Low (0x300), do the write access
        // the guest wanted to do and bail out.
        // Table 16-2. APIC Registers
        if message_type != 0b110 || apic_register != 0x300 {
            // Safety: GPA is same as PA in our NTPs, and the faulting address
            // is always the local APIC page, which is writable in the host
            // address space.
            let apic_reg = faulting_gpa as *mut u32;
            unsafe { apic_reg.write_volatile(value) };
            return;
        }

        // The BSP is trying to send Startup IPI. This must not be allowed because
        // SVM does not intercept it or deliver #VMEXIT. We need to prevent the
        // BSP from sending it and emulate the effect in software instead.

        // Figure 16-18. Interrupt Command Register (APIC Offset 300h–310h)
        assert!(!value.get_bit(11), "Destination Mode must be 'Physical'");
        assert!(
            value.get_bits(18..=19) == 0b00,
            "Destination Shorthand must be 'Destination'"
        );

        // Safety: GPA is same as PA in our NTPs, and the faulting address
        // is always the local APIC page, which is writable in the host
        // address space.
        let icr_high_addr = (faulting_gpa & !0xfff) | 0x310;
        let icr_high_value = unsafe { *(icr_high_addr as *mut u32) };

        // Collect necessary bits to emulate, that is, vector and destination.
        let vector = value.get_bits(0..=7) as u8;
        let apic_id = icr_high_value.get_bits(24..=31) as u8;
        let processor_id = apic_id::processor_id_from(apic_id).unwrap();
        log::debug!("SIPI to {apic_id} with vector {vector:#x?}");
        assert!(vector != GuestActivityState::WaitForSipi as u8);

        // Update the activity state of the target processor with the obtained
        // vector value. The target processor should get out from the busy loop
        // after this. Note that it is possible that the target processor is not
        // yet in the WaitForSipi state when #VMEXIT(#SX) has not been processed.
        // It is fine, as SIPI will be sent twice, and almost certain that 2nd
        // SIPI is late enough.
        let activity_state = &SHARED_GUEST_DATA.activity_states[processor_id];
        let _ = activity_state.compare_exchange(
            GuestActivityState::WaitForSipi as u8,
            vector,
            Ordering::Relaxed,
            Ordering::Relaxed,
        );
    }
}
impl Vmx {
    fn initialize_control(&mut self) {
        const SVM_INTERCEPT_MISC1_CPUID: u32 = 1 << 18;
        const SVM_INTERCEPT_MISC2_VMRUN: u32 = 1 << 0;
        const SVM_NP_ENABLE_NP_ENABLE: u64 = 1 << 0;

        self.guest_vmcb.control_area.intercept_misc1 = SVM_INTERCEPT_MISC1_CPUID;
        self.guest_vmcb.control_area.intercept_misc2 = SVM_INTERCEPT_MISC2_VMRUN;
        self.guest_vmcb.control_area.pause_filter_count = u16::MAX;

        // Address Space Identifier (ASID) is useful when the given logical processor
        // runs more than one guests. We do not but still need to set non-zero value.
        // See: 15.16 TLB Control
        self.guest_vmcb.control_area.guest_asid = 1;

        // Enable nested paging. This is done by:
        // - Setting the NP_ENABLE bit in VMCB, and
        // - Setting the base address of the nested PML4
        //
        // See: 15.25.3 Enabling Nested Paging
        unsafe { asm!("int 3") };
        let nested_pml4_addr = SHARED_GUEST_DATA.npt.read().as_ref() as *const _;
        self.guest_vmcb.control_area.np_enable = SVM_NP_ENABLE_NP_ENABLE;
        self.guest_vmcb.control_area.ncr3 = platform_ops::get().pa(nested_pml4_addr as _);

        // Convert #INIT to #SX. One cannot simply intercept #INIT because even
        // if we do, #INIT is still pending and will be delivered anyway.
        const SVM_MSR_VM_CR: u32 = 0xc001_0114;
        const R_INIT: u64 = 1 << 1;
        unsafe { wrmsr(SVM_MSR_VM_CR, rdmsr(SVM_MSR_VM_CR) | R_INIT); }

        const SECURITY_EXCEPTION: u32 = 1 << 30;
        self.guest_vmcb.control_area.intercept_exception = SECURITY_EXCEPTION;
    }

    fn initialize_guest(&mut self) {
        const EFER_SVME: u64 = 1 << 12;

        let idtr = sidt();
        let gdtr = sgdt();
        let guest_gdt = gdtr.base as u64;

        self.guest_vmcb.state_save_area.es_selector = es().bits();
        self.guest_vmcb.state_save_area.cs_selector = cs().bits();
        self.guest_vmcb.state_save_area.ss_selector = ss().bits();
        self.guest_vmcb.state_save_area.ds_selector = ds().bits();

        self.guest_vmcb.state_save_area.es_attrib = get_segment_access_right(guest_gdt, es().bits());
        self.guest_vmcb.state_save_area.cs_attrib = get_segment_access_right(guest_gdt, cs().bits());
        self.guest_vmcb.state_save_area.ss_attrib = get_segment_access_right(guest_gdt, ss().bits());
        self.guest_vmcb.state_save_area.ds_attrib = get_segment_access_right(guest_gdt, ds().bits());

        self.guest_vmcb.state_save_area.es_limit = get_segment_limit(guest_gdt, es().bits());
        self.guest_vmcb.state_save_area.cs_limit = get_segment_limit(guest_gdt, cs().bits());
        self.guest_vmcb.state_save_area.ss_limit = get_segment_limit(guest_gdt, ss().bits());
        self.guest_vmcb.state_save_area.ds_limit = get_segment_limit(guest_gdt, ds().bits());

        self.guest_vmcb.state_save_area.gdtr_base = gdtr.base as _;
        self.guest_vmcb.state_save_area.gdtr_limit = u32::from(gdtr.limit);
        self.guest_vmcb.state_save_area.idtr_base = idtr.base as _;
        self.guest_vmcb.state_save_area.idtr_limit = u32::from(idtr.limit);

        unsafe { self.guest_vmcb.state_save_area.efer = rdmsr(x86::msr::IA32_EFER) | EFER_SVME; }
        unsafe { self.guest_vmcb.state_save_area.cr0 = cr0().bits() as _; }
        unsafe { self.guest_vmcb.state_save_area.cr3 = cr3(); }
        unsafe { self.guest_vmcb.state_save_area.cr4 = cr4().bits() as _; }
        self.guest_vmcb.state_save_area.rip = self.registers.rip;
        self.guest_vmcb.state_save_area.rsp = self.registers.rsp;
        self.guest_vmcb.state_save_area.rflags = self.registers.rflags;
        self.guest_vmcb.state_save_area.rax = self.registers.rax;
        unsafe { self.guest_vmcb.state_save_area.gpat = rdmsr(x86::msr::IA32_PAT); }

        // VMSAVE copies some of the current register values into VMCB. Take
        // advantage of it.
        vmsave(self.guest_vmcb_pa);
    }

    fn initialize_host(&mut self) {
        let shared_host = SHARED_HOST_DATA.get().unwrap();

        if let Some(host_pt) = &shared_host.pt {
            let pml4 = addr_of!(*host_pt.as_ref());
            unsafe { cr3_write(platform_ops::get().pa(pml4 as _)) };
        }

        if let Some(host_gdt_and_tss) = &shared_host.gdts {
            host_gdt_and_tss[self.id].apply().unwrap();
        }

        if let Some(host_idt) = &shared_host.idt {
            unsafe { lidt(&host_idt.idtr()); }
        }

        // Save some of the current register values as host state. They are
        // restored shortly after #VMEXIT.
        vmsave(self.host_vmcb_pa);
    }
}

impl Vmx {
    pub(crate) fn new(id: usize) -> Self {
        let mut vm = Self {
            id,
            registers: Registers::default(),
            guest_vmcb: Vmcb::new(),
            guest_vmcb_pa: 0,
            host_vmcb: Vmcb::new(),
            host_vmcb_pa: 0,

            host_state: HostStateArea::new(),
            activity_state: &SHARED_GUEST_DATA.activity_states[id],
        };

        vm.guest_vmcb_pa = platform_ops::get().pa(addr_of!(*vm.guest_vmcb.as_ref()) as _);
        vm.host_vmcb_pa = platform_ops::get().pa(addr_of!(*vm.host_vmcb.as_ref()) as _);
        if cfg!(feature = "uefi") && vm.id == 0 {
            vm.intercept_apic_write(true);
        }
        vm
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
        let pa = platform_ops::get().pa(addr_of!(*self.host_state.as_ref()) as _);
        unsafe { wrmsr(SVM_MSR_VM_HSAVE_PA, pa); }
    }

    pub(crate) fn initialize(&mut self, registers: &Registers) {
        self.registers = *registers;
        self.initialize_control();
        self.initialize_guest();
        self.initialize_host();
    }

    pub(crate) fn run(&mut self) -> VmExitReason {
        const VMEXIT_EXCEPTION_SX: u64 = 0x5e;
        const VMEXIT_CPUID: u64 = 0x72;
        const VMEXIT_NPF: u64 = 0x400;

        self.guest_vmcb.state_save_area.rax = self.registers.rax;
        self.guest_vmcb.state_save_area.rip = self.registers.rip;
        self.guest_vmcb.state_save_area.rsp = self.registers.rsp;
        self.guest_vmcb.state_save_area.rflags = self.registers.rflags;

        log::trace!("Entering the guest");

        // Run the guest until the #VMEXIT occurs.
        unsafe { run_svm_guest(&mut self.registers, self.guest_vmcb_pa, self.host_vmcb_pa) };

        log::trace!("Exited the guest");

        // #VMEXIT occurred. Copy the guest register values from VMCB so that
        // `self.registers` is complete and up to date.
        self.registers.rax = self.guest_vmcb.state_save_area.rax;
        self.registers.rip = self.guest_vmcb.state_save_area.rip;
        self.registers.rsp = self.guest_vmcb.state_save_area.rsp;
        self.registers.rflags = self.guest_vmcb.state_save_area.rflags;

        // We might have requested flushing TLB. Clear the request.
        self.guest_vmcb.control_area.tlb_control = TlbControl::DoNotFlush as _;
        self.guest_vmcb.control_area.vmcb_clean = u32::MAX;

        // Handle #VMEXIT by translating it to the `VmExitReason` type.
        //
        // "On #VMEXIT, the processor:
        //  (...)
        //  - Saves the reason for exiting the guest in the VMCB's EXITCODE field."
        // See: 15.6 #VMEXIT
        //
        // For the list of possible exit codes,
        // See: Appendix C SVM Intercept Exit Codes
        match self.guest_vmcb.control_area.exit_code {
            VMEXIT_EXCEPTION_SX => {
                self.handle_security_exception();
                VmExitReason::InitSignal
            }
            VMEXIT_CPUID => VmExitReason::Cpuid(InstructionInfo {
                next_rip: self.guest_vmcb.control_area.nrip,
            }),
            VMEXIT_NPF => {
                unsafe { asm!("int 3") };
                self.handle_nested_page_fault();
                VmExitReason::NestedPageFault
            }
            _ => {
                log::error!("{:#x?}", self.guest_vmcb_pa);
                panic!(
                    "Unhandled #VMEXIT reason: {:?}",
                    self.guest_vmcb.control_area.exit_code
                )
            }
        }
    }

pub    fn regs(&mut self) -> &mut Registers {
        &mut self.registers
    }
}



