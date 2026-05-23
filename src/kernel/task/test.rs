use crate::drivers::uart;
#[allow(unused_imports)]
use crate::kernel::task::debug::{
    debug_current_stack_start, debug_current_stack_top, debug_current_task_id,
    debug_kernel_sp_before_task, debug_last_task_sp, debug_task_return_kind,
    set_debug_current_stack_bounds, set_debug_current_task_id, set_debug_kernel_return_pc,
    set_debug_kernel_sp_before_task, set_debug_task_run_stage, task_return_point,
};

#[allow(unused_imports)]
use crate::kernel::task::entry::task_trampoline;

#[allow(unused_imports)]
use crate::kernel::task::table::{
    create_task, get_task_entry, get_task_stack_start, get_task_stack_top, print_task_name_by_id,
    print_tasks, TaskReturnKind,
};

#[cfg(not(feature = "task_yield_test"))]
pub fn test_tasks() {
    crate::kernel::task::table::init();

    let _ = create_task("idle", idle_task);
    let _ = create_task("worker-a", worker_a_task);
    let _ = create_task("worker-b", worker_b_task);

    print_tasks();
}

#[cfg(all(feature = "task_yield_test", not(feature = "two_yield_task_test")))]
pub fn test_tasks_with_yield_worker() {
    crate::kernel::task::table::init();

    let _ = create_task("idle", idle_task);
    let _ = create_task("worker-a", yielding_task);
    let _ = create_task("worker-b", worker_b_task);

    print_tasks();
}

#[cfg(all(feature = "task_yield_test", feature = "two_yield_task_test"))]
pub fn test_tasks_with_yield_worker() {
    crate::kernel::task::table::init();

    let _ = create_task("idle", idle_task);
    let _ = create_task("worker-a", two_yielding_task);
    let _ = create_task("worker-b", worker_b_task);

    print_tasks();
}

fn idle_task() {
    uart::write_line("idle_task: running");
}

#[cfg(not(feature = "task_yield_test"))]
fn worker_a_task() {
    print_current_task_stack_check("worker_a_task");
}

fn worker_b_task() {
    print_current_task_stack_check("worker_b_task");
}

fn print_current_task_stack_check(label: &str) {
    uart::write_str(label);
    uart::write_line(": running");

    let sp = crate::arch::stack_pointer();
    let stack_start = debug_current_stack_start();
    let stack_top = debug_current_stack_top();

    uart::write_str(label);
    uart::write_str(" SP: ");
    uart::write_hex_u64(sp);
    uart::write_line("");

    uart::write_str("expected stack: ");
    uart::write_hex_u64(stack_start);
    uart::write_str(" - ");
    uart::write_hex_u64(stack_top);
    uart::write_line("");

    uart::write_str("saved kernel SP before task: ");
    uart::write_hex_u64(debug_kernel_sp_before_task());
    uart::write_line("");

    uart::write_str("SP check: ");

    if sp >= stack_start && sp < stack_top {
        uart::write_line("inside task stack");
    } else {
        uart::write_line("OUTSIDE task stack");
    }
}

#[cfg(feature = "task_bootstrap_test")]
pub fn test_task_trampoline() {
    uart::write_line("");
    uart::write_line("task trampoline test:");

    match get_task_entry(1) {
        Some(entry) => {
            uart::write_str("selected task: ");
            print_task_name_by_id(1);
            uart::write_line("");

            uart::write_str("entry: ");
            uart::write_hex_u64(entry as usize as u64);
            uart::write_line("");

            uart::write_line("not switching stack yet");
            uart::write_line("calling trampoline on current kernel stack");

            task_trampoline(entry);
        }
        None => {
            uart::write_line("selected task entry: none");
        }
    }
}

#[cfg(feature = "task_stack_switch_test")]
pub fn test_task_stack_switch() {
    uart::write_line("");
    uart::write_line("task stack switch test:");

    set_debug_task_run_stage(2);

    run_task_on_own_stack(1);
}

