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

pub fn test_task_sleep_wakeup_table_selftest() {
    uart::write_line("task sleep table selftest:");

    let Some(task_id) = crate::kernel::task::table::spawn("sleep-probe", sleep_probe, 0) else {
        uart::write_line("task sleep wake result: FAILED");
        crate::arch::halt();
    };

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

    let finished = crate::kernel::task::table::mark_task_finished(task_id);
    let reaped = crate::kernel::task::table::destroy(task_id);

    let no_image_ok = started
        && blocked
        && woke_early == 0
        && state_still_blocked
        && resumable_early.is_none()
        && woke_on_time == 1
        && state_ready
        && !can_resume_after_wake
        && return_kind_none
        && resumable_after_wake.is_none()
        && finished
        && reaped;

    if !no_image_ok {
        uart::write_line("task sleep wake result: FAILED");
        crate::arch::halt();
    }

    uart::write_line("task sleep wake no-image: OK");
    test_task_sleep_wakeup_with_saved_image();
}

extern "C" fn sleep_probe(_arg: u64) {
    crate::arch::halt();
}

/// Wake must set `can_resume` from the saved image, not unconditionally.
fn test_task_sleep_wakeup_with_saved_image() {
    uart::write_line("task sleep table selftest (saved image):");

    let Some(task_id) = crate::kernel::task::table::spawn("sleep-img", sleep_probe, 0) else {
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
    let pc = crate::arch::riscv64::user_trampoline_addr();

    let mut image = crate::kernel::trap_frame::TrapImage::empty();
    image.gpr.sp = sp;
    image.gpr.ra = pc;
    image.mepc = pc;

    let injected = sp >= stack_start
        && sp < stack_top
        && crate::kernel::task::table::set_task_trap_image(task_id, &image);

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

const _: &[fn()] = &[print_task_zero_context_guard, test_task_sleep_wakeup_table_selftest];
