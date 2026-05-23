use core::arch::asm;

#[inline(always)]
pub fn mhartid() -> u64 {
    let value: u64;

    unsafe {
        asm!(
            "csrr {0}, mhartid",
            out(reg) value,
            options(nomem, nostack, preserves_flags)
        );
    }

    value
}

#[inline(always)]
pub fn mstatus() -> u64 {
    let value: u64;

    unsafe {
        asm!(
            "csrr {0}, mstatus",
            out(reg) value,
            options(nomem, nostack, preserves_flags)
        );
    }

    value
}

#[inline(always)]
pub fn mtvec() -> u64 {
    let value: u64;

    unsafe {
        asm!(
            "csrr {0}, mtvec",
            out(reg) value,
            options(nomem, nostack, preserves_flags)
        );
    }

    value
}

#[inline(always)]
pub fn mepc() -> u64 {
    let value: u64;

    unsafe {
        asm!(
            "csrr {0}, mepc",
            out(reg) value,
            options(nomem, nostack, preserves_flags)
        );
    }

    value
}

#[inline(always)]
pub fn mcause() -> u64 {
    let value: u64;

    unsafe {
        asm!(
            "csrr {0}, mcause",
            out(reg) value,
            options(nomem, nostack, preserves_flags)
        );
    }

    value
}

#[inline(always)]
pub fn mie() -> u64 {
    let value: u64;

    unsafe {
        asm!(
            "csrr {0}, mie",
            out(reg) value,
            options(nomem, nostack, preserves_flags)
        );
    }

    value
}

#[inline(always)]
pub fn mip() -> u64 {
    let value: u64;

    unsafe {
        asm!(
            "csrr {0}, mip",
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
            "mv {0}, sp",
            out(reg) value,
            options(nomem, nostack, preserves_flags)
        );
    }

    value
}

#[inline(always)]
pub fn set_mtvec(addr: u64) {
    unsafe {
        asm!(
            "csrw mtvec, {0}",
            in(reg) addr,
            options(nomem, nostack, preserves_flags)
        );
    }
}

#[inline(always)]
pub fn mtval() -> u64 {
    let value: u64;

    unsafe {
        asm!(
            "csrr {0}, mtval",
            out(reg) value,
            options(nomem, nostack, preserves_flags)
        );
    }

    value
}

#[inline(always)]
pub fn enable_machine_interrupts() {
    unsafe {
        asm!("li t0, 0x8", "csrs mstatus, t0", options(nomem, nostack));
    }
}

#[inline(always)]
pub fn disable_machine_interrupts() {
    unsafe {
        asm!("li t0, 0x8", "csrc mstatus, t0", options(nomem, nostack));
    }
}

#[inline(always)]
pub fn enable_machine_timer_interrupt() {
    unsafe {
        asm!("li t0, 0x80", "csrs mie, t0", options(nomem, nostack));
    }
}

#[inline(always)]
pub fn disable_machine_timer_interrupt() {
    unsafe {
        asm!("li t0, 0x80", "csrc mie, t0", options(nomem, nostack));
    }
}