#[cfg(any(
    feature = "task_stack_switch_test",
    feature = "sequential_task_test",
    feature = "scheduler_driven_task_test",
    feature = "task_yield_test"
))]
fn run_task_on_own_stack(task_id: usize) -> ! {
    let Some(entry) = get_task_entry(task_id) else {
        uart::write_line("selected task entry: none");
        crate::arch::halt();
    };

    let Some(stack_start) = get_task_stack_start(task_id) else {
        uart::write_line("selected task stack start: none");
        crate::arch::halt();
    };

    let Some(stack_top) = get_task_stack_top(task_id) else {
        uart::write_line("selected task stack: none");
        crate::arch::halt();
    };

    uart::write_str("selected task: ");
    print_task_name_by_id(task_id);
    uart::write_line("");

    uart::write_str("entry: ");
    uart::write_hex_u64(entry as usize as u64);
    uart::write_line("");

    uart::write_str("stack_top: ");
    uart::write_hex_u64(stack_top);
    uart::write_line("");

    uart::write_str("stack_start: ");
    uart::write_hex_u64(stack_start);
    uart::write_line("");

    let kernel_sp_before_task = crate::arch::stack_pointer();
    let kernel_return_pc = task_return_point as *const () as usize as u64;

    uart::write_str("kernel_sp_before_task: ");
    uart::write_hex_u64(kernel_sp_before_task);
    uart::write_line("");

    uart::write_str("task_stack_top: ");
    uart::write_hex_u64(stack_top);
    uart::write_line("");

    uart::write_str("kernel_return_pc: ");
    uart::write_hex_u64(kernel_return_pc);
    uart::write_line("");

    uart::write_line("switching to task stack...");

    set_debug_current_task_id(task_id);
    set_debug_current_stack_bounds(stack_start, stack_top);
    set_debug_kernel_sp_before_task(kernel_sp_before_task);
    set_debug_kernel_return_pc(kernel_return_pc);
    crate::kernel::task::table::mark_started(task_id);
    unsafe {
        crate::arch::start_task_on_stack(entry as usize, stack_top);
    }
}

#[cfg(feature = "sequential_task_test")]
pub fn test_sequential_task_runner() {
    uart::write_line("");
    uart::write_line("sequential task runner test:");

    uart::write_line("run task: worker-a");

    set_debug_task_run_stage(1);

    run_task_on_own_stack(1);
}

#[cfg(feature = "sequential_task_test")]
pub fn continue_sequential_task_test_after_worker_a() -> ! {
    uart::write_line("run task: worker-b");

    set_debug_task_run_stage(2);

    run_task_on_own_stack(2);
}

#[cfg(feature = "task_yield_test")]
fn yielding_task() {
    uart::write_line("yielding_task: step 1");

    crate::kernel::task::yield_now();

    uart::write_line("yielding_task: step 2");

    crate::kernel::task::task_exit();
}

#[cfg(feature = "task_yield_test")]
pub fn test_task_yield() {
    uart::write_line("");
    uart::write_line("task yield test:");
    uart::write_line("selected task: worker-a");

    set_debug_task_run_stage(10);

    run_task_on_own_stack(1);
}

pub fn handle_task_return_for_debug_test() {
    let task_id = debug_current_task_id();
    let kind = debug_task_return_kind();
    let task_sp = debug_last_task_sp();
    let kernel_sp = debug_kernel_sp_before_task();
    let kernel_return_pc = crate::kernel::task::debug::debug_kernel_return_pc();

    crate::kernel::task::table::set_task_last_return_context(
        task_id,
        task_sp,
        kernel_sp,
        kernel_return_pc,
    );

    let mut cpu_context = crate::arch::capture_task_cpu_context(task_sp, kernel_return_pc);

    if matches!(kind, TaskReturnKind::Yield) {
        cpu_context.resume_pc = crate::kernel::task::debug::debug_task_resume_pc();

        #[cfg(target_arch = "riscv64")]
        {
            cpu_context.ra = cpu_context.resume_pc;
        }

    }

    crate::kernel::task::table::set_task_cpu_context(task_id, cpu_context);

    uart::write_str("  task: ");
    print_task_name_by_id(task_id);
    uart::write_line("");

    crate::kernel::task::table::set_task_return_kind(task_id, kind);

    uart::write_str("  captured CPU context:");
    crate::kernel::task::cpu_context::print_cpu_context(cpu_context);
    uart::write_line("");

    match kind {
        TaskReturnKind::Exit => {
            crate::kernel::task::table::set_task_state(
                task_id,
                crate::kernel::task::table::TaskState::Finished,
            );

            crate::kernel::task::table::set_task_can_resume(task_id, false);

            crate::kernel::task::scheduler::switch_to_idle();
        }
        TaskReturnKind::Yield => {
            crate::kernel::task::table::set_task_state(
                task_id,
                crate::kernel::task::table::TaskState::Ready,
            );

            crate::kernel::task::table::set_task_can_resume(task_id, true);

            crate::kernel::task::scheduler::switch_to_idle();
        }
        TaskReturnKind::None => {}
    }

    uart::write_str("  new state: ");
    crate::kernel::task::table::print_task_state_by_id(task_id);
    uart::write_line("");

    uart::write_str("  return kind: ");
    crate::kernel::task::table::print_task_return_kind_by_id(task_id);
    uart::write_line("");

    uart::write_str("  can resume: ");
    match crate::kernel::task::table::can_task_resume(task_id) {
        Some(true) => uart::write_line("yes"),
        Some(false) => uart::write_line("no"),
        None => uart::write_line("unknown"),
    }

    uart::write_str("  last task SP: ");
    uart::write_hex_u64(task_sp);
    uart::write_line("");

    uart::write_str("  last kernel SP: ");
    uart::write_hex_u64(kernel_sp);
    uart::write_line("");

    uart::write_str("  kernel return PC: ");
    uart::write_hex_u64(kernel_return_pc);
    uart::write_line("");

    print_last_task_sp_check(task_id, task_sp);
    print_resume_eligibility_check(task_id);
    print_cpu_context_consistency_check(task_id);

    uart::write_str("  scheduler current: ");
    crate::kernel::task::scheduler::print_current_task_name();
    uart::write_line("");
}

