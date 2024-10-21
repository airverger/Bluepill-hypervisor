use alloc::boxed::Box;
use wdk_sys::{ALL_PROCESSOR_GROUPS, GROUP_AFFINITY, NT_SUCCESS, PROCESSOR_NUMBER};
use wdk_sys::ntddk::{KeGetProcessorNumberFromIndex, KeQueryActiveProcessorCountEx, KeRevertToUserGroupAffinityThread, KeSetSystemGroupAffinityThread, MmGetPhysicalAddress};

pub struct WindowsOps;
pub trait PlatformOps {
    /// Runs `callback` on all logical processors one by one.
    // This function cannot be called in a nested manner.
    fn run_on_all_processors(&self, callback: fn());

    // Returns a physical address of a linear address specified by `va`.
    fn pa(&self, va: *const core::ffi::c_void) -> u64;
}

impl PlatformOps for WindowsOps {
    fn run_on_all_processors(&self, callback: fn()) {
        fn processor_count() -> u32 {
            unsafe { KeQueryActiveProcessorCountEx(u16::try_from(ALL_PROCESSOR_GROUPS).unwrap()) }
        }

        // PAGED_CODE!();

        for index in 0..processor_count() {
            let mut processor_number = PROCESSOR_NUMBER::default();
            let status = unsafe { KeGetProcessorNumberFromIndex(index, &mut processor_number) };
            assert!(NT_SUCCESS(status));

            let mut old_affinity = GROUP_AFFINITY::default();
            let mut affinity = GROUP_AFFINITY {
                Group: processor_number.Group,
                Mask: 1 << processor_number.Number,
                Reserved: [0, 0, 0],
            };
            unsafe { KeSetSystemGroupAffinityThread(&mut affinity, &mut old_affinity) };

            callback();

            unsafe { KeRevertToUserGroupAffinityThread(&mut old_affinity) };
        }
    }

    fn pa(&self, va: *const core::ffi::c_void) -> u64 {
        #[allow(clippy::cast_sign_loss)]
        unsafe {
            MmGetPhysicalAddress(va.cast_mut()).QuadPart as u64
        }
    }
}
pub fn init(ops: Box<dyn PlatformOps>) {
    unsafe { PLATFORM_OPS = Some(Box::leak(ops)) };
}

/// Returns the platform specific API.
pub fn get() -> &'static dyn PlatformOps {
    *unsafe { PLATFORM_OPS.as_ref() }.unwrap()
}

static mut PLATFORM_OPS: Option<&dyn PlatformOps> = None;
