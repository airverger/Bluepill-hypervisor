mod vmcb_wrapper;
mod page_wrapper;
mod exception_wrapper;

pub use vmcb_wrapper::VmcbRaw;
pub use vmcb_wrapper::ControlArea;
pub use vmcb_wrapper::StateSaveArea;
pub use vmcb_wrapper::HostStateAreaRaw;
pub use vmcb_wrapper::SegmentDescriptorRaw;
pub use vmcb_wrapper::GdtTssRaw;
pub use vmcb_wrapper::Gdtr;

pub use page_wrapper::Pml4;
pub use page_wrapper::Pdpt;
pub use page_wrapper::Pd;
pub use page_wrapper::Pt;
pub use page_wrapper::Table;
pub use page_wrapper::Entry;
pub use page_wrapper::PagingStructuresRaw;


pub use exception_wrapper::HostExceptionStack;
pub use exception_wrapper::handle_host_exception;
pub use exception_wrapper::asm_interrupt_handler0;
pub use exception_wrapper::InterruptDescriptorTableRaw;
pub use exception_wrapper::InterruptDescriptorTableEntry;