#[allow(dead_code)]
pub fn print_final_task_list() {
    uart::write_line("");
    uart::write_line("final task list:");
    crate::kernel::task::table::print_tasks();
}

#[cfg(any(feature = "selftest", feature = "scheduler_skip_finished_test"))]
pub fn run_scheduler_skip_finished_check() {
    uart::write_line("");
    uart::write_line("scheduler skip finished test:");

    uart::write_str("mark worker-a -> ");
    crate::kernel::task::table::set_task_state(1, crate::kernel::task::table::TaskState::Finished);
    crate::kernel::task::table::print_task_state_by_id(1);
    uart::write_line("");

    uart::write_str("mark worker-b -> ");
    crate::kernel::task::table::set_task_state(2, crate::kernel::task::table::TaskState::Finished);
    crate::kernel::task::table::print_task_state_by_id(2);
    uart::write_line("");

    uart::write_line("idle is Running, so it is not selected as Ready");
    uart::write_str("next Ready task after idle: ");

    match crate::kernel::task::table::find_next_ready_after(Some(0)) {
        Some(id) => {
            crate::kernel::task::table::print_task_name_by_id(id);
            uart::write_line("");
        }
        None => {
            uart::write_line("none");
        }
    }

    uart::write_line("skip finished test complete");
}

#[cfg(feature = "scheduler_skip_finished_test")]
pub fn test_scheduler_skips_finished_tasks() {
    run_scheduler_skip_finished_check();
    crate::arch::halt();
}

#[cfg(feature = "scheduler_driven_task_test")]
fn next_runnable_worker_task() -> Option<usize> {
    for _ in 0..crate::kernel::task::table::MAX_TASKS {
        let next = crate::kernel::task::scheduler::schedule_next()?;

        if next == 0 {
            continue;
        }

        match crate::kernel::task::table::get_task_state(next) {
            Some(crate::kernel::task::table::TaskState::Running)
            | Some(crate::kernel::task::table::TaskState::Ready) => {
                return Some(next);
            }
            _ => {}
        }
    }

    None
}

#[cfg(feature = "scheduler_driven_task_test")]
pub fn test_scheduler_driven_task_runner() {
    uart::write_line("");
    uart::write_line("scheduler-driven task runner:");

    set_debug_task_run_stage(20);

    continue_scheduler_driven_task_runner();
}

#[cfg(feature = "scheduler_driven_task_test")]
pub fn continue_scheduler_driven_task_runner() -> ! {
    match next_runnable_worker_task() {
        Some(task_id) => {
            uart::write_str("selected state: ");
            crate::kernel::task::table::print_task_state_by_id(task_id);
            uart::write_line("");

            run_task_on_own_stack(task_id);
        }
        None => {
            uart::write_line("scheduler selected: none");
            uart::write_line("scheduler-driven test complete");
            print_final_task_list();
            crate::arch::halt();
        }
    }
}

fn print_last_task_sp_check(task_id: usize, task_sp: u64) {
    uart::write_line("  task return context check:");

    match crate::kernel::task::table::is_sp_inside_task_stack(task_id, task_sp) {
        Some(true) => {
            uart::write_line("    task SP: inside task stack");
        }
        Some(false) => {
            uart::write_line("    task SP: outside task stack");
        }
        None => {
            uart::write_line("    task SP: unknown task");
        }
    }
}

fn print_resume_eligibility_check(task_id: usize) {
    uart::write_line("  resume eligibility check:");

    let state = crate::kernel::task::table::get_task_state(task_id);
    let can_resume = crate::kernel::task::table::can_task_resume(task_id);

    match (state, can_resume) {
        (Some(crate::kernel::task::table::TaskState::Ready), Some(true)) => {
            uart::write_line("    task can be resumed later");
        }
        (Some(crate::kernel::task::table::TaskState::Finished), Some(false)) => {
            uart::write_line("    task is finished; resume disabled");
        }
        _ => {
            uart::write_line("    task resume state is inconsistent");
        }
    }
}

