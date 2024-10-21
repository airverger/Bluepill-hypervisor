
use x86::{
    dtables::DescriptorTablePointer,
    segmentation::{SegmentSelector},
};

use crate::amd::guest::SegmentDescriptorRaw;
use crate::amd::guest::support::error::SegmentError;

pub struct SegmentDescriptor {
    low64: SegmentDescriptorRaw,
    upper_base: Option<u32>,
}

impl SegmentDescriptor {
    pub fn try_from_gdtr(
        gdtr: &DescriptorTablePointer<u64>,
        selector: SegmentSelector,
    ) -> Result<Self, SegmentError> {
        if selector.contains(SegmentSelector::TI_LDT) {
            return Err(SegmentError::LdtAccess { selector });
        }

        let index = selector.index() as usize;
        if index == 0 {
            return Err(SegmentError::NullDescriptor { selector });
        }

        let gdt = unsafe {
            core::slice::from_raw_parts(gdtr.base.cast::<u64>(), usize::from(gdtr.limit + 1) / 8)
        };

        let raw = gdt
            .get(index)
            .ok_or(SegmentError::OutOfGdtAccess { index })?;

        let low64 = SegmentDescriptorRaw::from(*raw);
        let upper_base = if low64.is_16byte() {
            let index: usize = index + 1;

            let raw = gdt
                .get(index)
                .ok_or(SegmentError::OutOfGdtAccess { index })?;

            let Ok(upper_base) = u32::try_from(*raw) else {
                return Err(SegmentError::InvalidGdtEntry { index, entry: *raw });
            };

            Some(upper_base)
        } else {
            None
        };
        Ok(Self { low64, upper_base })
    }

    pub fn base(&self) -> u64 {
        if let Some(upper_base) = self.upper_base {
            self.low64.base() as u64 | u64::from(upper_base) << 32
        } else {
            self.low64.base() as _
        }
    }
}

