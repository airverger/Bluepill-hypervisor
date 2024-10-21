//! This module implements management of GDT with TSS. TSS is used because Intel
//! processors require the host GDT to have a valid TSS.

use alloc::{boxed::Box};

use crate::amd::guest::GdtTssRaw;




#[derive(Clone, Debug, derive_deref::Deref, derive_deref::DerefMut)]
pub struct GdtTss {
    data: Box<GdtTssRaw>,
}

impl GdtTss  {
    #[allow(dead_code)]
    pub fn new_from_current() -> Self {
        Self {
            data: Box::new(GdtTssRaw::new_from_current()),
        }
    }
}


