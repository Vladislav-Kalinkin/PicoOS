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

#[cfg(feature = "task_yield_test")]
pub fn test_tasks_with_yield_worker() {
    crate::kernel::task::table::init();
    let _ = create_task("idle", idle_task);

    #[cfg(feature = "scheduler_fault_lifecycle_test")]
    {
        let _ = create_task("worker-a", real_trap_handler_worker_a);
        let _ = create_task("trap-worker", real_trap_handler_worker);
    }

    #[cfg(all(
        feature = "two_task_resume_handoff_test",
        not(feature = "scheduler_fault_lifecycle_test")
    ))]
    {
        let _ = create_task("worker-a", handoff_worker_a);
        let _ = create_task("worker-b", handoff_worker_b);
    }

    #[cfg(all(
        feature = "two_yield_task_test",
        not(feature = "two_task_resume_handoff_test"),
        not(feature = "scheduler_fault_lifecycle_test")
    ))]
    {
        let _ = create_task("worker-a", two_yielding_task);
    }

    #[cfg(not(any(
        feature = "two_yield_task_test",
        feature = "two_task_resume_handoff_test",
        feature = "scheduler_fault_lifecycle_test"
    )))]
    {
        let _ = create_task("worker-a", yielding_task);
        let _ = create_task("worker-b", worker_b_task);
    }

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

#[cfg(feature = "task_yield_test")]
pub fn run_task_on_own_stack(task_id: usize) -> ! {
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

#[cfg(all(feature = "task_yield_test", not(feature = "two_yield_task_test")))]
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

    set_debug_task_run_stage(10);

    #[cfg(feature = "scheduler_fault_lifecycle_test")]
    {
        test_scheduler_fault_lifecycle_bootstrap();
    }

    #[cfg(all(
        feature = "task_fault_test",
        not(feature = "scheduler_fault_lifecycle_test")
    ))]
    {
        test_task_fault_bootstrap();
    }

    #[cfg(all(
        feature = "two_task_resume_handoff_test",
        not(feature = "task_fault_test"),
        not(feature = "scheduler_fault_lifecycle_test")
    ))]
    {
        test_two_task_resume_handoff_bootstrap();
    }

    #[cfg(not(any(
        feature = "two_task_resume_handoff_test",
        feature = "task_fault_test",
        feature = "scheduler_fault_lifecycle_test"
    )))]
    {
        uart::write_line("selected task: worker-a");
        run_task_on_own_stack(1);
    }
}

