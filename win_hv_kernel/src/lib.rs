#![no_std]

use core::arch::asm;
use kernel_log::KernelLogger;
use log::LevelFilter;
use wdk_sys::{DRIVER_OBJECT, NTSTATUS, PUNICODE_STRING};

extern crate alloc;
#[cfg(not(test))]
extern crate wdk_panic;
#[cfg(not(test))]

use wdk_alloc::WDKAllocator;
use kernelutils::spoof_test;

#[cfg(not(test))]
#[global_allocator]
static GLOBAL_ALLOCATOR: WDKAllocator = WDKAllocator;
// type Ptra = unsafe extern "C" fn(DriverObject: *mut DRIVER_OBJECT) ;

#[export_name = "DriverEntry"]
pub unsafe extern "system" fn driver_entry(
    driver: &mut DRIVER_OBJECT,
    _registry_path: PUNICODE_STRING,
) -> NTSTATUS {



    driver.DriverUnload = Some(driver_unload);
    KernelLogger::init(LevelFilter::Trace).expect("Failed to initialize logger");

    // spoof_test();

    log::trace!("com_logger Hello Gorgon");

    hypervisor::amd::virtualize_system();
    0 as NTSTATUS
}



pub extern "C" fn driver_unload(_driver: *mut DRIVER_OBJECT) {
    log::trace!("Driver unloaded successfully!");
}
