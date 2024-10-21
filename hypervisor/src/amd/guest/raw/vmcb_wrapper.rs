use alloc::vec::Vec;
use bit_field::BitField;
use x86::bits64::task::TaskStateSegment;
use x86::dtables::{lgdt, DescriptorTablePointer};
use x86::segmentation::{cs, BuildDescriptor, Descriptor, DescriptorBuilder, GateDescriptorBuilder, SegmentSelector, SystemDescriptorTypes64};
use x86::task::{load_tr, tr};
use crate::amd::guest::area::SegmentDescriptor;
use crate::amd::guest::support::error::GdtTssError;

///! c style structure define

/// vmcb control area
#[derive(derivative::Derivative)]
#[derivative(Debug, Default)]
#[repr(C)]
pub struct ControlArea {
    intercept_cr_read: u16,              // +0x000
    intercept_cr_write: u16,             // +0x002
    pub(crate) intercept_dr_read: u16,   // +0x004
    intercept_dr_write: u16,             // +0x006
    pub(crate) intercept_exception: u32, // +0x008
    pub(crate) intercept_misc1: u32,     // +0x00c
    pub(crate) intercept_misc2: u32,     // +0x010
    intercept_misc3: u32,                // +0x014
    #[derivative(Debug = "ignore", Default(value = "[0; 36]"))]
    _padding1: [u8; 0x03c - 0x018], // +0x018
    pause_filter_threshold: u16,         // +0x03c
    pub(crate) pause_filter_count: u16,  // +0x03e
    iopm_base_pa: u64,                   // +0x040
    msrpm_base_pa: u64,                  // +0x048
    tsc_offset: u64,                     // +0x050
    pub(crate) guest_asid: u32,          // +0x058
    pub(crate) tlb_control: u32,         // +0x05c
    vintr: u64,                          // +0x060
    interrupt_shadow: u64,               // +0x068
    pub(crate) exit_code: u64,           // +0x070
    pub(crate) exit_info1: u64,          // +0x078
    pub(crate) exit_info2: u64,          // +0x080
    exit_int_info: u64,                  // +0x088
    pub(crate) np_enable: u64,           // +0x090
    avic_apic_bar: u64,                  // +0x098
    guest_pa_pf_ghcb: u64,               // +0x0a0
    event_inj: u64,                      // +0x0a8
    pub(crate) ncr3: u64,                // +0x0b0
    lbr_virtualization_enable: u64,      // +0x0b8
    pub(crate) vmcb_clean: u32,          // +0x0c0
    _reserved: u32,                      // +0x0c4
    pub(crate) nrip: u64,                // +0x0c8
    pub(crate) num_of_bytes_fetched: u8, // +0x0d0
    pub(crate) guest_instruction_bytes: [u8; 15], // +0x0d1
    avic_apic_backing_page_pointer: u64, // +0x0e0
    #[derivative(Debug = "ignore")]
    _padding2: u64, // +0x0e8
    avic_logical_table_pointer: u64,     // +0x0f0
    avic_physical_table_pointer: u64,    // +0x0f8
    #[derivative(Debug = "ignore")]
    _padding3: u64, // +0x100
    vmcb_save_state_pointer: u64,        // +0x108
    #[derivative(Debug = "ignore", Default(value = "[0; 720]"))]
    _padding4: [u8; 0x3e0 - 0x110], // +0x110
    reserved_for_host: [u8; 0x20],       // +0x3e0
}