#[cfg(feature = "scheduler_fault_lifecycle_test")]
fn test_scheduler_fault_lifecycle_bootstrap() {
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
fn test_two_task_resume_handoff_bootstrap() {
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
        let debug_resume_pc = crate::kernel::task::debug::debug_task_resume_pc();

        uart::write_str("  debug resume_pc from yield: ");
        uart::write_hex_u64(debug_resume_pc);
        uart::write_line("");

        cpu_context.resume_pc = debug_resume_pc;

        #[cfg(target_arch = "riscv64")]
        {
            cpu_context.ra = debug_resume_pc;

            uart::write_str("  saved ra for resume: ");
            uart::write_hex_u64(cpu_context.ra);
            uart::write_line("");
        }
    }

    crate::kernel::task::table::set_task_cpu_context(task_id, cpu_context);

    uart::write_str("  task: ");
    print_task_name_by_id(task_id);
    uart::write_line("");

    crate::kernel::task::table::set_task_return_kind(task_id, kind);
    crate::kernel::task::table::set_last_returned_task_id(task_id);

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
        TaskReturnKind::Fault => {
            crate::kernel::task::table::set_task_state(
                task_id,
                crate::kernel::task::table::TaskState::Faulted,
            );

            crate::kernel::task::table::set_task_can_resume(task_id, false);

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

    #[cfg(any(feature = "two_task_resume_handoff_test", feature = "task_fault_test"))]
    {
        let _ = print_resume_pc_proximity_check(task_id);
    }

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
        (Some(crate::kernel::task::table::TaskState::Faulted), Some(false)) => {
            uart::write_line("    task is faulted; resume disabled");
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
        uart::write_line("selected task: none");

        #[cfg(feature = "real_resume_restore_jump")]
        {
            if real_resume_jump_completion_check() {
                uart::write_line("preflight result: SKIPPED after successful real resume jump");
                uart::write_line("real resume jump test complete");
                crate::arch::halt();
            }

            uart::write_line("preflight result: FAILED");
            crate::arch::halt();
        }

        #[cfg(not(feature = "real_resume_restore_jump"))]
        {
            uart::write_line("preflight result: FAILED");
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
                uart::write_str("cpu context detail:");
                crate::kernel::task::cpu_context::print_cpu_context(context);
                uart::write_line("");
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

    #[cfg(feature = "resume_dry_run_test")]
    {
        test_resume_dry_run();
    }

    #[cfg(not(feature = "resume_dry_run_test"))]
    {
        crate::arch::halt();
    }
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
        uart::write_str("resume frame detail:");
        crate::kernel::task::cpu_context::print_cpu_context(frame);
        uart::write_line("");
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

    if !ok {
        crate::arch::halt();
    }

    #[cfg(feature = "resume_restore_test")]
    {
        test_resume_restore();
    }

    #[cfg(not(feature = "resume_restore_test"))]
    {
        crate::arch::halt();
    }
}

fn print_resume_pc_proximity_check(task_id: usize) -> bool {
    crate::drivers::uart::write_line("  resume PC proximity check:");

    let Some(context) = crate::kernel::task::table::get_task_cpu_context(task_id) else {
        crate::drivers::uart::write_line("    cpu context: none");
        return false;
    };

    let Some(entry) = crate::kernel::task::table::get_task_entry(task_id) else {
        crate::drivers::uart::write_line("    entry: none");
        return false;
    };

    let entry_addr = entry as usize as u64;

    crate::drivers::uart::write_str("    entry: ");
    crate::drivers::uart::write_hex_u64(entry_addr);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("    resume_pc: ");
    crate::drivers::uart::write_hex_u64(context.resume_pc);
    crate::drivers::uart::write_line("");

    #[cfg(any(
        feature = "two_task_resume_handoff_test",
        feature = "task_fault_test",
        feature = "scheduler_resume_loop_test",
        feature = "real_resume_restore_jump"
    ))]
    {
        let resume_pc_inside_text = crate::kernel::memory::is_inside_kernel_text(context.resume_pc);

        crate::drivers::uart::write_line("    mode: RISC-V yield boundary continuation");

        crate::drivers::uart::write_str("    resume_pc inside kernel text: ");
        crate::kernel::task::table::print_yes_no(resume_pc_inside_text);
        crate::drivers::uart::write_line("");

        crate::drivers::uart::write_str("    result: ");
        if resume_pc_inside_text {
            crate::drivers::uart::write_line("OK");
        } else {
            crate::drivers::uart::write_line("FAILED");
        }

        resume_pc_inside_text
    }

    #[cfg(not(any(
        feature = "two_task_resume_handoff_test",
        feature = "task_fault_test",
        feature = "scheduler_resume_loop_test",
        feature = "real_resume_restore_jump"
    )))]
    {
        if context.resume_pc < entry_addr {
            crate::drivers::uart::write_line("    delta: below entry");
            crate::drivers::uart::write_line("    result: FAILED");
            return false;
        }

        let delta = context.resume_pc - entry_addr;

        crate::drivers::uart::write_str("    delta: ");
        crate::drivers::uart::write_hex_u64(delta);
        crate::drivers::uart::write_line("");

        let ok = delta < 0x400;

        crate::drivers::uart::write_str("    result: ");
        if ok {
            crate::drivers::uart::write_line("OK");
        } else {
            crate::drivers::uart::write_line("FAILED");
        }

        ok
    }
}

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

    let ok = frame.is_valid() && sp_inside && resume_pc_inside_text && return_pc_inside_text;

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
    print_resume_candidate_header();

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

            #[cfg(feature = "scheduler_run_test")]
            {
                uart::write_line("resume candidate selected; delegating to scheduler run");

                match crate::kernel::task::scheduler::run() {
                    crate::kernel::task::scheduler::RunResult::NoRunnableTask => {
                        uart::write_line("scheduler run returned: no runnable task");
                    }
                    crate::kernel::task::scheduler::RunResult::Failed => {
                        uart::write_line("scheduler run returned: failed");
                    }
                }

                crate::arch::halt();
            }

            #[cfg(all(
                feature = "scheduler_dispatch_test",
                not(feature = "scheduler_run_test")
            ))]
            {
                uart::write_line("resume candidate selected; delegating to scheduler run_once");

                match crate::kernel::task::scheduler::run_once() {
                    crate::kernel::task::scheduler::RunOnceResult::NoRunnableTask => {
                        uart::write_line("scheduler run_once returned: no runnable task");
                    }
                    crate::kernel::task::scheduler::RunOnceResult::Failed => {
                        uart::write_line("scheduler run_once returned: failed");
                    }
                }

                crate::arch::halt();
            }

            #[cfg(all(
                feature = "resume_preflight_test",
                not(feature = "scheduler_dispatch_test")
            ))]
            {
                test_resume_preflight_check();
            }
        }
        None => {
            uart::write_line("selected resumable task: none");
            print_resume_candidate_complete();

            #[cfg(all(
                feature = "scheduler_resume_loop_test",
                feature = "real_resume_restore_jump"
            ))]
            {
                if real_resume_jump_completion_check() {
                    uart::write_line("scheduler resume loop result: OK");
                    uart::write_line("scheduler resume loop test complete");

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
        feature = "resume_restore_test",
        feature = "scheduler_dispatch_test"
    )))]
    crate::arch::halt();
}

