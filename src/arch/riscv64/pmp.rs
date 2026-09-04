use crate::arch::riscv64::cpu;
use crate::drivers::uart;
use crate::kernel::memory;

const PMP_ENTRIES: usize = cpu::PMP_ENTRIES;

/// TOR vs NAPOT (do not mix as TOR *i−1*):
///
/// - TOR entry *i* matches `pmpaddr[i-1] <= y < pmpaddr[i]` regardless of
///   `pmpcfg[i-1]`. Store `end >> 2` (first byte *not* in the region).
///   Entry 0's lower bound is zero: `y < pmpaddr0`.
/// - NAPOT 4 KiB at 4 KiB-aligned `base`: `(base >> 2) | 0x1FF`, `A=NAPOT`.
///   Not `base >> 2` alone (that is TOR).
///
/// Intended 0.2 layout (programmed OFF here; PR19 enables A bits):
/// pmp0 TOR/OFF bound at `__text_start`, pmp1 TOR RX `.text`, pmp2 TOR R
/// `.rodata`, pmp3 NAPOT current stack. TOR chain first, NAPOT last.
pub fn init() {
    cpu::set_pmpcfg0(0);
    cpu::set_pmpcfg2(0);

    for index in 0..PMP_ENTRIES {
        cpu::set_pmpaddr(index, 0);
    }

    cpu::set_pmpaddr(0, memory::text_start() >> 2);
    cpu::set_pmpaddr(1, memory::text_end() >> 2);
    cpu::set_pmpaddr(2, memory::rodata_end() >> 2);

    dump();
}

unsafe extern "C" {
    static __trap_stack_bottom: u8;
    static __trap_stack_top: u8;
}

fn dump() {
    uart::write_line("");
    uart::write_line("PMP (dump-only, all A=OFF):");

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
