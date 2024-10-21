use alloc::boxed::Box;
use core::ptr::addr_of;
use bit_field::BitField;
use x86::bits64::paging::{BASE_PAGE_SHIFT, BASE_PAGE_SIZE, LARGE_PAGE_SIZE};
use x86::msr::rdmsr;
use kernelutils::{physical_address, PhysicalAllocator};
use crate::amd::guest::{Entry, PagingStructuresRaw, Pt};

#[derive(Debug, derive_deref::Deref, derive_deref::DerefMut)]
pub struct PagingStructures {
    data: Box<PagingStructuresRaw, PhysicalAllocator>,
}

impl Default for PagingStructures {
    fn default() -> Self {
        Self::new()
    }
}

impl PagingStructures {
    pub fn new() -> Self {
        Self {
            data: unsafe { Box::new_zeroed_in(PhysicalAllocator).assume_init() },
        }
    }
}


#[derive(derive_deref::Deref, derive_deref::DerefMut)]
pub struct NestedPageTables {
    data: Box<PagingStructuresRaw, PhysicalAllocator>,
}
impl NestedPageTables {
    pub fn apic_pt(&mut self) -> &mut Pt {
        &mut self.pt_apic
    }
}
impl NestedPageTables {
    pub fn new() -> Self {
        let dada = unsafe { Box::new_zeroed_in(PhysicalAllocator).assume_init() };
        Self { data: dada }
    }

    pub(crate) fn build_identity_internal(
        ps: &mut Box<PagingStructuresRaw, PhysicalAllocator>,
        npt: bool,
    ) {
        let user = npt;

        let pml4 = &mut ps.pml4;
        pml4.0.entries[0].set_present(true);
        pml4.0.entries[0].set_writable(true);
        pml4.0.entries[0].set_user(user);
        pml4.0.entries[0]
            .set_pfn(physical_address(addr_of!(ps.pdpt) as _).as_u64() >> BASE_PAGE_SHIFT);

        let mut pa = 0;
        for (i, pdpte) in ps.pdpt.0.entries.iter_mut().enumerate() {
            pdpte.set_present(true);
            pdpte.set_writable(true);
            pdpte.set_user(user);
            pdpte.set_pfn(physical_address(addr_of!(ps.pd[i]) as _).as_u64() >> BASE_PAGE_SHIFT);
            for pde in &mut ps.pd[i].0.entries {
                // The first 2MB is mapped with 4KB pages if it is not for NPT. This
                // is to make the zero page non-present and cause #PF in case of null
                // pointer access. Helps debugging. All other pages are 2MB mapped.
                if pa == 0 && !npt {
                    pde.set_present(true);
                    pde.set_writable(true);
                    pde.set_user(user);
                    pde.set_pfn(physical_address(addr_of!(ps.pt) as _).as_u64() >> BASE_PAGE_SHIFT);
                    for pte in &mut ps.pt.0.entries {
                        pte.set_present(true);
                        pte.set_writable(true);
                        pte.set_user(user);
                        pte.set_pfn(pa >> BASE_PAGE_SHIFT);
                        pa += BASE_PAGE_SIZE as u64;
                    }
                    // Make the null page invalid to detect null pointer access.
                    ps.pt.0.entries[0].set_present(false);
                } else {
                    pde.set_present(true);
                    pde.set_writable(true);
                    pde.set_user(user);
                    pde.set_large(true);
                    pde.set_pfn(pa >> BASE_PAGE_SHIFT);
                    pa += LARGE_PAGE_SIZE as u64;
                }
            }
        }
    }

    pub fn split_apic_page(&mut self) {
        let apic_base_raw = unsafe { rdmsr(x86::msr::IA32_APIC_BASE) };
        let apic_base = apic_base_raw & !0xfff;
        let pdpt_index = apic_base.get_bits(30..=38) as usize; // [38:30]
        let pd_index = apic_base.get_bits(21..=29) as usize; // [29:21]
        let pde = &mut self.data.pd[pdpt_index].0.entries[pd_index];
        Self::split_2mb(pde, &mut self.data.pt_apic);
    }

    #[allow(dead_code)]
    pub fn pa(&self) -> u64 {
        physical_address(addr_of!(self.data) as _).as_u64()
    }
    pub fn build_identity(&mut self) {
        Self::build_identity_internal(&mut self.data, true);
    }

    fn split_2mb(pde: &mut Entry, pt: &mut Pt) {
        assert!(pde.present());
        assert!(pde.large());

        let writable = pde.writable();
        let user = pde.user();
        let mut pfn = pde.pfn();
        for pte in &mut pt.0.entries {
            assert!(!pte.present());
            pte.set_present(true);
            pte.set_writable(writable);
            pte.set_user(user);
            pte.set_large(false);
            pte.set_pfn(pfn);
            pfn += BASE_PAGE_SIZE as u64;
        }

        let pt_pa = physical_address(pt as *mut _ as _).as_u64();
        pde.set_pfn(pt_pa >> BASE_PAGE_SHIFT);
        pde.set_large(false);
    }
}