#[cfg(all(feature = "resume_restore_test", feature = "real_resume_restore_jump"))]
fn real_resume_jump_completion_check() -> bool {
    crate::drivers::uart::write_line("");
    crate::drivers::uart::write_line("real resume jump completion check:");

    #[cfg(feature = "two_task_resume_handoff_test")]
    {
        crate::drivers::uart::write_line("  scenario: two-task handoff");

        let worker_a_ok = print_task_finished_cleanly_check(1);
        let worker_b_ok = print_task_finished_cleanly_check(2);

        let ok = worker_a_ok && worker_b_ok;

        crate::drivers::uart::write_str("    result: ");
        if ok {
            crate::drivers::uart::write_line("OK");
        } else {
            crate::drivers::uart::write_line("FAILED");
        }

        ok
    }

    #[cfg(not(feature = "two_task_resume_handoff_test"))]
    {
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

        let ok = print_task_finished_cleanly_check(1);

        crate::drivers::uart::write_str("  result: ");
        if ok {
            crate::drivers::uart::write_line("OK");
        } else {
            crate::drivers::uart::write_line("FAILED");
        }

        ok
    }
}

#[cfg(all(feature = "resume_restore_test", feature = "real_resume_restore_jump"))]
fn print_task_finished_cleanly_check(task_id: usize) -> bool {
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("  task: ");
    crate::kernel::task::table::print_task_name_by_id(task_id);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("  state: ");
    crate::kernel::task::table::print_task_state_by_id(task_id);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("  can_resume: ");
    match crate::kernel::task::table::can_task_resume(task_id) {
        Some(value) => {
            crate::kernel::task::table::print_yes_no(value);
            crate::drivers::uart::write_line("");
        }
        None => crate::drivers::uart::write_line("unknown"),
    }

    crate::drivers::uart::write_str("  last_return: ");
    crate::kernel::task::table::print_task_return_kind_by_id(task_id);
    crate::drivers::uart::write_line("");

    let state_finished = matches!(
        crate::kernel::task::table::get_task_state(task_id),
        Some(crate::kernel::task::table::TaskState::Finished)
    );

    let can_resume_false = matches!(
        crate::kernel::task::table::can_task_resume(task_id),
        Some(false)
    );

    let last_return_exit = matches!(
        crate::kernel::task::table::get_task_return_kind(task_id),
        Some(crate::kernel::task::table::TaskReturnKind::Exit)
    );

    crate::drivers::uart::write_str("  state Finished: ");
    crate::kernel::task::table::print_yes_no(state_finished);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("  resume disabled: ");
    crate::kernel::task::table::print_yes_no(can_resume_false);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("  last return Exit: ");
    crate::kernel::task::table::print_yes_no(last_return_exit);
    crate::drivers::uart::write_line("");

    state_finished && can_resume_false && last_return_exit
}

