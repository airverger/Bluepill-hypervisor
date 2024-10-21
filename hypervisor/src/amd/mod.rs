mod guest;
pub use guest::VCpu;

use alloc::boxed::Box;
use core::arch::asm;

pub use guest::vmexit::InstructionInfo;
pub use guest::vmexit::VmExitReason;


use guest::vmexit::handle_cpuid;
use crate::arch::Architecture;
use kernelutils::nt::{platform_ops, switch_stack};
use kernelutils::Registers;
use crate::amd::guest::support::apic_id;

pub(crate) fn main(registers: &Registers) -> ! {
    unsafe { x86::irq::disable() };

    let id = apic_id::processor_id_from(apic_id::get()).unwrap();
    Architecture::enable();
    let mut guest = VCpu::new(id);

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
        #[allow(dead_code)]
        log::info!("Virtualized the current processor");
    });
}
