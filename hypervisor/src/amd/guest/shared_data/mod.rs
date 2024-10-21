use alloc::vec::Vec;
use core::sync::atomic::AtomicU8;
use spin::{Lazy, Once, RwLock};
use crate::amd::guest::gdt_tss::GdtTss;
use crate::amd::guest::interrupt_handlers::InterruptDescriptorTable;
use crate::amd::guest::NestedPageTables;
use crate::amd::guest::paging::PagingStructures;
use crate::amd::guest::support::GuestActivityState;

pub struct SharedGuestData {
    pub npt: RwLock<NestedPageTables>,
    pub activity_states: [AtomicU8; 0xff],
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
pub static SHARED_GUEST_DATA: Lazy<SharedGuestData> = Lazy::new(SharedGuestData::new);

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

pub static SHARED_HOST_DATA: Once<SharedHostData> = Once::new();
