use crate::arch::riscv64::cpu;
use crate::drivers::uart;
use crate::kernel::memory;

const PMP_ENTRIES: usize = cpu::PMP_ENTRIES;

const PMP_R: u64 = 1 << 0;
const PMP_W: u64 = 1 << 1;
const PMP_X: u64 = 1 << 2;
const PMP_A_TOR: u64 = 1 << 3;
const PMP_A_NAPOT: u64 = 3 << 3;

/// TOR vs NAPOT (do not mix as TOR *i−1*):
///
/// - TOR entry *i* matches `pmpaddr[i-1] <= y < pmpaddr[i]` regardless of
///   `pmpcfg[i-1]`. Store `end >> 2` (first byte *not* in the region).
///   Entry 0's lower bound is zero: `y < pmpaddr0`.
/// - NAPOT 4 KiB at 4 KiB-aligned `base`: `(base >> 2) | 0x1FF`, `A=NAPOT`.
///   Not `base >> 2` alone (that is TOR).
///
/// Layout: pmp0 TOR-deny `[0, __text_start)`, pmp1 TOR RX `.text`, pmp2 TOR R
/// `.rodata`, pmp3 NAPOT current stack (retargeted on switch).
pub fn init() {
    cpu::set_pmpcfg0(0);
    cpu::set_pmpcfg2(0);

    for index in 0..PMP_ENTRIES {
        cpu::set_pmpaddr(index, 0);
    }

    cpu::set_pmpaddr(0, memory::text_start() >> 2);
    cpu::set_pmpaddr(1, memory::text_end() >> 2);
    cpu::set_pmpaddr(2, memory::rodata_end() >> 2);
    cpu::set_pmpaddr(3, 0);

    let pmp0 = PMP_A_TOR;
    let pmp1 = PMP_R | PMP_X | PMP_A_TOR;
    let pmp2 = PMP_R | PMP_A_TOR;
    let pmp3 = PMP_R | PMP_W | PMP_A_NAPOT;
    cpu::set_pmpcfg0(pmp0 | (pmp1 << 8) | (pmp2 << 16) | (pmp3 << 24));
    cpu::set_pmpcfg2(0);

    dump();
}

pub fn set_current_stack(stack_start: u64) {
    cpu::set_pmpaddr(3, (stack_start >> 2) | 0x1FF);
}

unsafe extern "C" {
    static __trap_stack_bottom: u8;
    static __trap_stack_top: u8;
}

fn dump() {
    uart::write_line("");
    uart::write_line("PMP (TOR text/rodata, NAPOT stack):");

    uart::write_str("trap stack: ");
    uart::write_hex_u64(core::ptr::addr_of!(__trap_stack_bottom) as u64);
    uart::write_str(" - ");
    uart::write_hex_u64(core::ptr::addr_of!(__trap_stack_top) as u64);
    uart::write_line("");

    uart::write_str("pmpcfg0: ");
    uart::write_hex_u64(cpu::pmpcfg0());
    uart::write_line("");

    uart::write_str("pmpcfg2: ");
    uart::write_hex_u64(cpu::pmpcfg2());
    uart::write_line("");

    for index in 0..PMP_ENTRIES {
        uart::write_str("pmpaddr");
        uart::write_dec_u64(index as u64);
        uart::write_str(": ");
        uart::write_hex_u64(cpu::pmpaddr(index));
        uart::write_line("");
    }
}
