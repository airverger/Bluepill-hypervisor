pub mod apic_id;
pub mod error;

use alloc::alloc::handle_alloc_error;
use alloc::boxed::Box;
use core::alloc::Layout;
use core::arch::{asm, global_asm};
use x86::bits64::paging::BASE_PAGE_SHIFT;
use x86::dtables::DescriptorTablePointer;
use kernelutils::Registers;

/// Returns the access rights of the given segment for SVM.
pub fn get_segment_access_right(table_base: u64, selector: u16) -> u16 {
    let descriptor_value = get_segment_descriptor_value(table_base, selector);

    // First, get the AVL, L, D/B and G bits, while excluding the "Seg. Limit 19:16"
    // bits. Then, get the Type, S, DPL and P bits. Finally, return those bits
    // without the "Seg. Limit 19:16" bits.
    // See: Figure 3-8. Segment Descriptor
    let ar = (descriptor_value >> 40) as u16;
    let upper_ar = (ar >> 4) & 0b1111_0000_0000;
    let lower_ar = ar & 0b1111_1111;
    lower_ar | upper_ar
}

/// Returns the segment descriptor casted as a 64bit integer for the given
/// selector.
pub fn get_segment_descriptor_value(table_base: u64, selector: u16) -> u64 {
    let sel = x86::segmentation::SegmentSelector::from_raw(selector);
    let descriptor_addr = table_base + u64::from(sel.index() * 8);
    let ptr = descriptor_addr as *const u64;
    unsafe { *ptr }
}

/// Returns the limit of the given segment.
pub fn get_segment_limit(table_base: u64, selector: u16) -> u32 {
    let sel = x86::segmentation::SegmentSelector::from_raw(selector);
    if sel.index() == 0 && (sel.bits() >> 2) == 0 {
        return 0; // unusable
    }
    let descriptor_value = get_segment_descriptor_value(table_base, selector);
    let limit_low = descriptor_value & 0xffff;
    let limit_high = (descriptor_value >> (32 + 16)) & 0xF;
    let mut limit = limit_low | (limit_high << 16);
    if ((descriptor_value >> (32 + 23)) & 0x01) != 0 {
        limit = ((limit + 1) << BASE_PAGE_SHIFT) - 1;
    }
    limit as u32
}
#[allow(dead_code)]
const SVM_DEBUG_CONTROL_0: u16 = 1 << 0;
#[allow(dead_code)]
const SVM_DEBUG_CONTROL_6: u16 = 1 << 6;
#[allow(dead_code)]

const SVM_DEBUG_CONTROL_7: u16 = 1 << 7;

#[allow(dead_code)]
pub const SVM_NP_ENABLE_NP_ENABLE: u64 = 1 << 0;
#[allow(dead_code)]
pub const SECURITY_EXCEPTION: u32 = 1 << 30;


#[allow(dead_code)]
#[repr(u32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum TlbControl {
    DoNotFlush = 0x0,
    FlushAll = 0x1,
    FlushGuests = 0x3,
    FlushGuestsNonGlobal = 0x7,
}
pub fn zeroed_box<T>() -> Box<T> {
    let layout = Layout::new::<T>();
    let ptr = unsafe { alloc::alloc::alloc_zeroed(layout) }.cast::<T>();
    if ptr.is_null() {
        handle_alloc_error(layout);
    }
    unsafe { Box::from_raw(ptr) }
}


extern "C" {
    /// Runs the guest until #VMEXIT occurs.
    pub fn run_svm_guest(registers: &mut Registers, vmcb_pa: u64, host_vmcb_pa: u64);
}

global_asm!(include_str!("run_guest.s"));

pub fn lidt(idtr: &DescriptorTablePointer<u64>) {
    unsafe { x86::dtables::lidt(idtr) };
}
pub fn vmsave(vmcb_pa: u64) {
    unsafe {
        asm!(
        "mov rax, {}",
        "vmsave rax",
        in(reg) vmcb_pa, options(nostack, preserves_flags),
        )
    };
}
pub fn sidt() -> DescriptorTablePointer<u64> {
    let mut idtr = DescriptorTablePointer::<u64>::default();
    unsafe { x86::dtables::sidt(&mut idtr) };
    idtr
}
#[allow(dead_code)]
pub(crate) fn sgdt() -> DescriptorTablePointer<u64> {
    let mut gdtr = DescriptorTablePointer::<u64>::default();
    unsafe { x86::dtables::sgdt(&mut gdtr) };
    gdtr
}

#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum GuestActivityState {
    Active = 0,
    WaitForSipi = u8::MAX,
}


