use x86::segmentation::SegmentSelector;

#[derive(thiserror_no_std::Error, Debug)]
pub(crate) enum SegmentError {
    #[error("`{selector}` points to the null descriptor")]
    NullDescriptor { selector: SegmentSelector },

    #[error("`{selector}` points to LDT where parsing is unimplemented")]
    LdtAccess { selector: SegmentSelector },

    #[error("`{index}` points to outside GDT")]
    OutOfGdtAccess { index: usize },

    #[error("`{index}` points to `{entry}`, which is invalid as a descriptor")]
    InvalidGdtEntry { index: usize, entry: u64 },
}

#[derive(thiserror_no_std::Error, Clone, Copy, Debug)]
pub enum GdtTssError {
    #[error("TSS already in use in the current GDT")]
    TssAlreadyInUse,
}