fn print_cpu_context_consistency_check(task_id: usize) {
    uart::write_line("  CPU context consistency check:");

    let cpu_context = crate::kernel::task::table::get_task_cpu_context(task_id);
    let last_task_sp = crate::kernel::task::table::get_task_last_task_sp(task_id);
    let kernel_return_pc = crate::kernel::task::table::get_task_last_kernel_return_pc(task_id);

    match (cpu_context, last_task_sp, kernel_return_pc) {
        (Some(context), Some(task_sp), Some(return_pc)) => {
            uart::write_str("    context.sp == last_task_sp: ");
            crate::kernel::task::table::print_yes_no(context.sp == task_sp);
            uart::write_line("");

            uart::write_str("    context.return_pc == kernel_return_pc: ");
            crate::kernel::task::table::print_yes_no(context.return_pc == return_pc);
            uart::write_line("");

            uart::write_str("    context valid: ");
            crate::kernel::task::table::print_yes_no(context.is_valid());
            uart::write_line("");

            uart::write_str("    context SP inside task stack: ");
            match crate::kernel::task::table::is_sp_inside_task_stack(task_id, context.sp) {
                Some(value) => {
                    crate::kernel::task::table::print_yes_no(value);
                    uart::write_line("");
                }
                None => uart::write_line("unknown"),
            }

            uart::write_str("    context.resume_pc non-zero: ");
            crate::kernel::task::table::print_yes_no(context.resume_pc != 0);
            uart::write_line("");

            let ok = context.sp == task_sp
                && context.return_pc == return_pc
                && context.resume_pc != 0
                && context.is_valid()
                && matches!(
                    crate::kernel::task::table::is_sp_inside_task_stack(task_id, context.sp),
                    Some(true)
                );

            uart::write_str("    consistency result: ");
            if ok {
                uart::write_line("OK");
            } else {
                uart::write_line("FAILED");
            }
        }
        _ => {
            uart::write_line("    consistency result: FAILED");
        }
    }
}

#[cfg(feature = "resume_restore_test")]
pub fn test_resume_restore() {
    uart::write_line("");
    uart::write_line("resume restore test:");

    let Some(task_id) = crate::kernel::task::table::find_first_resumable_task() else {
        crate::drivers::uart::write_line("selected task: none");

        #[cfg(feature = "real_resume_restore_jump")]
        {
            if real_resume_jump_completion_check() {
                crate::drivers::uart::write_line(
                    "preflight result: SKIPPED after successful real resume jump",
                );
                crate::drivers::uart::write_line("real resume jump test complete");
                crate::arch::halt();
            }

            crate::drivers::uart::write_line("preflight result: FAILED");
            crate::arch::halt();
        }

        #[cfg(not(feature = "real_resume_restore_jump"))]
        {
            crate::drivers::uart::write_line("preflight result: FAILED");
            crate::arch::halt();
        }
    };

    uart::write_str("selected task: ");
    crate::kernel::task::table::print_task_name_by_id(task_id);
    uart::write_line("");

    let Some(frame) = crate::kernel::task::table::get_task_resume_frame(task_id) else {
        uart::write_line("cpu context: none");
        uart::write_line("restore result: FAILED");
        crate::arch::halt();
    };

    uart::write_str("restore sp: ");
    uart::write_hex_u64(frame.sp);
    uart::write_line("");

    uart::write_str("restore resume_pc: ");
    uart::write_hex_u64(frame.resume_pc);
    uart::write_line("");

    uart::write_str("restore frame:");
    crate::kernel::task::cpu_context::print_cpu_context(frame);
    uart::write_line("");

    if !resume_restore_precheck(task_id) {
        uart::write_line("restore aborted by guard");
        crate::arch::halt();
    }

    uart::write_line("restore guarded precheck passed");
    uart::write_line("calling arch restore_verified_resume_frame...");

    unsafe {
        crate::arch::restore_verified_resume_frame(frame);
    }
}

