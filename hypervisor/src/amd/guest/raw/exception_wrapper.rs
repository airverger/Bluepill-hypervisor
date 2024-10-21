use core::arch::global_asm;
use x86::bits64::rflags::RFlags;
use x86::controlregs::cr2;
use x86::segmentation::SegmentSelector;

/// The layout of the stack passed to [`handle_host_exception`].
#[derive(Debug)]
#[repr(C)]
pub struct HostExceptionStack {
    r15: u64,
    r14: u64,
    r13: u64,
    r12: u64,
    r11: u64,
    r10: u64,
    r9: u64,
    r8: u64,
    rdi: u64,
    rsi: u64,
    rbp: u64,
    rbx: u64,
    rdx: u64,
    rcx: u64,
    rax: u64,
    exception_number: u64, // Software saved (see interrupt_handler.S)
    error_code: u64,       // Software or hardware saved (see interrupt_handler.S)
    rip: u64,              // Hardware saved
    cs: u64,               // Hardware saved
    rflags: RFlags,        // Hardware saved
    rsp: u64,              // Hardware saved
    ss: u64,               // Hardware saved
}

/// The host interrupt handler.
#[no_mangle]
pub extern "C" fn handle_host_exception(stack: *mut HostExceptionStack) {
    assert!(!stack.is_null());
    let stack = unsafe { &*stack };
    unsafe {
        panic!(
            "Exception {} occurred in host: {stack:#x?}, cr2: {:#x?}",
            stack.exception_number,
            cr2(),
        );
    }
}

global_asm!(include_str!("interrupt_handlers.S"));
extern "C" {
    pub fn asm_interrupt_handler0();
}

#[derive(Debug)]
#[repr(C, align(4096))]
pub struct InterruptDescriptorTableRaw(pub [InterruptDescriptorTableEntry; 0x100]);
const _: () = assert!(core::mem::size_of::<InterruptDescriptorTableRaw>() == 4096);

#[derive(Debug)]
#[repr(C, align(16))]
pub struct InterruptDescriptorTableEntry {
    offset_low: u16,
    selector: u16,
    reserved_1: u8,
    gate_type: u8,
    offset_high: u16,
    offset_upper: u32,
    reserved_2: u32,
}
const _: () = assert!(core::mem::size_of::<InterruptDescriptorTableEntry>() == 16);

impl InterruptDescriptorTableEntry {
    pub fn new(handler: usize, cs: SegmentSelector) -> Self {
        // P=1, DPL=00b, S=0, type=1110b => type_attr=1000_1110b => 0x8E
        const INTERRUPT_GATE: u8 = 0x8E;
        Self {
            offset_low: handler as _,
            selector: cs.bits(),
            reserved_1: 0,
            gate_type: INTERRUPT_GATE,
            offset_high: (handler >> 16) as _,
            offset_upper: (handler >> 32) as _,
            reserved_2: 0,
        }
    }
}
