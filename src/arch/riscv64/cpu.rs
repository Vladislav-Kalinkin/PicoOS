use core::arch::asm;

pub const MSTATUS_MIE: u64 = 1 << 3;
pub const MSTATUS_MPIE: u64 = 1 << 7;
pub const MSTATUS_MPP: u64 = 0b11 << 11;
pub const MSTATUS_MPP_M: u64 = 0b11 << 11;
pub const MSTATUS_MPRV: u64 = 1 << 17;
pub const MSTATUS_FS: u64 = 0b11 << 13;

/// Run `f` with `mstatus.MIE` clear. Restores the previous MIE bit (does not
/// force MIE=1). Nestable: an inner call that starts with MIE already clear
/// leaves it clear.
pub fn without_interrupts<T>(f: impl FnOnce() -> T) -> T {
    let saved = mstatus();
    disable_machine_interrupts();
    let result = f();
    if saved & MSTATUS_MIE != 0 {
        enable_machine_interrupts();
    }
    result
}

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
pub fn set_mepc(value: u64) {
    unsafe {
        asm!(
            "csrw mepc, {0}",
            in(reg) value,
            options(nomem, nostack, preserves_flags)
        );
    }
}

#[inline(always)]
pub fn set_mstatus(value: u64) {
    unsafe {
        asm!(
            "csrw mstatus, {0}",
            in(reg) value,
            options(nomem, nostack, preserves_flags)
        );
    }
}

/// Worker `mret`: `MIE=0`, `MPIE=1`, `MPRV=0`, `MPP=M` (PR16). Never copy a
/// saved `mstatus` verbatim — that would re-enable IRQs on the trap stack.
pub fn synthesize_mstatus_for_mret_worker() -> u64 {
    let current = mstatus();
    let fs = current & MSTATUS_FS;
    (current & !(MSTATUS_MIE | MSTATUS_MPIE | MSTATUS_MPP | MSTATUS_MPRV))
        | MSTATUS_MPIE
        | MSTATUS_MPP_M
        | fs
}

/// Idle-exit: `MPP=M`, `MIE=1`, `MPIE=1`, `MPRV=0`. Used only when jumping to
/// `idle_loop` without `mret`.
pub fn synthesize_mstatus_for_idle() -> u64 {
    let current = mstatus();
    let fs = current & MSTATUS_FS;
    (current & !(MSTATUS_MIE | MSTATUS_MPIE | MSTATUS_MPP | MSTATUS_MPRV))
        | MSTATUS_MIE
        | MSTATUS_MPIE
        | MSTATUS_MPP_M
        | fs
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
pub fn set_mscratch(value: u64) {
    unsafe {
        asm!(
            "csrw mscratch, {0}",
            in(reg) value,
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

macro_rules! csr_rw {
    ($read:ident, $write:ident, $csr:literal) => {
        #[inline(always)]
        pub fn $read() -> u64 {
            let value: u64;
            unsafe {
                asm!(
                    concat!("csrr {0}, ", $csr),
                    out(reg) value,
                    options(nomem, nostack, preserves_flags)
                );
            }
            value
        }

        #[inline(always)]
        pub fn $write(value: u64) {
            unsafe {
                asm!(
                    concat!("csrw ", $csr, ", {0}"),
                    in(reg) value,
                    options(nomem, nostack, preserves_flags)
                );
            }
        }
    };
}

csr_rw!(pmpcfg0, set_pmpcfg0, "pmpcfg0");
csr_rw!(pmpcfg2, set_pmpcfg2, "pmpcfg2");
csr_rw!(pmpaddr0, set_pmpaddr0, "pmpaddr0");
csr_rw!(pmpaddr1, set_pmpaddr1, "pmpaddr1");
csr_rw!(pmpaddr2, set_pmpaddr2, "pmpaddr2");
csr_rw!(pmpaddr3, set_pmpaddr3, "pmpaddr3");
csr_rw!(pmpaddr4, set_pmpaddr4, "pmpaddr4");
csr_rw!(pmpaddr5, set_pmpaddr5, "pmpaddr5");
csr_rw!(pmpaddr6, set_pmpaddr6, "pmpaddr6");
csr_rw!(pmpaddr7, set_pmpaddr7, "pmpaddr7");
csr_rw!(pmpaddr8, set_pmpaddr8, "pmpaddr8");
csr_rw!(pmpaddr9, set_pmpaddr9, "pmpaddr9");
csr_rw!(pmpaddr10, set_pmpaddr10, "pmpaddr10");
csr_rw!(pmpaddr11, set_pmpaddr11, "pmpaddr11");
csr_rw!(pmpaddr12, set_pmpaddr12, "pmpaddr12");
csr_rw!(pmpaddr13, set_pmpaddr13, "pmpaddr13");
csr_rw!(pmpaddr14, set_pmpaddr14, "pmpaddr14");
csr_rw!(pmpaddr15, set_pmpaddr15, "pmpaddr15");

pub const PMP_ENTRIES: usize = 16;

pub fn pmpaddr(index: usize) -> u64 {
    match index {
        0 => pmpaddr0(),
        1 => pmpaddr1(),
        2 => pmpaddr2(),
        3 => pmpaddr3(),
        4 => pmpaddr4(),
        5 => pmpaddr5(),
        6 => pmpaddr6(),
        7 => pmpaddr7(),
        8 => pmpaddr8(),
        9 => pmpaddr9(),
        10 => pmpaddr10(),
        11 => pmpaddr11(),
        12 => pmpaddr12(),
        13 => pmpaddr13(),
        14 => pmpaddr14(),
        15 => pmpaddr15(),
        _ => 0,
    }
}

pub fn set_pmpaddr(index: usize, value: u64) {
    match index {
        0 => set_pmpaddr0(value),
        1 => set_pmpaddr1(value),
        2 => set_pmpaddr2(value),
        3 => set_pmpaddr3(value),
        4 => set_pmpaddr4(value),
        5 => set_pmpaddr5(value),
        6 => set_pmpaddr6(value),
        7 => set_pmpaddr7(value),
        8 => set_pmpaddr8(value),
        9 => set_pmpaddr9(value),
        10 => set_pmpaddr10(value),
        11 => set_pmpaddr11(value),
        12 => set_pmpaddr12(value),
        13 => set_pmpaddr13(value),
        14 => set_pmpaddr14(value),
        15 => set_pmpaddr15(value),
        _ => {}
    }
}