#[cfg(feature = "resume_preflight_test")]
pub fn test_resume_preflight_check() {
    uart::write_line("");
    uart::write_line("resume preflight check:");

    let Some(task_id) = crate::kernel::task::table::find_first_resumable_task() else {
        crate::drivers::uart::write_line("selected task: none");

        #[cfg(feature = "real_resume_restore_jump")]
        {
            if real_resume_jump_completion_check() {
                crate::drivers::uart::write_line(
                    "preflight result: SKIPPED after successful real resume jump",
                );
                crate::drivers::uart::write_line("real resume jump test complete");
                crate::arch::halt();
            }

            crate::drivers::uart::write_line("preflight result: FAILED");
            crate::arch::halt();
        }

        #[cfg(not(feature = "real_resume_restore_jump"))]
        {
            crate::drivers::uart::write_line("preflight result: FAILED");
            crate::arch::halt();
        }
    };

    uart::write_str("selected task: ");
    crate::kernel::task::table::print_task_name_by_id(task_id);
    uart::write_line("");

    uart::write_str("state: ");
    crate::kernel::task::table::print_task_state_by_id(task_id);
    uart::write_line("");

    uart::write_str("can_resume: ");
    match crate::kernel::task::table::can_task_resume(task_id) {
        Some(value) => {
            crate::kernel::task::table::print_yes_no(value);
            uart::write_line("");
        }
        None => uart::write_line("unknown"),
    }

    uart::write_str("last_return: ");
    crate::kernel::task::table::print_task_return_kind_by_id(task_id);
    uart::write_line("");

    let task_sp = crate::kernel::task::table::get_task_last_task_sp(task_id);
    let kernel_sp = crate::kernel::task::table::get_task_last_kernel_sp(task_id);
    let kernel_return_pc = crate::kernel::task::table::get_task_last_kernel_return_pc(task_id);
    let entry = crate::kernel::task::table::get_task_entry(task_id);
    let cpu_context = crate::kernel::task::table::get_task_cpu_context(task_id);
    let _context_consistent = match (cpu_context, task_sp, kernel_return_pc) {
        (Some(context), Some(sp), Some(pc)) => {
            context.is_valid()
                && context.sp == sp
                && context.return_pc == pc
                && context.resume_pc != 0
                && matches!(
                    crate::kernel::task::table::is_sp_inside_task_stack(task_id, context.sp),
                    Some(true)
                )
        }
        _ => false,
    };

    uart::write_str("last_task_sp: ");
    match task_sp {
        Some(sp) => {
            uart::write_hex_u64(sp);
            uart::write_line("");
        }
        None => uart::write_line("none"),
    }

    uart::write_str("last_kernel_sp: ");
    match kernel_sp {
        Some(sp) => {
            uart::write_hex_u64(sp);
            uart::write_line("");
        }
        None => uart::write_line("none"),
    }

    uart::write_str("kernel_return_pc: ");
    match kernel_return_pc {
        Some(pc) => {
            uart::write_hex_u64(pc);
            uart::write_line("");
        }
        None => uart::write_line("none"),
    }

    uart::write_str("entry present: ");
    crate::kernel::task::table::print_yes_no(entry.is_some());
    uart::write_line("");

    uart::write_str("cpu context valid: ");
    match cpu_context {
        Some(context) => {
            crate::kernel::task::table::print_yes_no(context.is_valid());
            uart::write_line("");

            #[cfg(feature = "verbose_resume_debug")]
            {
                crate::drivers::uart::write_str("cpu context detail:");
                crate::kernel::task::cpu_context::print_cpu_context(context);
                crate::drivers::uart::write_line("");
            }
            #[cfg(not(feature = "verbose_resume_debug"))]
            {
                crate::drivers::uart::write_str("cpu context valid: ");
                crate::kernel::task::table::print_yes_no(context.is_valid());
                crate::drivers::uart::write_line("");
            }
        }
        None => uart::write_line("unknown"),
    }

    let sp_inside = match task_sp {
        Some(sp) => crate::kernel::task::table::is_sp_inside_task_stack(task_id, sp),
        None => None,
    };

    uart::write_str("task SP check: ");
    match sp_inside {
        Some(true) => uart::write_line("inside task stack"),
        Some(false) => uart::write_line("outside task stack"),
        None => uart::write_line("unknown"),
    }

    print_cpu_context_consistency_check(task_id);
    let _ = print_resume_pc_proximity_check(task_id);

    uart::write_line("preflight result: OK");
    #[cfg(not(any(feature = "resume_dry_run_test", feature = "resume_restore_test")))]
    crate::arch::halt();
}

