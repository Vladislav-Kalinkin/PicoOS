#[cfg(any(
    feature = "scheduler_fault_lifecycle_test",
    feature = "two_task_resume_handoff_test",
    feature = "task_fault_test",
    feature = "task_sleep_test"
))]
use crate::drivers::uart;

pub fn print_task_zero_context_guard() {
    use crate::kernel::task::debug::TrapExecutionContext;

    crate::kernel::task::debug::set_debug_current_task_id(0);

    let ok = matches!(
        crate::kernel::task::debug::current_trap_execution_context(),
        TrapExecutionContext::Task
    );

    crate::kernel::task::debug::clear_debug_current_task_id();

    crate::drivers::uart::write_str("task id 0 context guard: ");
    crate::kernel::task::table::print_yes_no(ok);
    crate::drivers::uart::write_line("");

    if !ok {
        crate::arch::halt();
    }
}

#[cfg(feature = "task_sleep_test")]
pub fn test_task_sleep_wakeup_table_selftest() {
    uart::write_line("task sleep table selftest:");

    let task_id = 1usize;

    let blocked = crate::kernel::task::table::mark_task_blocked_until(task_id, 3);
    uart::write_str("  mark blocked until tick=3: ");
    crate::kernel::task::table::print_yes_no(blocked);
    uart::write_line("");

    let woke_early = crate::kernel::task::table::wake_sleeping_tasks(2);
    uart::write_str("  woke at tick=2: ");
    uart::write_dec_u64(woke_early as u64);
    uart::write_line("");

    let state_still_blocked = matches!(
        crate::kernel::task::table::get_task_state(task_id),
        Some(crate::kernel::task::table::TaskState::Blocked)
    );
    uart::write_str("  still blocked at tick=2: ");
    crate::kernel::task::table::print_yes_no(state_still_blocked);
    uart::write_line("");

    let woke_on_time = crate::kernel::task::table::wake_sleeping_tasks(3);
    uart::write_str("  woke at tick=3: ");
    uart::write_dec_u64(woke_on_time as u64);
    uart::write_line("");

    let state_ready = matches!(
        crate::kernel::task::table::get_task_state(task_id),
        Some(crate::kernel::task::table::TaskState::Ready)
    );
    uart::write_str("  state Ready after wake: ");
    crate::kernel::task::table::print_yes_no(state_ready);
    uart::write_line("");

    if blocked && woke_early == 0 && state_still_blocked && woke_on_time == 1 && state_ready {
        uart::write_line("task sleep wake result: OK");
    } else {
        uart::write_line("task sleep wake result: FAILED");
        crate::arch::halt();
    }
}

#[cfg(feature = "scheduler_fault_lifecycle_test")]
pub fn test_scheduler_fault_lifecycle_bootstrap() {
    uart::write_line("scheduler fault lifecycle bootstrap:");
    uart::write_line("bootstrap action: scheduler starts first fresh task");

    crate::kernel::task::scheduler::set_current_task(0);

    match crate::kernel::task::scheduler::run() {
        crate::kernel::task::scheduler::RunResult::NoRunnableTask => {
            uart::write_line("scheduler fault lifecycle bootstrap result: no runnable task");
        }
        crate::kernel::task::scheduler::RunResult::Failed => {
            uart::write_line("scheduler fault lifecycle bootstrap result: failed");
        }
    }
}

#[cfg(feature = "two_task_resume_handoff_test")]
pub fn test_two_task_resume_handoff_bootstrap() {
    uart::write_line("two-task handoff bootstrap:");
    uart::write_line("bootstrap action: scheduler starts first fresh task");

    crate::kernel::task::scheduler::set_current_task(0);

    match crate::kernel::task::scheduler::run() {
        crate::kernel::task::scheduler::RunResult::NoRunnableTask => {
            uart::write_line("two-task handoff bootstrap result: no runnable task");
            crate::arch::halt();
        }
        crate::kernel::task::scheduler::RunResult::Failed => {
            uart::write_line("two-task handoff bootstrap result: FAILED");
            crate::arch::halt();
        }
    }
}

#[cfg(feature = "task_fault_test")]
pub fn test_task_fault_bootstrap() {
    uart::write_line("task fault bootstrap:");
    uart::write_line("bootstrap action: scheduler starts first fresh task");

    crate::kernel::task::scheduler::set_current_task(0);

    match crate::kernel::task::scheduler::run() {
        crate::kernel::task::scheduler::RunResult::NoRunnableTask => {
            uart::write_line("task fault bootstrap result: no runnable task");
            crate::arch::halt();
        }
        crate::kernel::task::scheduler::RunResult::Failed => {
            uart::write_line("task fault bootstrap result: FAILED");
            crate::arch::halt();
        }
    }
}
