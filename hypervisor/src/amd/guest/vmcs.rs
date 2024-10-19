use crate::amd::guest::{ControlArea, StateSaveArea};

#[derive(Debug, Default)]
#[repr(C, align(4096))]
pub struct VmcbRaw {
    pub(crate) control_area: ControlArea,
    pub(crate) state_save_area: StateSaveArea,
}
#[repr(C, align(4096))]
pub struct HostStateAreaRaw([u8; 0x1000]);