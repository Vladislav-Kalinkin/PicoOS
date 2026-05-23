use core::arch::asm;

#[inline(always)]
pub fn current_el_raw() -> u64 {
    let value: u64;

    unsafe {
        asm!(
            "mrs {0}, CurrentEL",
            out(reg) value,
            options(nomem, nostack, preserves_flags)
        );
    }

    value
}

#[inline(always)]
pub fn current_el() -> u64 {
    (current_el_raw() >> 2) & 0b11
}

#[inline(always)]
pub fn sctlr_el1() -> u64 {
    let value: u64;

    unsafe {
        asm!(
            "mrs {0}, SCTLR_EL1",
            out(reg) value,
            options(nomem, nostack, preserves_flags)
        );
    }

    value
}

#[inline(always)]
pub fn daif() -> u64 {
    let value: u64;

    unsafe {
        asm!(
            "mrs {0}, DAIF",
            out(reg) value,
            options(nomem, nostack, preserves_flags)
        );
    }

    value
}

#[inline(always)]
pub fn stack_pointer() -> u64 {
    let value: u64;

    unsafe {
        asm!(
            "mov {0}, sp",
            out(reg) value,
            options(nomem, nostack, preserves_flags)
        );
    }

    value
}

#[inline(always)]
pub fn vbar_el1() -> u64 {
    let value: u64;

    unsafe {
        asm!(
            "mrs {0}, VBAR_EL1",
            out(reg) value,
            options(nomem, nostack, preserves_flags)
        );
    }

    value
}

#[inline(always)]
pub fn set_vbar_el1(addr: u64) {
    unsafe {
        asm!(
            "msr VBAR_EL1, {0}",
            in(reg) addr,
            options(nomem, nostack, preserves_flags)
        );

        asm!("isb", options(nomem, nostack, preserves_flags));
    }
}

#[inline(always)]
pub fn esr_el1() -> u64 {
    let value: u64;

    unsafe {
        asm!(
            "mrs {0}, ESR_EL1",
            out(reg) value,
            options(nomem, nostack, preserves_flags)
        );
    }

    value
}

#[inline(always)]
pub fn elr_el1() -> u64 {
    let value: u64;

    unsafe {
        asm!(
            "mrs {0}, ELR_EL1",
            out(reg) value,
            options(nomem, nostack, preserves_flags)
        );
    }

    value
}

#[inline(always)]
pub fn far_el1() -> u64 {
    let value: u64;

    unsafe {
        asm!(
            "mrs {0}, FAR_EL1",
            out(reg) value,
            options(nomem, nostack, preserves_flags)
        );
    }

    value
}

#[inline(always)]
pub fn spsr_el1() -> u64 {
    let value: u64;

    unsafe {
        asm!(
            "mrs {0}, SPSR_EL1",
            out(reg) value,
            options(nomem, nostack, preserves_flags)
        );
    }

    value
}

#[inline(always)]
pub fn enable_irq() {
    unsafe {
        asm!("msr daifclr, #2", options(nomem, nostack, preserves_flags));
    }
}

#[inline(always)]
pub fn disable_irq() {
    unsafe {
        asm!("msr daifset, #2", options(nomem, nostack, preserves_flags));
    }
}

pub fn enable_fp_simd() {
    let mut cpacr_el1: u64;

    unsafe {
        core::arch::asm!(
            "mrs {cpacr_el1}, CPACR_EL1",
            cpacr_el1 = out(reg) cpacr_el1,
        );

        // FPEN bits [21:20] = 0b11: allow FP/SIMD access at EL0/EL1.
        cpacr_el1 |= 0b11 << 20;

        core::arch::asm!(
            "msr CPACR_EL1, {cpacr_el1}",
            "isb",
            cpacr_el1 = in(reg) cpacr_el1,
        );
    }
}
