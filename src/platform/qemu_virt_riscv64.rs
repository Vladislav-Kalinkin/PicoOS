pub const UART0_BASE: usize = 0x1000_0000;

pub const CLINT_BASE: usize = 0x0200_0000;
pub const CLINT_MTIMECMP: usize = CLINT_BASE + 0x4000;
pub const CLINT_MTIME: usize = CLINT_BASE + 0xBFF8;

pub const TIMEBASE_FREQ: u64 = 10_000_000;

pub const RAM_START: usize = 0x8000_0000;
pub const RAM_SIZE: usize = 128 * 1024 * 1024;

#[allow(dead_code)]
pub const NAME: &str = "QEMU virt riscv64";
