#[cfg(any(
    feature = "scheduler_fault_lifecycle_test",
    feature = "two_task_resume_handoff_test",
    feature = "task_fault_test",
    feature = "task_sleep_test"
))]
use crate::drivers::uart;

pub fn print_task_zero_context_guard() {
    use crate::kernel::cpu::TrapExecutionContext;

    crate::kernel::cpu::set_current(0);

    let ok = matches!(
        crate::kernel::cpu::trap_execution_context(),
        TrapExecutionContext::Task
    );

    crate::kernel::cpu::clear_current();

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
    let started = crate::kernel::task::table::mark_task_started(task_id);
    uart::write_str("  mark started before sleep: ");
    crate::kernel::task::table::print_yes_no(started);
    uart::write_line("");

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

    let resumable_early = crate::kernel::task::table::find_first_resumable_task();
    uart::write_str("  resumable at tick=2: ");
    match resumable_early {
        Some(id) => {
            crate::drivers::uart::write_dec_u64(id as u64);
            uart::write_line("");
        }
        None => uart::write_line("none"),
    }

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

    let can_resume_after_wake = matches!(
        crate::kernel::task::table::can_task_resume(task_id),
        Some(true)
    );
    uart::write_str("  can resume after wake: ");
    crate::kernel::task::table::print_yes_no(can_resume_after_wake);
    uart::write_line("");

    let return_kind_none = matches!(
        crate::kernel::task::table::get_task_return_kind(task_id),
        Some(crate::kernel::task::table::TaskReturnKind::None)
    );
    uart::write_str("  last return is None after wake: ");
    crate::kernel::task::table::print_yes_no(return_kind_none);
    uart::write_line("");

    let resumable_after_wake = crate::kernel::task::table::find_first_resumable_task();
    uart::write_str("  resumable at tick=3: ");
    match resumable_after_wake {
        Some(id) => {
            crate::drivers::uart::write_dec_u64(id as u64);
            uart::write_line("");
        }
        None => uart::write_line("none"),
    }

    let no_image_ok = started
        && blocked
        && woke_early == 0
        && state_still_blocked
        && resumable_early.is_none()
        && woke_on_time == 1
        && state_ready
        && !can_resume_after_wake
        && return_kind_none
        && resumable_after_wake.is_none();

    if !no_image_ok {
        uart::write_line("task sleep wake result: FAILED");
        crate::arch::halt();
    }

    uart::write_line("task sleep wake no-image: OK");
    test_task_sleep_wakeup_with_saved_image();
}

#[cfg(feature = "task_sleep_test")]
fn sleep_image_probe() {
    crate::arch::halt();
}

/// Wake must set `can_resume` from the saved image, not unconditionally.
#[cfg(feature = "task_sleep_test")]
fn test_task_sleep_wakeup_with_saved_image() {
    uart::write_line("task sleep table selftest (saved image):");

    let Some(task_id) = crate::kernel::task::table::create_task("sleep-img", sleep_image_probe)
    else {
        uart::write_line("task sleep wake image result: FAILED");
        crate::arch::halt();
    };

    let Some(stack_start) = crate::kernel::task::table::get_task_stack_start(task_id) else {
        uart::write_line("task sleep wake image result: FAILED");
        crate::arch::halt();
    };
    let Some(stack_top) = crate::kernel::task::table::get_task_stack_top(task_id) else {
        uart::write_line("task sleep wake image result: FAILED");
        crate::arch::halt();
    };

    let started = crate::kernel::task::table::mark_task_started(task_id);
    let sp = stack_top.saturating_sub(16);
    let pc = sleep_image_probe as *const () as u64;

    let image = crate::kernel::task::cpu_context::TaskCpuContext {
        sp,
        return_pc: pc,
        resume_pc: pc,
        ra: pc,
        s: [0; 12],
    };

    let injected = sp >= stack_start
        && sp < stack_top
        && crate::kernel::task::table::set_task_cpu_context(task_id, image)
        && crate::kernel::task::table::set_task_last_return_context(task_id, sp, 0, pc);

    uart::write_str("  injected resume image: ");
    crate::kernel::task::table::print_yes_no(started && injected);
    uart::write_line("");

    let blocked = crate::kernel::task::table::mark_task_blocked_until(task_id, 10);
    let woke_early = crate::kernel::task::table::wake_sleeping_tasks(9);
    let woke_on_time = crate::kernel::task::table::wake_sleeping_tasks(10);

    let can_resume_after_wake = matches!(
        crate::kernel::task::table::can_task_resume(task_id),
        Some(true)
    );
    uart::write_str("  can resume after wake with image: ");
    crate::kernel::task::table::print_yes_no(can_resume_after_wake);
    uart::write_line("");

    let return_kind_sleep = matches!(
        crate::kernel::task::table::get_task_return_kind(task_id),
        Some(crate::kernel::task::table::TaskReturnKind::Sleep)
    );
    uart::write_str("  last return is Sleep after wake with image: ");
    crate::kernel::task::table::print_yes_no(return_kind_sleep);
    uart::write_line("");

    let reaped = crate::kernel::task::table::mark_task_finished(task_id)
        && crate::kernel::task::table::destroy(task_id);

    if started
        && injected
        && blocked
        && woke_early == 0
        && woke_on_time == 1
        && can_resume_after_wake
        && return_kind_sleep
        && reaped
    {
        uart::write_line("task sleep wake image result: OK");
        uart::write_line("task sleep wake result: OK");
    } else {
        uart::write_line("task sleep wake image result: FAILED");
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
