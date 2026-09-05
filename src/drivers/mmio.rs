mod imp {
    use core::arch::asm;

    /// # Safety
    /// `addr` must be a mapped MMIO register for this platform.
    #[inline(always)]
    pub unsafe fn read8(addr: usize) -> u8 {
        let value: u8;
        // SAFETY: Caller guarantees `addr` is a valid 8-bit MMIO register.
        unsafe {
            core::arch::asm!(
                "lbu {value}, 0({addr})",
                addr = in(reg) addr,
                value = out(reg) value,
                options(nostack, preserves_flags)
            );
        }
        value
    }

    /// # Safety
    /// `addr` must be a mapped MMIO register for this platform.
    #[inline(always)]
    pub unsafe fn write32(addr: usize, value: u32) {
        let value = value as usize;
        // SAFETY: Caller guarantees `addr` is a valid 32-bit MMIO register.
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
        // SAFETY: `addr` is a CLINT/platform 64-bit MMIO register.
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

        // SAFETY: `addr` is a CLINT/platform 64-bit MMIO register.
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
