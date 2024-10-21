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
// const _: () = assert!(core::mem::size_of::<ControlArea>() == 0x400);