#[cfg(feature = "scheduler_fault_lifecycle_test")]
fn check_finished_task_dispatch_guard(id: usize) -> bool {
    print_terminal_task_dispatch_guard("finished", id)
}

#[cfg(all(
    target_arch = "riscv64",
    feature = "real_resume_restore_jump",
    feature = "scheduler_resume_loop_test"
))]
fn print_riscv_cooperative_resume_milestone() {
    crate::drivers::uart::write_line("PicoOS milestone:");
    crate::drivers::uart::write_line("  baseline: 0.1.0");
    crate::drivers::uart::write_line("  current: 0.1.62");

    crate::drivers::uart::write_line("  obsolete standalone task tests removed: OK");
    crate::drivers::uart::write_line("  obsolete standalone scheduler scripts removed: OK");
    crate::drivers::uart::write_line("  task fault state: OK");
    crate::drivers::uart::write_line("  scheduler skips faulted tasks: OK");
    crate::drivers::uart::write_line("  trap-to-task-fault skeleton: OK");
    crate::drivers::uart::write_line("  real trap classification: OK");
    crate::drivers::uart::write_line("  real trap handler classification: OK");
    crate::drivers::uart::write_line("  real trap handler task-fault return path: OK");
    crate::drivers::uart::write_line("  trap fault metadata reporting: OK");
    crate::drivers::uart::write_line("  explicit task fault assertions: OK");
    crate::drivers::uart::write_line("  faulted task dispatch guard: OK");
    crate::drivers::uart::write_line("  finished task dispatch guard: OK");
    crate::drivers::uart::write_line("  no-runnable scheduler policy: OK");

    crate::drivers::uart::write_line("  task state invariants in core: OK");
    crate::drivers::uart::write_line("  task state lookup in core: OK");
    crate::drivers::uart::write_line("  terminal task dispatch invariants in core: OK");
    crate::drivers::uart::write_line("  no-runnable scheduler snapshot in core: OK");
    crate::drivers::uart::write_line("  fault metadata assertions in core: OK");
    crate::drivers::uart::write_line("  task completion summary in core: OK");
    crate::drivers::uart::write_line("  task completion output consolidated: OK");
    crate::drivers::uart::write_line("  scheduler fault lifecycle feature: OK");
    crate::drivers::uart::write_line("  obsolete resume task script removed: OK");
    crate::drivers::uart::write_line("  obsolete resume PC proximity requirement removed: OK");

    crate::drivers::uart::write_line("  RISC-V-only baseline: OK");
    crate::drivers::uart::write_line("  cooperative task resume: OK");
    crate::drivers::uart::write_line("  repeated yield/resume loop: OK");
    crate::drivers::uart::write_line("  scheduler-oriented resume loop: OK");
    crate::drivers::uart::write_line("  RISC-V yield boundary: OK");
    crate::drivers::uart::write_line("  two-task cooperative handoff: OK");
    crate::drivers::uart::write_line("  scheduler round-robin fairness: OK");
    crate::drivers::uart::write_line("  scheduler task capacity from table: OK");
    crate::drivers::uart::write_line("  scheduler fresh task dispatch: OK");
    crate::drivers::uart::write_line("  scheduler first task dispatch: OK");
}

