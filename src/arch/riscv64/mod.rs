use core::arch::asm;

pub mod cpu;
pub mod pmp;
pub mod restore;
pub mod timer;
pub mod traps;

pub use cpu::without_interrupts;

#[path = "yield.rs"]
mod task_yield;

pub use restore::{idle_exit_from_trap, mret_to_trap_image, restore_verified_resume_frame};

pub use task_yield::capture_task_cpu_context;

pub use task_yield::task_yield_boundary;

core::arch::global_asm!(include_str!("boot.S"));
core::arch::global_asm!(include_str!("trap.S"));

unsafe extern "C" {
    static trap_vector: u8;
    static __trap_stack_top: u8;
}

#[inline(always)]
fn symbol_addr(symbol: *const u8) -> u64 {
    symbol as u64
}

#[inline(always)]
pub fn halt() -> ! {
    loop {
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

pub fn reset_trap_stack_pointer_for_next_trap() {
    cpu::set_mscratch(trap_stack_top());
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

#[allow(dead_code)]
#[inline(never)]
pub unsafe fn start_task_on_stack(entry: usize, stack_top: u64) -> ! {
    unsafe {
        asm!(
        "mv sp, {stack}",
        "mv a0, {entry}",
        "call {trampoline}",
        stack = in(reg) stack_top,
        entry = in(reg) entry,
        trampoline = sym crate::kernel::task::task_trampoline_raw,
        options(noreturn)
        );
    }
}

#[inline(always)]
pub fn stack_pointer() -> u64 {
    cpu::stack_pointer()
}

#[inline(never)]
pub unsafe fn return_to_kernel_stack(kernel_sp: u64, return_pc: u64) -> ! {
    unsafe {
        asm!(
        "mv sp, {kernel_sp}",
        "jr {return_pc}",
        kernel_sp = in(reg) kernel_sp,
        return_pc = in(reg) return_pc,
        options(noreturn)
        );
    }
}

pub fn return_to_kernel_stack_checked(kernel_sp: u64, return_pc: u64) -> ! {
    if kernel_sp == 0 || !crate::kernel::memory::is_inside_kernel_text(return_pc) {
        crate::drivers::uart::write_line("invalid kernel return context");
        crate::arch::halt();
    }

    reset_trap_stack_pointer_for_next_trap();

    unsafe {
        return_to_kernel_stack(kernel_sp, return_pc);
    }
}
