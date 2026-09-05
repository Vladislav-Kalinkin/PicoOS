use core::arch::asm;

pub const MSTATUS_MIE: u64 = 1 << 3;
pub const MSTATUS_MPIE: u64 = 1 << 7;
pub const MSTATUS_MPP: u64 = 0b11 << 11;
pub const MSTATUS_MPP_M: u64 = 0b11 << 11;
pub const MSTATUS_MPP_U: u64 = 0;
pub const MSTATUS_MPRV: u64 = 1 << 17;
pub const MSTATUS_FS: u64 = 0b11 << 13;
pub const MIE_MTIE: u64 = 1 << 7;

macro_rules! csr_read {
    ($name:ident, $csr:literal) => {
        #[inline(always)]
        pub fn $name() -> u64 {
            let value: u64;
            // SAFETY: M-mode `csrr`; `$csr` is a valid CSR encoded in the instruction.
            unsafe {
                asm!(
                    concat!("csrr {0}, ", $csr),
                    out(reg) value,
                    options(nomem, nostack, preserves_flags)
                );
            }
            value
        }
    };
}

macro_rules! csr_write {
    ($name:ident, $csr:literal) => {
        #[inline(always)]
        pub fn $name(value: u64) {
            // SAFETY: M-mode `csrw`; `$csr` is a valid CSR encoded in the instruction.
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

macro_rules! csr_rw {
    ($read:ident, $write:ident, $csr:literal) => {
        csr_read!($read, $csr);
        csr_write!($write, $csr);
    };
}

csr_read!(mhartid, "mhartid");
csr_rw!(mstatus, set_mstatus, "mstatus");
csr_rw!(mtvec, set_mtvec, "mtvec");
csr_rw!(mepc, set_mepc, "mepc");
csr_read!(mcause, "mcause");
csr_read!(mie, "mie");
csr_read!(mip, "mip");
csr_read!(mtval, "mtval");
csr_write!(set_mscratch, "mscratch");

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

fn worker_mpp() -> u64 {
    MSTATUS_MPP_U
}

/// Worker `mret`: `MIE=0`, `MPIE=1`, `MPRV=0`, `MPP=U`. Never copy saved `mstatus`.
pub fn synthesize_mstatus_for_mret_worker() -> u64 {
    let current = mstatus();
    let fs = current & MSTATUS_FS;
    (current & !(MSTATUS_MIE | MSTATUS_MPIE | MSTATUS_MPP | MSTATUS_MPRV))
        | MSTATUS_MPIE
        | worker_mpp()
        | fs
}

pub fn trapped_from_user() -> bool {
    (mstatus() & MSTATUS_MPP) == MSTATUS_MPP_U
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
pub fn stack_pointer() -> u64 {
    let value: u64;
    // SAFETY: Reads `sp`; does not change memory or control state.
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
pub fn enable_machine_interrupts() {
    // SAFETY: M-mode `csrs mstatus.MIE`.
    unsafe {
        asm!("csrs mstatus, {0}", in(reg) MSTATUS_MIE, options(nomem, nostack));
    }
}

#[inline(always)]
pub fn disable_machine_interrupts() {
    // SAFETY: M-mode `csrc mstatus.MIE`.
    unsafe {
        asm!("csrc mstatus, {0}", in(reg) MSTATUS_MIE, options(nomem, nostack));
    }
}

#[inline(always)]
pub fn enable_machine_timer_interrupt() {
    // SAFETY: M-mode `csrs mie.MTIE`.
    unsafe {
        asm!("csrs mie, {0}", in(reg) MIE_MTIE, options(nomem, nostack));
    }
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