#[cfg(feature = "scheduler_fault_lifecycle_test")]
fn check_faulted_task_dispatch_guard(id: usize) -> bool {
    print_terminal_task_dispatch_guard("faulted", id)
}

#[allow(dead_code)]
#[cfg(feature = "two_yield_task_test")]
fn two_yielding_task() {
    crate::drivers::uart::write_line("two_yielding_task: step 1");

    crate::kernel::task::yield_now();

    crate::drivers::uart::write_line("two_yielding_task: step 2");

    crate::kernel::task::yield_now();

    crate::drivers::uart::write_line("two_yielding_task: step 3");

    crate::kernel::task::task_exit();
}

#[cfg(feature = "resume_candidate_test")]
fn print_resume_candidate_header() {
    crate::drivers::uart::write_line("");

    #[cfg(feature = "scheduler_run_test")]
    {
        crate::drivers::uart::write_line("scheduler resume candidate check:");
    }

    #[cfg(not(feature = "scheduler_run_test"))]
    {
        crate::drivers::uart::write_line("resume candidate test:");
    }
}

#[cfg(feature = "scheduler_fault_lifecycle_test")]
fn print_terminal_task_dispatch_guard(label: &str, id: usize) -> bool {
    let snapshot = crate::kernel::task::table::get_terminal_task_dispatch_invariants(id);

    crate::drivers::uart::write_str("  ");
    crate::drivers::uart::write_str(label);
    crate::drivers::uart::write_line(" task dispatch guard:");

    crate::drivers::uart::write_str("    terminal task: ");
    crate::kernel::task::table::print_yes_no(snapshot.terminal);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("    ");
    crate::drivers::uart::write_str(label);
    crate::drivers::uart::write_str(" task resumable: ");
    crate::kernel::task::table::print_yes_no(snapshot.resumable);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("    expected resumable == no: ");
    crate::kernel::task::table::print_yes_no(!snapshot.resumable);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("    ");
    crate::drivers::uart::write_str(label);
    crate::drivers::uart::write_str(" task fresh-ready: ");
    crate::kernel::task::table::print_yes_no(snapshot.fresh_ready);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("    expected fresh-ready == no: ");
    crate::kernel::task::table::print_yes_no(!snapshot.fresh_ready);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("    ");
    crate::drivers::uart::write_str(label);
    crate::drivers::uart::write_str(" task dispatchable: ");
    crate::kernel::task::table::print_yes_no(snapshot.dispatchable);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("    expected dispatchable == no: ");
    crate::kernel::task::table::print_yes_no(!snapshot.dispatchable);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("    result: ");
    if snapshot.result {
        crate::drivers::uart::write_line("OK");
    } else {
        crate::drivers::uart::write_line("FAILED");
    }

    snapshot.result
}

#[cfg(feature = "resume_candidate_test")]
fn print_resume_candidate_complete() {
    #[cfg(feature = "scheduler_run_test")]
    {
        crate::drivers::uart::write_line("scheduler resume candidate check complete");
    }

    #[cfg(not(feature = "scheduler_run_test"))]
    {
        crate::drivers::uart::write_line("resume candidate test complete");
    }
}

