pub const UART0_BASE: usize = 0x1000_0000;
/// NS16550 Line Status Register (byte offset 5).
pub const UART0_LSR: usize = UART0_BASE + 5;
/// Transmit Holding Register Empty.
pub const UART_LSR_THRE: u8 = 1 << 5;

pub const CLINT_BASE: usize = 0x0200_0000;
pub const CLINT_MTIME: usize = CLINT_BASE + 0xBFF8;

pub const fn clint_msip(hart: usize) -> usize {
    CLINT_BASE + 4 * hart
}

pub const fn clint_mtimecmp(hart: usize) -> usize {
    CLINT_BASE + 0x4000 + 8 * hart
}

pub const TIMEBASE_FREQ: u64 = 10_000_000;

pub const RAM_START: usize = 0x8000_0000;
pub const RAM_SIZE: usize = 128 * 1024 * 1024;

/// QEMU virt topology bound (like `PMP_ENTRIES`), not a software `MAX_TASKS`.
/// `mhartid >= HART_SLOTS` parks forever in `boot.S` and must not index tables.
pub const HART_SLOTS: usize = 8;

pub const NAME: &str = "QEMU virt riscv64";
