mod addresses;
pub mod undocumented;
pub mod platform_ops;
pub mod switch_stack;



use core::alloc::Layout;
use core::arch::global_asm;
pub use addresses::PhysicalAddress;
pub use addresses::physical_address;
use wdk_sys::ntddk::{KeGetProcessorNumberFromIndex, KeLowerIrql, KeQueryActiveProcessorCountEx, KeRevertToUserGroupAffinityThread, KeSetSystemGroupAffinityThread, KeStackAttachProcess, KeUnstackDetachProcess, MmGetPhysicalAddress, MmGetSystemRoutineAddress};
use wdk_sys::{ALL_PROCESSOR_GROUPS, GROUP_AFFINITY, KIRQL, NT_SUCCESS, PAGED_CODE, PEPROCESS, PRKPROCESS, PROCESSOR_NUMBER, PVOID, _KAPC_STATE};
use x86::bits64::paging::BASE_PAGE_SIZE;
use crate::{misc, HypervisorError, Registers};

/// Gets a pointer to a function from ntoskrnl.exe exports.
///
/// # Arguments
/// * `function_name` - The name of the function to retrieve.
///
/// # Returns
/// A pointer to the requested function, or null if not found.
pub fn get_ntoskrnl_export(function_name: &str) -> PVOID {
    let unicode_string = misc::str_to_unicode(function_name);
    let routine_address =
        unsafe { MmGetSystemRoutineAddress(&unicode_string as *const _ as *mut _) };
    routine_address
}
pub fn raise_irql_to_dpc_level() -> Result<KIRQL, HypervisorError> {
    type FnKeRaiseIrqlToDpcLevel = unsafe extern "system" fn() -> KIRQL;

    // Get the address of the function from ntoskrnl
    let routine_address = get_ntoskrnl_export("KeRaiseIrqlToDpcLevel");

    // Ensure that the address is valid
    let p_ke_raise_irql_to_dpc_level = if !routine_address.is_null() {
        unsafe { core::mem::transmute::<PVOID, FnKeRaiseIrqlToDpcLevel>(routine_address) }
    } else {
        return Err(HypervisorError::KeRaiseIrqlToDpcLevelNull);
    };

    // Invoke the retrieved function
    Ok(unsafe { p_ke_raise_irql_to_dpc_level() })
}
pub fn lower_irql_to_old_level(old_irql: KIRQL) {
    // Directly manipulating the IRQL is an unsafe operation
    unsafe { KeLowerIrql(old_irql) };
}
/// Represents the CR3 (Directory Table Base) of the system process.
///
/// This is typically used to store the page table root physical address
/// of the system process for use in virtual-to-physical address translation.
pub static mut NTOSKRNL_CR3: u64 = 0;

/// Updates the `NTOSKRNL_CR3` static with the CR3 of the system process.
///
/// Retrieves the Directory Table Base (DirBase) of the system process,
/// typically corresponding to the NT kernel (`ntoskrnl`).
///
/// # Credits
///
/// Credits to @Drew from https://github.com/drew-gpf for the help.
pub fn update_ntoskrnl_cr3() {
    // Default initialization of APC state.
    let mut apc_state = _KAPC_STATE::default();

    // Attach to the system process's stack safely.
    // `KeStackAttachProcess` is unsafe as it manipulates thread execution context.
    unsafe { KeStackAttachProcess(PsInitialSystemProcess as PRKPROCESS, &mut apc_state) };

    // Update the NTOSKRNL_CR3 static with the current CR3 value.
    // Accessing CR3 is an unsafe operation as it involves reading a control register.
    unsafe {
        NTOSKRNL_CR3 = x86::controlregs::cr3();
    }

    log::trace!("NTOSKRNL_CR3: {:#x}", unsafe { NTOSKRNL_CR3 });

    // Detach from the system process's stack safely.
    // `KeUnstackDetachProcess` is unsafe as it restores the previous thread execution context.
    unsafe { KeUnstackDetachProcess(&mut apc_state) };
}
#[link(name = "ntoskrnl")]
extern "C" {
    pub static mut PsInitialSystemProcess: PEPROCESS;
}

#[link(name = "ntoskrnl")]
extern "system" {
    /// The RtlCopyMemory routine copies the contents of a source memory block to a destination memory block.
    /// Callers of RtlCopyMemory can be running at any IRQL if the source and destination memory blocks are in nonpaged system memory.
    /// Otherwise, the caller must be running at IRQL <= APC_LEVEL.
    /// https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/wdm/nf-wdm-rtlcopymemory
    pub fn RtlCopyMemory(destination: *mut u64, source: *mut u64, length: usize);
}
extern "C" {
    /// Jumps to the landing code with the new stack pointer.
    fn switch_stack(registers: &Registers, destination: usize, stack_base: u64) -> !;
}