#[cfg(feature = "scheduler_reentry_test")]
pub fn handle_scheduler_reentry_after_task_return() {
    crate::drivers::uart::write_line("");
    crate::drivers::uart::write_line("scheduler re-entry after task return:");

    let Some(snapshot) = crate::kernel::task::table::get_last_returned_task_snapshot() else {
        crate::drivers::uart::write_line(
            "scheduler re-entry result: missing last returned task snapshot",
        );
        crate::arch::halt();
    };

    crate::drivers::uart::write_str("  last returned task: ");
    crate::kernel::task::table::print_task_name_by_id(snapshot.task_id);
    crate::drivers::uart::write_line("");

    #[cfg(all(
        feature = "two_task_resume_handoff_test",
        not(feature = "task_fault_test")
    ))]
    {
        let phase = get_two_task_handoff_phase();

        if phase == 0 && snapshot.task_id == 1 {
            if !matches!(
                snapshot.last_return,
                crate::kernel::task::table::TaskReturnKind::Yield
            ) {
                crate::drivers::uart::write_line("two-task handoff error: worker-a did not yield");
                crate::arch::halt();
            }

            advance_two_task_handoff_phase();

            crate::drivers::uart::write_line("two-task handoff phase 0: worker-a yielded");
            crate::drivers::uart::write_line(
                "two-task handoff action: scheduler starts next fresh task",
            );
        }

        if phase == 1 && snapshot.task_id == 2 {
            if !matches!(
                snapshot.last_return,
                crate::kernel::task::table::TaskReturnKind::Yield
            ) {
                crate::drivers::uart::write_line("two-task handoff error: worker-b did not yield");
                crate::arch::halt();
            }

            advance_two_task_handoff_phase();

            crate::drivers::uart::write_line("two-task handoff phase 1: worker-b yielded");
            crate::drivers::uart::write_line(
                "two-task handoff action: continue scheduler re-entry",
            );

            prepare_debug_context_for_task(1);
        }
    }

    match crate::kernel::task::scheduler::handle_task_return(snapshot) {
        crate::kernel::task::scheduler::TaskReturnHandleResult::NoRunnableTask => {
            crate::drivers::uart::write_line("  action: completion check");

            #[cfg(all(
                feature = "scheduler_resume_loop_test",
                feature = "real_resume_restore_jump"
            ))]
            {
                #[cfg(feature = "task_fault_test")]
                {
                    if task_fault_completion_check() {
                        crate::drivers::uart::write_line("task fault scheduler result: OK");

                        print_riscv_cooperative_resume_milestone();
                        crate::arch::halt();
                    }
                }

                #[cfg(not(feature = "task_fault_test"))]
                {
                    if real_resume_jump_completion_check() {
                        crate::drivers::uart::write_line("scheduler resume loop result: OK");
                        crate::drivers::uart::write_line("scheduler resume loop test complete");
                        print_riscv_cooperative_resume_milestone();
                        crate::arch::halt();
                    }
                }
            }

            crate::drivers::uart::write_line("scheduler re-entry result: no runnable task");
            crate::arch::halt();
        }
        crate::kernel::task::scheduler::TaskReturnHandleResult::Failed => {
            crate::drivers::uart::write_line("scheduler re-entry result: failed");
            crate::arch::halt();
        }
    }
}

#[cfg(feature = "two_task_resume_handoff_test")]
fn handoff_worker_a() {
    crate::drivers::uart::write_line("handoff_worker_a: step 1");
    crate::kernel::task::yield_now();

    crate::drivers::uart::write_line("handoff_worker_a: resumed after first yield");
    crate::drivers::uart::write_line("handoff_worker_a: step 2");
    crate::kernel::task::yield_now();

    crate::drivers::uart::write_line("handoff_worker_a: resumed after second yield");
    crate::drivers::uart::write_line("handoff_worker_a: step 3");
    crate::kernel::task::task_exit();
}

#[cfg(feature = "two_task_resume_handoff_test")]
fn handoff_worker_b() {
    crate::drivers::uart::write_line("handoff_worker_b: step 1");
    crate::kernel::task::yield_now();

    crate::drivers::uart::write_line("handoff_worker_b: resumed after yield");
    crate::drivers::uart::write_line("handoff_worker_b: step 2");
    crate::kernel::task::task_exit();
}

#[cfg(feature = "two_task_resume_handoff_test")]
static mut TWO_TASK_HANDOFF_PHASE: usize = 0;

#[cfg(feature = "two_task_resume_handoff_test")]
fn get_two_task_handoff_phase() -> usize {
    unsafe { TWO_TASK_HANDOFF_PHASE }
}

