mod guest;



pub use guest::Vmx;
use kernelutils::{jump_with_new_stack, Registers};


pub(crate) fn main(registers: &Registers) -> ! {

    unsafe { x86::irq::disable() };
    Vmx::enable();
    let mut vm = crate::amd::guest::vmx::Vmx::new(registers);
    // let id = apic_id::processor_id_from(apic_id::get()).unwrap();
    // let guest = &mut Arch::Guest::new(id);
    // guest.activate();
    // guest.initialize(registers);
    vm.activate();
    vm.save_host();
    vm.save_guest();
    loop {
        vm.run();

    }
}
pub  fn test_amd_vm(){

    let registers = Registers::capture_current();
    jump_with_new_stack(main,&registers);
}