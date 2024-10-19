use alloc::alloc::handle_alloc_error;
use core::alloc::{AllocError, Allocator, GlobalAlloc, Layout};
use core::ptr::NonNull;
use wdk_sys::_POOL_TYPE::NonPagedPool;
use wdk_sys::ntddk::{ExAllocatePool, ExFreePool};

pub struct KernelAlloc;

unsafe impl Allocator for KernelAlloc {
    /// Allocates a block of kernel memory.
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
        let memory = unsafe { ExAllocatePool(NonPagedPool, layout.size() as _) } as *mut u8;

        if memory.is_null() {
            Err(AllocError)
        } else {
            let slice = unsafe { core::slice::from_raw_parts_mut(memory, layout.size()) };
            Ok(unsafe { NonNull::new_unchecked(slice) })
        }
    }

    /// Frees an allocated block of kernel memory.
    ///
    /// # Parameters
    ///
    /// * `ptr` - Non-null pointer to the memory to be released.
    /// * `_layout` - Memory layout (not used in this implementation).
    unsafe fn deallocate(&self, ptr: NonNull<u8>, _layout: Layout) {
        ExFreePool(ptr.as_ptr() as _);
    }
}

/// Global allocator using the `KernelAlloc` mechanism.
///
/// This implementation allows `KernelAlloc` to be used as the global allocator,
/// thereby providing memory allocation capabilities for the entire kernel space.
/// It interfaces directly with the WDK's `ExAllocatePool` and `ExFreePool` functions.
unsafe impl GlobalAlloc for KernelAlloc {
    /// Allocates a block of memory in the kernel space.
    ///
    /// This function leverages the `ExAllocatePool` function from the WDK to
    /// provide memory allocation capabilities.
    ///
    /// # Parameters
    ///
    /// * `layout` - Memory layout specifications.
    ///
    /// # Returns
    ///
    /// A raw pointer to the allocated block of memory.
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let memory = unsafe { ExAllocatePool(NonPagedPool, layout.size() as _) } as *mut u8;

        if memory.is_null() {
            handle_alloc_error(layout);
        }

        memory as _
    }

    /// Frees a previously allocated block of memory in the kernel space.
    ///
    /// This function leverages the `ExFreePool` function from the WDK to
    /// release the memory back to the system.
    ///
    /// # Parameters
    ///
    /// * `ptr` - Raw pointer to the memory block to be released.
    /// * `_layout` - Memory layout specifications (not used in this implementation).
    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        ExFreePool(ptr as _);
    }
}
