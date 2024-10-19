#![no_std]
#![feature(allocator_api)]
#![feature(const_trait_impl)]
#![feature(naked_functions)]
#![feature(once_cell_try)]
#![feature(decl_macro)]
extern crate alloc;

mod kernel_alloc;
pub mod nt;
mod misc;
mod hard;

pub use kernel_alloc::PhysicalAllocator;
pub use kernel_alloc::KernelAlloc;

pub use crate::misc::OwnedUnicodeString;
pub use crate::misc::str_to_unicode;
pub use crate::misc::HypervisorError;
pub use hard::CPUVersion;
pub use hard::get_cpu_version;
pub use hard::Registers;
pub use hard::Xmm;
pub use hard::capture_registers;

pub use nt::PhysicalAddress;
pub use nt::physical_address;
pub use nt::jump_with_new_stack;

// get_segment_limit
// get_segment_descriptor_value
// get_segment_access_right