/// vmcb state save area
#[derive(derivative::Derivative)]
#[derivative(Debug, Default)]
#[repr(C)]
pub struct StateSaveArea {
    pub(crate) es_selector: u16, // +0x000
    pub(crate) es_attrib: u16,   // +0x002
    pub(crate) es_limit: u32,    // +0x004
    pub(crate) es_base: u64,                // +0x008
    pub(crate) cs_selector: u16, // +0x010
    pub(crate) cs_attrib: u16,   // +0x012
    pub(crate) cs_limit: u32,    // +0x014
    pub(crate) cs_base: u64,                // +0x018
    pub(crate) ss_selector: u16, // +0x020
    pub(crate) ss_attrib: u16,   // +0x022
    pub(crate) ss_limit: u32,    // +0x024
    pub(crate) ss_base: u64,                // +0x028
    pub(crate) ds_selector: u16, // +0x030
    pub(crate) ds_attrib: u16,   // +0x032
    pub(crate) ds_limit: u32,    // +0x034
    pub(crate) ds_base: u64,                // +0x038
    pub(crate) fs_selector: u16,            // +0x040
    pub(crate) fs_attrib: u16,              // +0x042
    pub(crate) fs_limit: u32,               // +0x044
    pub(crate) fs_base: u64,                // +0x048
    pub(crate) gs_selector: u16,            // +0x050
    pub(crate) gs_attrib: u16,              // +0x052
    pub(crate) gs_limit: u32,               // +0x054
    pub(crate) gs_base: u64,                // +0x058
    pub(crate) gdtr_selector: u16,          // +0x060 (Reserved)
    pub(crate) gdtr_attrib: u16,            // +0x062 (Reserved)
    pub(crate) gdtr_limit: u32,  // +0x064
    pub(crate) gdtr_base: u64,   // +0x068
    pub(crate) ldtr_selector: u16,          // +0x070 (Reserved)
    pub(crate) ldtr_attrib: u16,            // +0x072 (Reserved)
    pub(crate) ldtr_limit: u32,             // +0x074
    pub(crate) ldtr_base: u64,              // +0x078
    pub(crate) idtr_selector: u16,          // +0x080
    pub(crate) idtr_attrib: u16,            // +0x082
    pub(crate) idtr_limit: u32,  // +0x084
    pub(crate) idtr_base: u64,   // +0x088
    pub(crate) tr_selector: u16,            // +0x090
    pub(crate) tr_attrib: u16,              // +0x092
    pub(crate) tr_limit: u32,               // +0x094
    pub(crate) tr_base: u64,                // +0x098
    #[derivative(Debug = "ignore", Default(value = "[0; 43]"))]
    _padding1: [u8; 0x0cb - 0x0a0], // +0x0a0
    pub(crate) cpl: u8,                     // +0x0cb
    #[derivative(Debug = "ignore")]
    _padding2: u32, // +0x0cc
    pub(crate) efer: u64,        // +0x0d0
    #[derivative(Debug = "ignore", Default(value = "[0; 112]"))]
    _padding3: [u8; 0x148 - 0x0d8], // +0x0d8
    pub(crate) cr4: u64,         // +0x148
    pub(crate) cr3: u64,         // +0x150
    pub(crate) cr0: u64,         // +0x158
    pub(crate) dr7: u64,                    // +0x160
    pub(crate) dr6: u64,                    // +0x168
    pub(crate) rflags: u64,      // +0x170
    pub(crate) rip: u64,         // +0x178
    #[derivative(Debug = "ignore", Default(value = "[0; 88]"))]
    _padding4: [u8; 0x1d8 - 0x180], // +0x180
    pub(crate) rsp: u64,         // +0x1d8
    pub(crate) s_cet: u64,                  // +0x1e0
    pub(crate) ssp: u64,                    // +0x1e8
    pub(crate) isst_addr: u64,              // +0x1f0
    pub(crate) rax: u64,         // +0x1f8
    pub(crate) star: u64,                   // +0x200
    pub(crate) lstar: u64,                  // +0x208
    pub(crate) cstar: u64,                  // +0x210
    pub(crate) sf_mask: u64,                // +0x218
    pub(crate) kernel_gs_base: u64,         // +0x220
    pub(crate) sysenter_cs: u64,            // +0x228
    pub(crate) sysenter_esp: u64,           // +0x230
    pub(crate) sysenter_eip: u64,           // +0x238
    pub(crate) cr2: u64,                    // +0x240
    #[derivative(Debug = "ignore", Default(value = "[0; 32]"))]
    _padding5: [u8; 0x268 - 0x248], // +0x248
    pub(crate) gpat: u64,        // +0x268
    pub(crate) dbg_ctl: u64,                // +0x270
    pub(crate) br_from: u64,                // +0x278
    pub(crate) br_to: u64,                  // +0x280
    pub(crate) last_excep_from: u64,        // +0x288
    pub(crate) last_excep_to: u64,          // +0x290
    #[derivative(Debug = "ignore", Default(value = "[0; 71]"))]
    _padding6: [u8; 0x2df - 0x298], // +0x298
    pub(crate) spec_ctl: u64,               // +0x2e0
}


/// Raw VMCB structure
#[derive(Debug, Default)]
#[repr(C, align(4096))]
pub struct VmcbRaw {
    pub(crate) control_area: ControlArea,
    pub(crate) state_save_area: StateSaveArea,
}


