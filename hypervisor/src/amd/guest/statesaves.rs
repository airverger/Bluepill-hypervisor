#[derive(derivative::Derivative)]
#[derivative(Debug, Default)]
#[repr(C)]
pub struct StateSaveArea {
    pub(crate) es_selector: u16,   // +0x000
    pub(crate) es_attrib: u16,     // +0x002
    pub(crate) es_limit: u32,      // +0x004
    es_base: u64,       // +0x008
    pub(crate) cs_selector: u16,   // +0x010
    pub(crate) cs_attrib: u16,     // +0x012
    pub(crate) cs_limit: u32,      // +0x014
    cs_base: u64,       // +0x018
    pub(crate) ss_selector: u16,   // +0x020
    pub(crate) ss_attrib: u16,     // +0x022
    pub(crate) ss_limit: u32,      // +0x024
    ss_base: u64,       // +0x028
    pub(crate) ds_selector: u16,   // +0x030
    pub(crate) ds_attrib: u16,     // +0x032
    pub(crate) ds_limit: u32,      // +0x034
    ds_base: u64,       // +0x038
    fs_selector: u16,   // +0x040
    fs_attrib: u16,     // +0x042
    fs_limit: u32,      // +0x044
    fs_base: u64,       // +0x048
    gs_selector: u16,   // +0x050
    gs_attrib: u16,     // +0x052
    gs_limit: u32,      // +0x054
    gs_base: u64,       // +0x058
    gdtr_selector: u16, // +0x060 (Reserved)
    gdtr_attrib: u16,   // +0x062 (Reserved)
    pub(crate) gdtr_limit: u32,    // +0x064
    pub(crate) gdtr_base: u64,     // +0x068
    ldtr_selector: u16, // +0x070 (Reserved)
    ldtr_attrib: u16,   // +0x072 (Reserved)
    ldtr_limit: u32,    // +0x074
    ldtr_base: u64,     // +0x078
    idtr_selector: u16, // +0x080
    idtr_attrib: u16,   // +0x082
    pub(crate) idtr_limit: u32,    // +0x084
    pub(crate) idtr_base: u64,     // +0x088
    tr_selector: u16,   // +0x090
    tr_attrib: u16,     // +0x092
    tr_limit: u32,      // +0x094
    tr_base: u64,       // +0x098
    #[derivative(Debug = "ignore", Default(value = "[0; 43]"))]
    _padding1: [u8; 0x0cb - 0x0a0], // +0x0a0
    cpl: u8,            // +0x0cb
    #[derivative(Debug = "ignore")]
    _padding2: u32, // +0x0cc
    pub(crate) efer: u64,          // +0x0d0
    #[derivative(Debug = "ignore", Default(value = "[0; 112]"))]
    _padding3: [u8; 0x148 - 0x0d8], // +0x0d8
    pub(crate) cr4: u64,           // +0x148
    pub(crate) cr3: u64,           // +0x150
    pub(crate) cr0: u64,           // +0x158
    dr7: u64,           // +0x160
    dr6: u64,           // +0x168
    pub(crate) rflags: u64,        // +0x170
    pub(crate) rip: u64,           // +0x178
    #[derivative(Debug = "ignore", Default(value = "[0; 88]"))]
    _padding4: [u8; 0x1d8 - 0x180], // +0x180
    pub(crate) rsp: u64,           // +0x1d8
    s_cet: u64,         // +0x1e0
    ssp: u64,           // +0x1e8
    isst_addr: u64,     // +0x1f0
    pub(crate) rax: u64,           // +0x1f8
    star: u64,          // +0x200
    lstar: u64,         // +0x208
    cstar: u64,         // +0x210
    sf_mask: u64,       // +0x218
    kernel_gs_base: u64, // +0x220
    sysenter_cs: u64,   // +0x228
    sysenter_esp: u64,  // +0x230
    sysenter_eip: u64,  // +0x238
    cr2: u64,           // +0x240
    #[derivative(Debug = "ignore", Default(value = "[0; 32]"))]
    _padding5: [u8; 0x268 - 0x248], // +0x248
    pub(crate) gpat: u64,          // +0x268
    dbg_ctl: u64,       // +0x270
    br_from: u64,       // +0x278
    br_to: u64,         // +0x280
    last_excep_from: u64, // +0x288
    last_excep_to: u64, // +0x290
    #[derivative(Debug = "ignore", Default(value = "[0; 71]"))]
    _padding6: [u8; 0x2df - 0x298], // +0x298
    spec_ctl: u64,      // +0x2e0
}
// const _: () = assert!(core::mem::size_of::<StateSaveArea>() == 0x2e8);
