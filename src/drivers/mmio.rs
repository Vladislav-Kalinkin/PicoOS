mod imp {
    use core::arch::asm;

    #[inline(always)]
    pub unsafe fn write32(addr: usize, value: u32) {
        let value = value as usize;
        core::arch::asm!(
            "sw {value}, 0({addr})",
            addr = in(reg) addr,
            value = in(reg) value,
            options(nostack, preserves_flags)
        );
    }

    #[allow(dead_code)]
    #[inline(always)]
    pub unsafe fn read32(addr: usize) -> u32 {
        let value: usize;

        core::arch::asm!(
            "lwu {value}, 0({addr})",
            addr = in(reg) addr,
            value = out(reg) value,
            options(nostack, preserves_flags)
        );

        value as u32
    }

    #[allow(dead_code)]
    #[inline(always)]
    pub unsafe fn write8(addr: usize, value: u8) {
        let value = value as usize;

        core::arch::asm!(
            "sb {value}, 0({addr})",
            addr = in(reg) addr,
            value = in(reg) value,
            options(nostack, preserves_flags)
        );
    }

    #[allow(dead_code)]
    #[inline(always)]
    pub unsafe fn read8(addr: usize) -> u8 {
        let value: usize;

        core::arch::asm!(
            "lbu {value}, 0({addr})",
            addr = in(reg) addr,
            value = out(reg) value,
            options(nostack, preserves_flags)
        );

        value as u8
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
