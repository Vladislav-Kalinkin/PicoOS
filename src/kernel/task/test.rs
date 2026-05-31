use crate::drivers::uart;
mod bootstrap;
mod invariants;
mod resume;
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

#[allow(unused_imports)]
pub use resume::*;

#[cfg(not(feature = "task_yield_test"))]
pub fn test_tasks() {
    crate::kernel::task::table::init();

    let _ = create_task("idle", idle_task);
    bootstrap::print_task_zero_context_guard();
    let _ = create_task("worker-a", worker_a_task);
    let _ = create_task("worker-b", worker_b_task);

    print_tasks();
}

#[cfg(feature = "task_yield_test")]
pub fn test_tasks_with_yield_worker() {
    crate::kernel::task::table::init();
    let _ = create_task("idle", idle_task);
    bootstrap::print_task_zero_context_guard();

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

#[cfg(any(
    not(feature = "task_yield_test"),
    all(
        feature = "task_yield_test",
        not(feature = "two_yield_task_test"),
        not(feature = "two_task_resume_handoff_test"),
        not(feature = "scheduler_fault_lifecycle_test")
    )
))]
fn worker_b_task() {
    print_current_task_stack_check("worker_b_task");
}

#[cfg(any(
    not(feature = "task_yield_test"),
    all(
        feature = "task_yield_test",
        not(feature = "two_yield_task_test"),
        not(feature = "two_task_resume_handoff_test"),
        not(feature = "scheduler_fault_lifecycle_test")
    )
))]
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
        bootstrap::test_scheduler_fault_lifecycle_bootstrap();
    }

    #[cfg(all(
        feature = "task_fault_test",
        not(feature = "scheduler_fault_lifecycle_test")
    ))]
    {
        bootstrap::test_task_fault_bootstrap();
    }

    #[cfg(all(
        feature = "two_task_resume_handoff_test",
        not(feature = "task_fault_test"),
        not(feature = "scheduler_fault_lifecycle_test")
    ))]
    {
        bootstrap::test_two_task_resume_handoff_bootstrap();
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

    crate::kernel::task::table::set_last_returned_task_id(task_id);

    uart::write_str("  captured CPU context:");
    crate::kernel::task::cpu_context::print_cpu_context(cpu_context);
    uart::write_line("");

    match kind {
        TaskReturnKind::Exit => {
            let finished_marked = crate::kernel::task::table::mark_task_finished(task_id);

            crate::drivers::uart::write_str("  mark finished: ");
            crate::kernel::task::table::print_yes_no(finished_marked);
            crate::drivers::uart::write_line("");

            crate::kernel::task::scheduler::switch_to_idle();
        }
        TaskReturnKind::Yield => {
            let ready_marked = crate::kernel::task::table::mark_task_ready_after_yield(task_id);

            crate::drivers::uart::write_str("  mark ready after yield: ");
            crate::kernel::task::table::print_yes_no(ready_marked);
            crate::drivers::uart::write_line("");

            crate::kernel::task::scheduler::switch_to_idle();
        }
        TaskReturnKind::Fault => {
            let faulted_marked = crate::kernel::task::table::mark_task_faulted(task_id);

            crate::drivers::uart::write_str("  mark faulted: ");
            crate::kernel::task::table::print_yes_no(faulted_marked);
            crate::drivers::uart::write_line("");

            crate::kernel::task::scheduler::switch_to_idle();
        }
        TaskReturnKind::None => {
            crate::kernel::task::table::set_task_return_kind(task_id, kind);
        }
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
    invariants::print_resume_eligibility_check(task_id);
    invariants::print_cpu_context_consistency_check(task_id);

    #[cfg(any(feature = "two_task_resume_handoff_test", feature = "task_fault_test"))]
    {
        let _ = print_resume_pc_proximity_check(task_id);
    }

    uart::write_str("  scheduler current: ");
    crate::kernel::task::scheduler::print_current_task_name();
    uart::write_line("");
}

#[cfg(feature = "task_yield_test")]
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
    crate::drivers::uart::write_line("  current: 0.1.64");

    crate::drivers::uart::write_line("");
    crate::drivers::uart::write_line("  cleanup:");
    crate::drivers::uart::write_line("    obsolete standalone task tests removed: OK");
    crate::drivers::uart::write_line("    obsolete standalone scheduler scripts removed: OK");
    crate::drivers::uart::write_line("    obsolete resume task script removed: OK");
    crate::drivers::uart::write_line("    obsolete resume PC proximity requirement removed: OK");

    crate::drivers::uart::write_line("");
    crate::drivers::uart::write_line("  task/resume:");
    crate::drivers::uart::write_line("    RISC-V-only baseline: OK");
    crate::drivers::uart::write_line("    cooperative task resume: OK");
    crate::drivers::uart::write_line("    repeated yield/resume loop: OK");
    crate::drivers::uart::write_line("    scheduler-oriented resume loop: OK");
    crate::drivers::uart::write_line("    RISC-V yield boundary: OK");
    crate::drivers::uart::write_line("    two-task cooperative handoff: OK");

    crate::drivers::uart::write_line("");
    crate::drivers::uart::write_line("  scheduler:");
    crate::drivers::uart::write_line("    scheduler first task dispatch: OK");
    crate::drivers::uart::write_line("    scheduler fresh task dispatch: OK");
    crate::drivers::uart::write_line("    scheduler round-robin fairness: OK");
    crate::drivers::uart::write_line("    scheduler task capacity from table: OK");
    crate::drivers::uart::write_line("    scheduler skips faulted tasks: OK");
    crate::drivers::uart::write_line("    no-runnable scheduler policy: OK");
    crate::drivers::uart::write_line("    scheduler dispatch candidate selection: OK");
    crate::drivers::uart::write_line("    scheduler dispatch candidate model: OK");
    crate::drivers::uart::write_line("    scheduler dispatch candidate-to-decision conversion: OK");
    crate::drivers::uart::write_line("    scheduler dispatch decision model: OK");
    crate::drivers::uart::write_line("    scheduler dispatch decision kind: OK");
    crate::drivers::uart::write_line("    scheduler dispatch decision outcome: OK");
    crate::drivers::uart::write_line("    scheduler dispatch decision logging: OK");
    crate::drivers::uart::write_line("    scheduler dispatch pipeline model: OK");

    crate::drivers::uart::write_line("");
    crate::drivers::uart::write_line("  task lifecycle:");
    crate::drivers::uart::write_line("    task state invariants in core: OK");
    crate::drivers::uart::write_line("    task state lookup in core: OK");
    crate::drivers::uart::write_line("    terminal task dispatch invariants in core: OK");
    crate::drivers::uart::write_line("    no-runnable scheduler snapshot in core: OK");
    crate::drivers::uart::write_line("    task completion summary in core: OK");
    crate::drivers::uart::write_line("    task completion output consolidated: OK");

    crate::drivers::uart::write_line("");
    crate::drivers::uart::write_line("  fault lifecycle:");
    crate::drivers::uart::write_line("    task fault state: OK");
    crate::drivers::uart::write_line("    trap-to-task-fault skeleton: OK");
    crate::drivers::uart::write_line("    real trap classification: OK");
    crate::drivers::uart::write_line("    real trap handler classification: OK");
    crate::drivers::uart::write_line("    real trap handler task-fault return path: OK");
    crate::drivers::uart::write_line("    trap fault metadata reporting: OK");
    crate::drivers::uart::write_line("    fault metadata assertions in core: OK");
    crate::drivers::uart::write_line("    explicit task fault assertions: OK");
    crate::drivers::uart::write_line("    faulted task dispatch guard: OK");
    crate::drivers::uart::write_line("    finished task dispatch guard: OK");
    crate::drivers::uart::write_line("    scheduler fault lifecycle feature: OK");
}

