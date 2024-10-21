use x86::msr::{rdmsr, wrmsr};

pub struct Architecture;

impl Architecture {
    #[allow(dead_code)]
    fn enable_intel() {}
    pub fn enable() {
        if x86::cpuid::CpuId::new().get_vendor_info().unwrap().as_str() == "GenuineIntel" {
        } else {
            Self::enable_amd();
        }
    }

    fn enable_amd() {
        const EFER_SVME: u64 = 1 << 12;

        // Enable SVM. We assume the processor is compatible with this.
        // See: 15.4 Enabling SVM
        unsafe {
            wrmsr(x86::msr::IA32_EFER, rdmsr(x86::msr::IA32_EFER) | EFER_SVME);
        }
    }
}
