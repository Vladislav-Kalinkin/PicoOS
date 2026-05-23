#[cfg(target_arch = "aarch64")]
mod imp {
    use core::arch::asm;

    #[inline(always)]
    pub fn write32(addr: usize, value: u32) {
        unsafe {
            asm!(
                "str w1, [x0]",
                in("x0") addr,
                in("w1") value,
                options(nostack, preserves_flags)
            );
        }
    }

    #[allow(dead_code)]
    #[inline(always)]
    pub fn read32(addr: usize) -> u32 {
        let value: u32;

        unsafe {
            asm!(
                "ldr w0, [x1]",
                out("w0") value,
                in("x1") addr,
                options(nostack, preserves_flags)
            );
        }

        value
    }

    #[allow(dead_code)]
    #[inline(always)]
    pub fn write64(addr: usize, value: u64) {
        unsafe {
            asm!(
                "str x1, [x0]",
                in("x0") addr,
                in("x1") value,
                options(nostack, preserves_flags)
            );
        }
    }

    #[allow(dead_code)]
    #[inline(always)]
    pub fn read64(addr: usize) -> u64 {
        let value: u64;

        unsafe {
            asm!(
                "ldr x0, [x1]",
                out("x0") value,
                in("x1") addr,
                options(nostack, preserves_flags)
            );
        }

        value
    }

    #[inline(always)]
    pub fn write8(addr: usize, value: u8) {
        unsafe {
            asm!(
                "strb w1, [x0]",
                in("x0") addr,
                in("w1") value as u32,
                options(nostack, preserves_flags)
            );
        }
    }
}

#[cfg(target_arch = "riscv64")]
mod imp {
    use core::arch::asm;

    #[inline(always)]
    pub fn write32(addr: usize, value: u32) {
        unsafe {
            asm!(
                "sw {value}, 0({addr})",
                addr = in(reg) addr,
                value = in(reg) value,
                options(nostack)
            );
        }
    }

    #[allow(dead_code)]
    #[inline(always)]
    pub fn read32(addr: usize) -> u32 {
        let value: u32;

        unsafe {
            asm!(
                "lw {value}, 0({addr})",
                addr = in(reg) addr,
                value = out(reg) value,
                options(nostack)
            );
        }

        value
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

    #[allow(dead_code)]
    #[inline(always)]
    pub fn write8(addr: usize, value: u8) {
        unsafe {
            asm!(
                "sb {value}, 0({addr})",
                addr = in(reg) addr,
                value = in(reg) value,
                options(nostack)
            );
        }
    }
}

pub use imp::*;