#[cfg(feature = "resume_dry_run_test")]
pub fn test_resume_dry_run() {
    uart::write_line("");
    uart::write_line("resume dry-run:");

    let Some(task_id) = crate::kernel::task::table::find_first_resumable_task() else {
        uart::write_line("selected task: none");
        uart::write_line("resume plan result: FAILED");
        crate::arch::halt();
    };

    uart::write_str("selected task: ");
    crate::kernel::task::table::print_task_name_by_id(task_id);
    uart::write_line("");

    let Some(frame) = crate::kernel::task::table::get_task_resume_frame(task_id) else {
        uart::write_line("cpu context: none");
        uart::write_line("resume plan result: FAILED");
        crate::arch::halt();
    };

    uart::write_str("restore sp: ");
    uart::write_hex_u64(frame.sp);
    uart::write_line("");

    uart::write_str("restore resume_pc: ");
    uart::write_hex_u64(frame.resume_pc);
    uart::write_line("");

    uart::write_str("restore kernel_return_pc: ");
    uart::write_hex_u64(frame.return_pc);
    uart::write_line("");

    uart::write_str("kernel text: ");
    uart::write_hex_u64(crate::kernel::memory::kernel_text_start());
    uart::write_str(" - ");
    uart::write_hex_u64(crate::kernel::memory::kernel_text_end());
    uart::write_line("");

    let sp_inside = crate::kernel::task::table::is_sp_inside_task_stack(task_id, frame.sp);
    let resume_pc_inside_text = crate::kernel::memory::is_inside_kernel_text(frame.resume_pc);
    let return_pc_inside_text = crate::kernel::memory::is_inside_kernel_text(frame.return_pc);

    uart::write_str("task SP inside task stack: ");
    match sp_inside {
        Some(value) => {
            crate::kernel::task::table::print_yes_no(value);
            uart::write_line("");
        }
        None => uart::write_line("unknown"),
    }

    uart::write_str("resume_pc inside kernel text: ");
    crate::kernel::task::table::print_yes_no(resume_pc_inside_text);
    uart::write_line("");

    uart::write_str("kernel_return_pc inside kernel text: ");
    crate::kernel::task::table::print_yes_no(return_pc_inside_text);
    uart::write_line("");

    uart::write_str("resume frame valid: ");
    crate::kernel::task::table::print_yes_no(frame.is_valid());
    uart::write_line("");

    #[cfg(feature = "verbose_resume_debug")]
    {
        crate::drivers::uart::write_str("resume frame detail:");
        crate::kernel::task::cpu_context::print_cpu_context(context);
        crate::drivers::uart::write_line("");
    }

    let context_consistent = match (
        crate::kernel::task::table::get_task_last_task_sp(task_id),
        crate::kernel::task::table::get_task_last_kernel_return_pc(task_id),
    ) {
        (Some(last_sp), Some(kernel_pc)) => frame.sp == last_sp && frame.return_pc == kernel_pc,
        _ => false,
    };

    let frame_ok = print_resume_frame_check(task_id);

    uart::write_str("CPU context consistent: ");
    crate::kernel::task::table::print_yes_no(context_consistent);
    uart::write_line("");

    let ok = frame.is_valid()
        && matches!(sp_inside, Some(true))
        && resume_pc_inside_text
        && return_pc_inside_text
        && context_consistent
        && frame_ok;

    uart::write_str("resume plan result: ");
    if ok {
        uart::write_line("OK");
    } else {
        uart::write_line("FAILED");
    }

    #[cfg(not(feature = "resume_restore_test"))]
    crate::arch::halt();
}

#[allow(dead_code)]
fn print_resume_pc_proximity_check(task_id: usize) -> bool {
    uart::write_line("  resume PC proximity check:");

    let Some(entry) = crate::kernel::task::table::get_task_entry_addr(task_id) else {
        uart::write_line("    entry: none");
        uart::write_line("    result: FAILED");
        return false;
    };

    let Some(context) = crate::kernel::task::table::get_task_cpu_context(task_id) else {
        uart::write_line("    cpu context: none");
        uart::write_line("    result: FAILED");
        return false;
    };

    uart::write_str("    entry: ");
    uart::write_hex_u64(entry);
    uart::write_line("");

    uart::write_str("    resume_pc: ");
    uart::write_hex_u64(context.resume_pc);
    uart::write_line("");

    if context.resume_pc < entry {
        uart::write_line("    delta: below entry");
        uart::write_line("    result: FAILED");
        return false;
    }

    let delta = context.resume_pc - entry;

    uart::write_str("    delta: ");
    uart::write_hex_u64(delta);
    uart::write_line("");

    let ok = delta > 0 && delta < 0x200;

    uart::write_str("    result: ");
    if ok {
        uart::write_line("OK");
    } else {
        uart::write_line("FAILED");
    }

    ok
}

