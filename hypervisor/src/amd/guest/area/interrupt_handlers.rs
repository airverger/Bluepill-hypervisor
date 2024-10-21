//! This module implements initialization of the host IDT and host interrupt handlers.

use alloc::boxed::Box;
use x86::{dtables::DescriptorTablePointer, segmentation::SegmentSelector};
use kernelutils::PhysicalAllocator;
use crate::amd::guest::{asm_interrupt_handler0, InterruptDescriptorTableEntry, InterruptDescriptorTableRaw};

/// Logical representation of the IDT.
#[derive(Debug, derive_deref::Deref, derive_deref::DerefMut)]
pub struct InterruptDescriptorTable {
    data: Box<InterruptDescriptorTableRaw, PhysicalAllocator>,
}

impl InterruptDescriptorTable {
    #[allow(dead_code)]
    pub fn new(cs: SegmentSelector) -> Self {
        // Build the IDT. Each interrupt handler (ie. asm_interrupt_handlerN) is
        // 16 byte long and can be located from asm_interrupt_handler0.
        let mut idt: Box<InterruptDescriptorTableRaw, PhysicalAllocator> = unsafe { Box::new_zeroed_in(PhysicalAllocator).assume_init() };
        for i in 0..idt.0.len() {
            let handler = asm_interrupt_handler0 as usize + 0x10 * i;
            idt.0[i] = InterruptDescriptorTableEntry::new(handler, cs);
        }

        Self { data: idt }
    }

    pub(crate) fn idtr(&self) -> DescriptorTablePointer<u64> {
        let mut idtr = DescriptorTablePointer::<u64>::default();
        let base = self.data.as_ref() as *const _;
        idtr.base = base as _;
        idtr.limit = u16::try_from(core::mem::size_of_val(self.data.as_ref()) - 1).unwrap();
        idtr
    }
}

