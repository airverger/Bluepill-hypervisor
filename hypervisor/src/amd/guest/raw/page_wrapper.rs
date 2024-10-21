#[derive(Debug, Clone, Copy)]
pub struct Pml4(pub(crate) Table);

#[derive(Debug, Clone, Copy)]
pub struct Pdpt(pub(crate) Table);

#[derive(Debug, Clone, Copy)]
pub struct Pd(pub(crate) Table);

#[derive(Debug, Clone, Copy)]
pub struct Pt(#[allow(dead_code)] pub(crate) Table);

#[derive(Debug, Clone, Copy)]
#[repr(C, align(4096))]
pub struct Table {
    pub(crate) entries: [Entry; 512],
}

bitfield::bitfield! {
    #[derive(Clone, Copy)]
    pub struct Entry(u64);
    impl Debug;
    pub present, set_present: 0;
    pub writable, set_writable: 1;
    pub user, set_user: 2;
    pub large, set_large: 7;
    pub pfn, set_pfn: 51, 12;
}
#[derive(Debug)]
pub struct PagingStructuresRaw {
    pub(crate) pml4: Pml4,
    pub(crate) pdpt: Pdpt,
    pub(crate) pd: [Pd; 512],
    pub(crate) pt: Pt,
    pub(crate) pt_apic: Pt,
}
