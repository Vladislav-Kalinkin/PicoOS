use crate::drivers::mmio;
use crate::drivers::uart;
use crate::platform;

const GICD_CTLR: usize = platform::GICD_BASE;
const GICD_ISENABLER0: usize = platform::GICD_BASE + 0x100;
const GICD_IPRIORITYR: usize = platform::GICD_BASE + 0x400;

const GICC_CTLR: usize = platform::GICC_BASE;
const GICC_PMR: usize = platform::GICC_BASE + 0x0004;
const GICC_IAR: usize = platform::GICC_BASE + 0x000C;
const GICC_EOIR: usize = platform::GICC_BASE + 0x0010;

pub const IRQ_PHYSICAL_TIMER: u32 = platform::IRQ_PHYSICAL_TIMER;

pub fn init() {
    uart::write_line("GIC: init");

    mmio::write32(GICD_CTLR, 0);

    set_priority(IRQ_PHYSICAL_TIMER, 0x80);
    enable_irq(IRQ_PHYSICAL_TIMER);

    mmio::write32(GICC_PMR, 0xFF);
    mmio::write32(GICC_CTLR, 1);

    mmio::write32(GICD_CTLR, 1);

    uart::write_line("GIC: enabled");
}

fn set_priority(irq: u32, priority: u8) {
    let addr = GICD_IPRIORITYR + irq as usize;
    mmio::write8(addr, priority);
}

fn enable_irq(irq: u32) {
    let register = GICD_ISENABLER0 + ((irq / 32) as usize) * 4;
    let bit = 1u32 << (irq % 32);

    mmio::write32(register, bit);
}

pub fn acknowledge_irq() -> u32 {
    mmio::read32(GICC_IAR)
}

pub fn end_irq(value: u32) {
    mmio::write32(GICC_EOIR, value);
}
