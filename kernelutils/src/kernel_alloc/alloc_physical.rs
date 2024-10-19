use core::alloc::{AllocError, Allocator, Layout};
use core::ptr::NonNull;
use wdk_sys::_MEMORY_CACHING_TYPE::MmCached;
use wdk_sys::ntddk::{MmAllocateContiguousMemorySpecifyCacheNode, MmFreeContiguousMemory};
use wdk_sys::{MM_ANY_NODE_OK, PHYSICAL_ADDRESS};

/// Physical memory allocator for kernel space.
///
/// Leverages `MmAllocateContiguousMemorySpecifyCacheNode` from the WDK to
/// allocate memory that is physically contiguous.
pub struct PhysicalAllocator;

unsafe impl Allocator for PhysicalAllocator {
    /// Allocates a contiguous block of physical memory.
    ///
    /// # Parameters
    ///
    /// * `layout` - Memory layout specifications.
    ///
    /// # Returns
    ///
    /// A result containing a non-null pointer to the memory block if successful.
    /// Returns an `AllocError` if the allocation fails.
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        let mut boundary: PHYSICAL_ADDRESS = unsafe { core::mem::zeroed() };
        let mut lowest: PHYSICAL_ADDRESS = unsafe { core::mem::zeroed() };
        let mut highest: PHYSICAL_ADDRESS = unsafe { core::mem::zeroed() };

        boundary.QuadPart = 0;
        lowest.QuadPart = 0;
        highest.QuadPart = -1;

        let memory = unsafe {
            MmAllocateContiguousMemorySpecifyCacheNode(
                layout.size() as _,
                lowest,
                highest,
                boundary,
                MmCached,
                MM_ANY_NODE_OK,
            )
        } as *mut u8;

        if memory.is_null() {
            Err(AllocError)
        } else {
            let slice = unsafe { core::slice::from_raw_parts_mut(memory, layout.size()) };
            Ok(unsafe { NonNull::new_unchecked(slice) })
        }
    }

    /// Frees an allocated block of physical memory.
    ///
    /// # Parameters
    ///
    /// * `ptr` - Non-null pointer to the memory to be released.
    /// * `_layout` - Memory layout (not used in this implementation).
    unsafe fn deallocate(&self, ptr: NonNull<u8>, _layout: Layout) {
        MmFreeContiguousMemory(ptr.as_ptr() as _);
    }
}