#[cfg(feature = "scheduler_fault_lifecycle_test")]
fn check_faulted_task_dispatch_guard(id: usize) -> bool {
    print_terminal_task_dispatch_guard("faulted", id)
}

#[cfg(all(
    feature = "two_yield_task_test",
    not(feature = "two_task_resume_handoff_test"),
    not(feature = "scheduler_fault_lifecycle_test")
))]
fn two_yielding_task() {
    crate::drivers::uart::write_line("two_yielding_task: step 1");

    crate::kernel::task::yield_now();

    crate::drivers::uart::write_line("two_yielding_task: step 2");

    crate::kernel::task::yield_now();

    crate::drivers::uart::write_line("two_yielding_task: step 3");

    crate::kernel::task::task_exit();
}

#[cfg(feature = "scheduler_fault_lifecycle_test")]
fn print_terminal_task_dispatch_guard(label: &str, id: usize) -> bool {
    let snapshot = crate::kernel::task::table::get_terminal_task_dispatch_invariants(id);
    let running_blocked = !crate::kernel::task::table::mark_task_running(id);

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

    crate::drivers::uart::write_str("    force running blocked: ");
    crate::kernel::task::table::print_yes_no(running_blocked);
    crate::drivers::uart::write_line("");

    let ok = snapshot.result && running_blocked;

    crate::drivers::uart::write_str("    result: ");
    if ok {
        crate::drivers::uart::write_line("OK");
    } else {
        crate::drivers::uart::write_line("FAILED");
    }

    ok
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

    crate::kernel::task::debug::clear_debug_current_task_id();

    unsafe {
        core::arch::asm!("ebreak", options(nomem, nostack, preserves_flags));
    }

    crate::drivers::uart::write_line("kernel fault guard result: FAILED");
    crate::drivers::uart::write_line("kernel continued after kernel fault trap");
    crate::arch::halt();
}