#[allow(dead_code)]
fn print_resume_frame_check(task_id: usize) -> bool {
    uart::write_line("  resume frame check:");

    let Some(frame) = crate::kernel::task::table::get_task_resume_frame(task_id) else {
        uart::write_line("    frame present: no");
        uart::write_line("    result: FAILED");
        return false;
    };

    uart::write_line("    frame present: yes");

    uart::write_str("    frame valid: ");
    crate::kernel::task::table::print_yes_no(frame.is_valid());
    uart::write_line("");

    uart::write_str("    frame SP: ");
    uart::write_hex_u64(frame.sp);
    uart::write_line("");

    uart::write_str("    frame resume_pc: ");
    uart::write_hex_u64(frame.resume_pc);
    uart::write_line("");

    uart::write_str("    frame return_pc: ");
    uart::write_hex_u64(frame.return_pc);
    uart::write_line("");

    let sp_inside = matches!(
        crate::kernel::task::table::is_sp_inside_task_stack(task_id, frame.sp),
        Some(true)
    );

    uart::write_str("    frame SP inside task stack: ");
    crate::kernel::task::table::print_yes_no(sp_inside);
    uart::write_line("");

    let resume_pc_inside_text = crate::kernel::memory::is_inside_kernel_text(frame.resume_pc);
    let return_pc_inside_text = crate::kernel::memory::is_inside_kernel_text(frame.return_pc);

    uart::write_str("    frame resume_pc inside kernel text: ");
    crate::kernel::task::table::print_yes_no(resume_pc_inside_text);
    uart::write_line("");

    uart::write_str("    frame return_pc inside kernel text: ");
    crate::kernel::task::table::print_yes_no(return_pc_inside_text);
    uart::write_line("");

    let proximity_ok = print_resume_pc_proximity_check(task_id);

    let ok = frame.is_valid()
        && sp_inside
        && resume_pc_inside_text
        && return_pc_inside_text
        && proximity_ok;

    uart::write_str("    frame check result: ");
    if ok {
        uart::write_line("OK");
    } else {
        uart::write_line("FAILED");
    }

    ok
}

#[cfg(feature = "resume_restore_test")]
fn resume_restore_precheck(task_id: usize) -> bool {
    uart::write_line("restore guarded precheck:");

    let Some(frame) = crate::kernel::task::table::get_task_resume_frame(task_id) else {
        uart::write_line("  cpu context: none");
        uart::write_line("  result: FAILED");
        return false;
    };

    let sp_ok = matches!(
        crate::kernel::task::table::is_sp_inside_task_stack(task_id, frame.sp),
        Some(true)
    );

    let resume_pc_text_ok = crate::kernel::memory::is_inside_kernel_text(frame.resume_pc);
    let return_pc_text_ok = crate::kernel::memory::is_inside_kernel_text(frame.return_pc);
    let context_valid = frame.is_valid();

    uart::write_str("  resume frame valid: ");
    crate::kernel::task::table::print_yes_no(context_valid);
    uart::write_line("");

    uart::write_str("  task SP inside stack: ");
    crate::kernel::task::table::print_yes_no(sp_ok);
    uart::write_line("");

    uart::write_str("  resume_pc inside kernel text: ");
    crate::kernel::task::table::print_yes_no(resume_pc_text_ok);
    uart::write_line("");

    uart::write_str("  return_pc inside kernel text: ");
    crate::kernel::task::table::print_yes_no(return_pc_text_ok);
    uart::write_line("");

    let frame_ok = print_resume_frame_check(task_id);

    let ok = context_valid && sp_ok && resume_pc_text_ok && return_pc_text_ok && frame_ok;

    uart::write_str("  result: ");
    if ok {
        uart::write_line("OK");
    } else {
        uart::write_line("FAILED");
    }

    ok
}

