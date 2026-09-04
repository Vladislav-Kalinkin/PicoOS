mod imp {
    use core::arch::asm;

    #[inline(always)]
    pub unsafe fn write32(addr: usize, value: u32) {
        let value = value as usize;
        unsafe {
            core::arch::asm!(
                "sw {value}, 0({addr})",
                addr = in(reg) addr,
                value = in(reg) value,
                options(nostack, preserves_flags)
            );
        }
    }

    #[inline(always)]
    pub fn write64(addr: usize, value: u64) {
        unsafe {
            asm!(
                "sd {value}, 0({addr})",
                addr = in(reg) addr,
                value = in(reg) value,
                options(nostack)
            );
        }
    }

    #[inline(always)]
    pub fn read64(addr: usize) -> u64 {
        let value: u64;

        unsafe {
            asm!(
                "ld {value}, 0({addr})",
                addr = in(reg) addr,
                value = out(reg) value,
                options(nostack)
            );
        }

        value
    }
}

pub use imp::*;
