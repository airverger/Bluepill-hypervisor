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
// const _: () = assert!(core::mem::size_of::<StateSaveArea>() == 0x2e8);