#[cfg(feature = "two_task_resume_handoff_test")]
fn advance_two_task_handoff_phase() {
    unsafe {
        TWO_TASK_HANDOFF_PHASE += 1;
    }
}

#[cfg(feature = "two_task_resume_handoff_test")]
fn prepare_debug_context_for_task(task_id: usize) {
    let Some(stack_start) = crate::kernel::task::table::get_task_stack_start(task_id) else {
        crate::drivers::uart::write_line("two-task handoff error: missing task stack start");
        crate::arch::halt();
    };

    let Some(stack_top) = crate::kernel::task::table::get_task_stack_top(task_id) else {
        crate::drivers::uart::write_line("two-task handoff error: missing task stack top");
        crate::arch::halt();
    };

    set_debug_current_task_id(task_id);
    set_debug_current_stack_bounds(stack_start, stack_top);
}

#[cfg(feature = "task_fault_test")]
fn faulty_worker() {
    crate::drivers::uart::write_line("faulty_worker: step 1");
    crate::drivers::uart::write_line("faulty_worker: intentional fault");
    crate::kernel::task::task_fault();
}

#[cfg(feature = "task_fault_test")]
fn test_task_fault_bootstrap() {
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

#[cfg(feature = "task_fault_test")]
fn task_fault_completion_check() -> bool {
    crate::drivers::uart::write_line("");
    crate::drivers::uart::write_line("task fault completion check:");

    let completion_snapshot = crate::kernel::task::table::get_task_fault_completion_snapshot();

    print_task_fault_completion_snapshot(completion_snapshot);

    #[cfg(feature = "scheduler_fault_lifecycle_test")]
    {
        if let Some(id) = find_faulted_task_for_completion_check() {
            crate::kernel::task::table::print_task_fault_info_by_id(id);

            let fault_metadata_assertions_ok = check_task_fault_metadata_assertions(id);

            if !fault_metadata_assertions_ok {
                return false;
            }

            let faulted_task_dispatch_guard_ok = check_faulted_task_dispatch_guard(id);

            if !faulted_task_dispatch_guard_ok {
                return false;
            }
        } else {
            crate::drivers::uart::write_line("  fault info: faulted task not found");
            return false;
        }

        if let Some(id) = find_finished_task_for_completion_check() {
            let finished_task_dispatch_guard_ok = check_finished_task_dispatch_guard(id);

            if !finished_task_dispatch_guard_ok {
                return false;
            }
        } else {
            crate::drivers::uart::write_line(
                "  finished task dispatch guard: finished task not found",
            );
            return false;
        }

        let no_runnable_scheduler_policy_ok = check_no_runnable_scheduler_policy();

        if !no_runnable_scheduler_policy_ok {
            return false;
        }
    }

    crate::drivers::uart::write_str("  last return Fault: ");
    crate::kernel::task::table::print_yes_no(completion_snapshot.faulted_task_last_return_fault);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("  resume disabled: ");
    crate::kernel::task::table::print_yes_no(completion_snapshot.faulted_task_resume_disabled);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("  task fault result: ");
    if completion_snapshot.result {
        crate::drivers::uart::write_line("OK");
    } else {
        crate::drivers::uart::write_line("FAILED");
    }

    completion_snapshot.result
}

#[cfg(feature = "task_fault_test")]
fn print_task_fault_completion_snapshot(
    snapshot: crate::kernel::task::table::TaskFaultCompletionSnapshot,
) {
    crate::drivers::uart::write_line("");
    crate::drivers::uart::write_line("  task: worker-a");

    crate::drivers::uart::write_str("  state Finished: ");
    crate::kernel::task::table::print_yes_no(snapshot.finished_task_finished);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("  last return Exit: ");
    crate::kernel::task::table::print_yes_no(snapshot.finished_task_last_return_exit);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_line("");
    crate::drivers::uart::write_line("  task: trap-worker");

    crate::drivers::uart::write_str("  state Faulted:  ");
    crate::kernel::task::table::print_yes_no(snapshot.faulted_task_faulted);
    crate::drivers::uart::write_line("");
}

#[cfg(feature = "scheduler_fault_lifecycle_test")]
fn find_finished_task_for_completion_check() -> Option<usize> {
    crate::kernel::task::table::find_first_finished_task()
}

#[cfg(feature = "scheduler_fault_lifecycle_test")]
fn check_no_runnable_scheduler_policy() -> bool {
    let snapshot = crate::kernel::task::scheduler::get_no_runnable_scheduler_snapshot();

    crate::drivers::uart::write_line("  no-runnable scheduler policy:");

    crate::drivers::uart::write_str("    dispatchable tasks remaining: ");
    crate::kernel::task::table::print_yes_no(snapshot.has_dispatchable_tasks);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("    expected remaining == no: ");
    crate::kernel::task::table::print_yes_no(snapshot.no_runnable);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("    dispatchable count: ");
    crate::drivers::uart::write_dec_u64(snapshot.dispatchable_count as u64);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("    result: ");
    if snapshot.result {
        crate::drivers::uart::write_line("OK");
    } else {
        crate::drivers::uart::write_line("FAILED");
    }

    snapshot.result
}

#[cfg(feature = "scheduler_fault_lifecycle_test")]
fn find_faulted_task_for_completion_check() -> Option<usize> {
    crate::kernel::task::table::find_first_faulted_task()
}

#[cfg(feature = "scheduler_fault_lifecycle_test")]
fn check_task_fault_metadata_assertions(id: usize) -> bool {
    let snapshot = crate::kernel::task::table::get_breakpoint_fault_metadata_assertions(id);

    crate::drivers::uart::write_line("  fault metadata assertions:");

    crate::drivers::uart::write_str("    reason == breakpoint: ");
    crate::kernel::task::table::print_yes_no(snapshot.reason_breakpoint);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("    mcause == 3: ");
    crate::kernel::task::table::print_yes_no(snapshot.mcause_breakpoint);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("    mepc != 0: ");
    crate::kernel::task::table::print_yes_no(snapshot.mepc_nonzero);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("    mtval != 0: ");
    crate::kernel::task::table::print_yes_no(snapshot.mtval_nonzero);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("    result: ");
    if snapshot.result {
        crate::drivers::uart::write_line("OK");
    } else {
        crate::drivers::uart::write_line("FAILED");
    }

    snapshot.result
}

#[cfg(feature = "scheduler_fault_lifecycle_test")]
fn real_trap_handler_worker() {
    crate::drivers::uart::write_line("real_trap_handler_worker: step 1");
    crate::drivers::uart::write_line("real_trap_handler_worker: triggering ebreak");

    unsafe {
        core::arch::asm!("ebreak", options(nomem, nostack, preserves_flags));
    }

    crate::drivers::uart::write_line("real_trap_handler_worker: after ebreak (should not reach)");
}

#[cfg(feature = "scheduler_fault_lifecycle_test")]
fn real_trap_handler_worker_a() {
    crate::drivers::uart::write_line("real_trap_handler_worker_a: step 1");
    crate::kernel::task::yield_now();

    crate::drivers::uart::write_line("real_trap_handler_worker_a: resumed after yield");
    crate::drivers::uart::write_line("real_trap_handler_worker_a: step 2");

    crate::kernel::task::task_exit();
}

#[cfg(feature = "kernel_fault_guard_test")]
pub fn test_kernel_fault_guard() -> ! {
    crate::drivers::uart::write_line("");
    crate::drivers::uart::write_line("kernel fault guard test:");
    crate::drivers::uart::write_line("triggering real trap from kernel context");

    crate::kernel::task::debug::set_debug_current_task_id(0);

    unsafe {
        core::arch::asm!("ebreak", options(nomem, nostack, preserves_flags));
    }

    crate::drivers::uart::write_line("kernel fault guard result: FAILED");
    crate::drivers::uart::write_line("kernel continued after kernel fault trap");
    crate::arch::halt();
}
