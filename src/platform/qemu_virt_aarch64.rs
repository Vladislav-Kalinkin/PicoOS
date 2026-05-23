pub const UART0_BASE: usize = 0x0900_0000;

pub const GICD_BASE: usize = 0x0800_0000;
pub const GICC_BASE: usize = 0x0801_0000;

pub const IRQ_PHYSICAL_TIMER: u32 = 30;

pub const RAM_START: usize = 0x4000_0000;
pub const RAM_SIZE: usize = 128 * 1024 * 1024;

#[allow(dead_code)]
pub const NAME: &str = "QEMU virt aarch64";
