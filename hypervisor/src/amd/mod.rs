mod guest;
pub mod vmexit;

use alloc::boxed::Box;
use core::arch::asm;
pub use guest::Vmx;
pub use vmexit::InstructionInfo;
pub use vmexit::VmExitReason;

use crate::amd::guest::apic_id;
use crate::amd::vmexit::handle_cpuid;
use crate::arch::Architecture;
use kernelutils::nt::{platform_ops, switch_stack};
use kernelutils::Registers;

pub(crate) fn main(registers: &Registers) -> ! {
    unsafe { x86::irq::disable() };

    let id = apic_id::processor_id_from(apic_id::get()).unwrap();
    Architecture::enable();
    let mut guest = Vmx::new(id);

    guest.activate();

    guest.initialize(registers);


    loop {
        let reason = guest.run();


        if let VmExitReason::Cpuid(info) = reason {
            handle_cpuid(&mut guest, &info);
        }
    }
}

pub fn virtualize_system() {
    platform_ops::init(Box::new(platform_ops::WindowsOps));

    apic_id::init();
    platform_ops::get().run_on_all_processors(|| {
        let registers = Registers::capture_current();

        log::info!("Virtualizing the current processor");

        unsafe { asm!("int 3") }
        switch_stack::jump_with_new_stack(main, &registers);
        #[allow(unreachable_code)]
        log::info!("Virtualized the current processor");
    });
}
