use core::arch::asm;

pub mod cpu;
pub mod ecall;
pub mod pmp;
pub mod restore;
pub mod timer;
pub mod traps;

pub use cpu::without_interrupts;

pub use restore::{idle_exit_from_trap, mret_to_trap_image};

core::arch::global_asm!(include_str!("boot.S"));
core::arch::global_asm!(include_str!("trap.S"));
core::arch::global_asm!(include_str!("trampoline.S"));

unsafe extern "C" {
    static trap_vector: u8;
    static __trap_stack_top: u8;
    pub fn user_trampoline();
}

pub fn user_trampoline_addr() -> u64 {
    user_trampoline as *const () as usize as u64
}

#[inline(always)]
fn symbol_addr(symbol: *const u8) -> u64 {
    symbol as u64
}

#[inline(always)]
pub fn halt() -> ! {
    loop {
        // SAFETY: `wfi` is valid in M-mode; this hart is parked until reset.
        unsafe {
            asm!("wfi", options(nomem, nostack, preserves_flags));
        }
    }
}

pub fn init_exceptions() {
    let trap_addr = symbol_addr(core::ptr::addr_of!(trap_vector));
    let trap_stack_top = trap_stack_top();

    cpu::set_mtvec(trap_addr);
    cpu::set_mscratch(trap_stack_top);

    crate::drivers::uart::write_str("mtvec: ");
    crate::drivers::uart::write_hex_u64(cpu::mtvec());
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("trap stack top: ");
    crate::drivers::uart::write_hex_u64(trap_stack_top);
    crate::drivers::uart::write_line("");
}

pub(crate) fn trap_stack_top() -> u64 {
    symbol_addr(core::ptr::addr_of!(__trap_stack_top))
}

pub fn is_trap_stack_addr(addr: u64) -> bool {
    let top = trap_stack_top();

    addr >= top - 4096 && addr < top
}

pub fn is_kernel_stack_addr(addr: u64) -> bool {
    let top = crate::kernel::memory::stack_top();
    addr >= top - 0x10000 && addr < top
}

pub fn enable_irq() {
    cpu::enable_machine_interrupts();
}

pub fn disable_irq() {
    cpu::disable_machine_interrupts();
}

#[inline(always)]
pub fn wait_for_interrupt() {
    // SAFETY: `wfi` is valid in M-mode idle.
    unsafe {
        asm!("wfi", options(nomem, nostack, preserves_flags));
    }
}

pub fn print_cpu_info() {
    crate::drivers::uart::write_line("riscv64 CPU info:");

    crate::drivers::uart::write_str("mhartid: ");
    crate::drivers::uart::write_hex_u64(cpu::mhartid());
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("mstatus: ");
    crate::drivers::uart::write_hex_u64(cpu::mstatus());
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("mtvec: ");
    crate::drivers::uart::write_hex_u64(cpu::mtvec());
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("mepc: ");
    crate::drivers::uart::write_hex_u64(cpu::mepc());
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("mcause: ");
    crate::drivers::uart::write_hex_u64(cpu::mcause());
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("mie: ");
    crate::drivers::uart::write_hex_u64(cpu::mie());
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("mip: ");
    crate::drivers::uart::write_hex_u64(cpu::mip());
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("SP: ");
    crate::drivers::uart::write_hex_u64(cpu::stack_pointer());
    crate::drivers::uart::write_line("");
}
