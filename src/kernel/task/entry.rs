use crate::drivers::uart;
use crate::kernel::cpu;
use crate::kernel::task::table::TaskEntry;
use crate::kernel::task::table::TaskReturnKind;

pub fn task_trampoline(entry: TaskEntry) -> ! {
    #[cfg(any(
        feature = "task_yield_test",
        feature = "selftest",
        feature = "kernel_fault_guard_test"
    ))]
    {
        uart::write_line("");
        uart::write_line("task trampoline:");
        uart::write_str("calling entry: ");
        #[allow(clippy::fn_to_numeric_cast_any)]
        uart::write_hex_u64(entry as usize as u64);
        uart::write_line("");
    }

    entry();

    #[cfg(any(
        feature = "task_yield_test",
        feature = "selftest",
        feature = "kernel_fault_guard_test"
    ))]
    task_exit();

    #[cfg(not(any(
        feature = "task_yield_test",
        feature = "selftest",
        feature = "kernel_fault_guard_test"
    )))]
    crate::kernel::sys::u_sys_exit();
}

#[unsafe(no_mangle)]
pub extern "C" fn task_trampoline_raw(entry_addr: usize) -> ! {
    let entry: TaskEntry = unsafe { core::mem::transmute(entry_addr) };

    task_trampoline(entry);
}

#[allow(dead_code)]
pub fn task_exit() -> ! {
    uart::write_line("task returned; task_exit called");

    let current_sp = crate::arch::stack_pointer();
    let kernel_sp = cpu::kernel_sp_before_task();
    let return_pc = cpu::kernel_return_pc();

    cpu::set_last_task_sp(current_sp);

    uart::write_str("task_exit current SP: ");
    uart::write_hex_u64(current_sp);
    uart::write_line("");

    uart::write_str("task_exit saved kernel SP: ");
    uart::write_hex_u64(kernel_sp);
    uart::write_line("");

    uart::write_str("task_exit return PC: ");
    uart::write_hex_u64(return_pc);
    uart::write_line("");

    uart::write_line("returning to kernel stack...");

    cpu::set_task_return_kind(TaskReturnKind::Exit);

    crate::arch::return_to_kernel_stack_checked(kernel_sp, return_pc);
}

#[cfg(any(
    feature = "task_fault_test",
    feature = "scheduler_fault_lifecycle_test",
))]
pub fn task_fault() -> ! {
    uart::write_line("task fault requested");

    let current_sp = crate::arch::stack_pointer();
    let kernel_sp = cpu::kernel_sp_before_task();
    let return_pc = cpu::kernel_return_pc();

    uart::write_str("task_fault current SP: ");
    uart::write_hex_u64(current_sp);
    uart::write_line("");

    uart::write_str("task_fault saved kernel SP: ");
    uart::write_hex_u64(kernel_sp);
    uart::write_line("");

    uart::write_str("task_fault return PC: ");
    uart::write_hex_u64(return_pc);
    uart::write_line("");

    uart::write_line("returning to kernel stack after task fault...");

    crate::kernel::task::fault::return_current_task_fault(current_sp, kernel_sp, return_pc);
}

#[allow(dead_code)]
pub fn yield_now() {
    #[cfg(feature = "task_yield_test")]
    {
        crate::drivers::uart::write_line("task yield requested");
    }

    let kernel_sp = crate::kernel::cpu::kernel_sp_before_task();
    let return_pc = crate::kernel::cpu::kernel_return_pc();

    #[cfg(feature = "task_yield_test")]
    {
        crate::drivers::uart::write_str("yield saved kernel SP: ");
        crate::drivers::uart::write_hex_u64(kernel_sp);
        crate::drivers::uart::write_line("");

        crate::drivers::uart::write_str("yield return PC: ");
        crate::drivers::uart::write_hex_u64(return_pc);
        crate::drivers::uart::write_line("");

        crate::drivers::uart::write_line("yielding to kernel via RISC-V boundary...");
        crate::drivers::uart::write_line("yield ABI: save s0-s11 at boundary");
    }

    unsafe {
        crate::arch::task_yield_boundary(kernel_sp, return_pc);
    }
}

#[allow(dead_code)]
pub fn task_sleep_ticks(ticks: u64) {
    let Some(task_id) = crate::kernel::cpu::current() else {
        crate::kernel::log::fail("sleep", "sleep requested with no current task");
        crate::arch::halt();
    };
    let wake_tick = crate::kernel::ticks::get().saturating_add(ticks.max(1));

    #[cfg(feature = "task_sleep_runtime_e2e_test")]
    crate::kernel::log::info("sleep", "task requested timed sleep");

    if !crate::kernel::task::table::mark_task_blocked_until(task_id, wake_tick) {
        crate::kernel::log::fail("sleep", "failed to mark task Blocked");
        crate::arch::halt();
    }

    cpu::set_task_return_kind(TaskReturnKind::Sleep);

    let kernel_sp = crate::kernel::cpu::kernel_sp_before_task();
    let return_pc = crate::kernel::cpu::kernel_return_pc();

    unsafe {
        crate::arch::task_yield_boundary(kernel_sp, return_pc);
    }
}