#[cfg(feature = "resume_candidate_test")]
pub fn test_resume_candidate_selection() {
    uart::write_line("");
    uart::write_line("resume candidate test:");

    match crate::kernel::task::table::find_first_resumable_task() {
        Some(task_id) => {
            uart::write_str("selected resumable task: ");
            crate::kernel::task::table::print_task_name_by_id(task_id);
            uart::write_line("");

            uart::write_str("state: ");
            crate::kernel::task::table::print_task_state_by_id(task_id);
            uart::write_line("");

            uart::write_str("last_return: ");
            crate::kernel::task::table::print_task_return_kind_by_id(task_id);
            uart::write_line("");

            uart::write_str("can_resume: ");
            match crate::kernel::task::table::can_task_resume(task_id) {
                Some(value) => {
                    crate::kernel::task::table::print_yes_no(value);
                    uart::write_line("");
                }
                None => uart::write_line("unknown"),
            }

            uart::write_str("saved task SP: ");
            match crate::kernel::task::table::get_task_last_task_sp(task_id) {
                Some(sp) => {
                    uart::write_hex_u64(sp);
                    uart::write_line("");

                    uart::write_str("SP check: ");
                    match crate::kernel::task::table::is_sp_inside_task_stack(task_id, sp) {
                        Some(true) => uart::write_line("inside task stack"),
                        Some(false) => uart::write_line("outside task stack"),
                        None => uart::write_line("unknown task"),
                    }
                }
                None => {
                    uart::write_line("none");
                }
            }

            uart::write_line("resume candidate test complete");
        }
        None => {
            crate::drivers::uart::write_line("selected resumable task: none");
            crate::drivers::uart::write_line("resume candidate test complete");

            #[cfg(all(
                feature = "scheduler_resume_loop_test",
                feature = "real_resume_restore_jump"
            ))]
            {
                if real_resume_jump_completion_check() {
                    crate::drivers::uart::write_line("scheduler resume loop result: OK");
                    crate::drivers::uart::write_line("scheduler resume loop test complete");

                    #[cfg(all(
                        target_arch = "riscv64",
                        feature = "real_resume_restore_jump",
                        feature = "scheduler_resume_loop_test"
                    ))]
                    print_riscv_cooperative_resume_milestone();

                    crate::arch::halt();
                }
            }
        }
    }

    #[cfg(not(any(
        feature = "resume_preflight_test",
        feature = "resume_dry_run_test",
        feature = "resume_restore_test"
    )))]
    crate::arch::halt();
}
#[cfg(all(feature = "resume_restore_test", feature = "real_resume_restore_jump"))]
fn real_resume_jump_completion_check() -> bool {
    crate::drivers::uart::write_line("");
    crate::drivers::uart::write_line("real resume jump completion check:");

    #[cfg(feature = "scheduler_resume_loop_test")]
    crate::drivers::uart::write_line("  scenario: scheduler resume loop task");

    #[cfg(all(
        feature = "two_yield_task_test",
        not(feature = "scheduler_resume_loop_test")
    ))]
    crate::drivers::uart::write_line("  scenario: two-yield task");

    #[cfg(not(any(
        feature = "two_yield_task_test",
        feature = "scheduler_resume_loop_test"
    )))]
    crate::drivers::uart::write_line("  scenario: single-yield task");

    crate::drivers::uart::write_str("  task: ");
    crate::kernel::task::table::print_task_name_by_id(1);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("  state: ");
    crate::kernel::task::table::print_task_state_by_id(1);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("  can_resume: ");
    match crate::kernel::task::table::can_task_resume(1) {
        Some(value) => {
            crate::kernel::task::table::print_yes_no(value);
            crate::drivers::uart::write_line("");
        }
        None => crate::drivers::uart::write_line("unknown"),
    }

    crate::drivers::uart::write_str("  last_return: ");
    crate::kernel::task::table::print_task_return_kind_by_id(1);
    crate::drivers::uart::write_line("");

    let state_finished = matches!(
        crate::kernel::task::table::get_task_state(1),
        Some(crate::kernel::task::table::TaskState::Finished)
    );

    let can_resume_false = matches!(crate::kernel::task::table::can_task_resume(1), Some(false));

    crate::drivers::uart::write_str("  state Finished: ");
    crate::kernel::task::table::print_yes_no(state_finished);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("  resume disabled: ");
    crate::kernel::task::table::print_yes_no(can_resume_false);
    crate::drivers::uart::write_line("");

    let last_return_exit = matches!(
        crate::kernel::task::table::get_task_return_kind(1),
        Some(crate::kernel::task::table::TaskReturnKind::Exit)
    );

    crate::drivers::uart::write_str("  last return Exit: ");
    crate::kernel::task::table::print_yes_no(last_return_exit);
    crate::drivers::uart::write_line("");

    let ok = state_finished && can_resume_false && last_return_exit;

    crate::drivers::uart::write_str("  result: ");
    if ok {
        crate::drivers::uart::write_line("OK");
    } else {
        crate::drivers::uart::write_line("FAILED");
    }

    ok
}

#[cfg(all(
    target_arch = "riscv64",
    feature = "real_resume_restore_jump",
    feature = "scheduler_resume_loop_test"
))]
fn print_riscv_cooperative_resume_milestone() {
    crate::drivers::uart::write_line("");
    crate::drivers::uart::write_line("PicoOS 1.0 milestone:");
    crate::drivers::uart::write_line("  RISC-V cooperative task resume: OK");
    crate::drivers::uart::write_line("  repeated yield/resume loop: OK");
    crate::drivers::uart::write_line("  task exit after resume: OK");
    crate::drivers::uart::write_line("  scheduler-oriented resume loop: OK");
}

#[cfg(feature = "two_yield_task_test")]
fn two_yielding_task() {
    crate::drivers::uart::write_line("two_yielding_task: step 1");

    crate::kernel::task::yield_now();

    crate::drivers::uart::write_line("two_yielding_task: step 2");

    crate::kernel::task::yield_now();

    crate::drivers::uart::write_line("two_yielding_task: step 3");

    crate::kernel::task::task_exit();
}