/// raw VMCB HOST AREA structure
#[derive(derivative::Derivative)]
#[derive(Debug)]
#[repr(C, align(4096))]
pub struct HostStateAreaRaw(
    [u8; 0x1000
    ]
);


/// Raw representation of a segment descriptor.
/// See: 3.4.5 Segment Descriptors
pub struct SegmentDescriptorRaw {
   pub raw: u64,
}

impl SegmentDescriptorRaw {
    // "In 64-bit mode, the TSS descriptor is expanded to 16 bytes (...)."
    // See: 8.2.3 TSS Descriptor in 64-bit mode
    pub(crate) fn is_16byte(&self) -> bool {
        let high32 = self.raw.get_bits(32..);
        let system = high32.get_bit(12); // descriptor type
        let type_ = high32.get_bits(8..=11) as u8;
        !system
            && (type_ == SystemDescriptorTypes64::TssAvailable as u8
            || type_ == SystemDescriptorTypes64::TssBusy as u8)
    }

    pub(crate) fn base(&self) -> u32 {
        let low32 = self.raw.get_bits(..=31);
        let high32 = self.raw.get_bits(32..);

        let base_high = high32.get_bits(24..=31) << 24;
        let base_middle = high32.get_bits(0..=7) << 16;
        let base_low = low32.get_bits(16..=31);
        u32::try_from(base_high | base_middle | base_low).unwrap()
    }
}

impl From<u64> for SegmentDescriptorRaw {
    fn from(raw: u64) -> Self {
        Self { raw }
    }
}


pub type Gdtr = DescriptorTablePointer<u64>;
#[derive(Clone, Debug)]
pub struct GdtTssRaw {
    pub gdt: Vec<u64>,
    #[allow(dead_code)]
    pub cs: SegmentSelector,
    pub tss: Option<TaskStateSegment>,
    pub tr: Option<SegmentSelector>,
}



impl GdtTssRaw {
    #[allow(dead_code)]
    pub fn new_from_current() -> Self {
        let gdtr = Self::sgdt();

        let gdt =
            unsafe { core::slice::from_raw_parts(gdtr.base, usize::from(gdtr.limit + 1) / 8) }
                .to_vec();

        let tr = unsafe { tr() };
        let tr = if tr.bits() == 0 { None } else { Some(tr) };

        let tss = if let Some(tr) = tr {
            let sg = SegmentDescriptor::try_from_gdtr(&gdtr, tr).unwrap();
            let tss = sg.base() as *mut TaskStateSegment;
            Some(unsafe { *tss })
        } else {
            None
        };

        let cs = cs();
        Self { gdt, cs, tss, tr }
    }

    #[allow(dead_code)]
    pub fn append_tss(&mut self, tss: TaskStateSegment) -> &Self {
        if self.tss.is_some() || self.tr.is_some() {
            return self;
        }

        let index = self.gdt.len() as u16;
        self.tr = Some(SegmentSelector::new(index, x86::Ring::Ring0));
        self.tss = Some(tss);

        let tss = self.tss.as_ref().unwrap();
        self.gdt.push(Self::task_segment_descriptor(tss).as_u64());
        self.gdt.push(0);

        self
    }

    #[allow(dead_code)]
    pub fn apply(&self) -> Result<(), GdtTssError> {
        if unsafe { tr() }.bits() != 0 {
            return Err(GdtTssError::TssAlreadyInUse);
        }

        let gdtr = Gdtr::new_from_slice(&self.gdt);
        unsafe { lgdt(&gdtr) };

        if let Some(tr) = self.tr {
            unsafe { load_tr(tr) };
        }

        Ok(())
    }

    /// Builds a segment descriptor from the task state segment.
    // FIXME: Just define our own one and properly represent 128 bit width descriptor.
    fn task_segment_descriptor(tss: &TaskStateSegment) -> Descriptor {
        let base = tss as *const _ as _;
        let limit = core::mem::size_of_val(tss) as u64 - 1;
        <DescriptorBuilder as GateDescriptorBuilder<u32>>::tss_descriptor(base, limit, true)
            .present()
            .dpl(x86::Ring::Ring0)
            .finish()
    }

    fn sgdt() ->Gdtr {
        let mut gdtr = Gdtr::default();
        unsafe { x86::dtables::sgdt(&mut gdtr) };
        gdtr
    }
}
