use crate::drivers::uart;
mod bootstrap;
mod fault;
mod handoff;
mod invariants;
mod reentry;
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
    print_tasks, TaskReturnContext, TaskReturnKind,
};

#[allow(unused_imports)]
pub use fault::*;
#[allow(unused_imports)]
pub use handoff::*;
#[allow(unused_imports)]
pub use reentry::*;
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

    #[cfg(feature = "task_sleep_test")]
    {
        bootstrap::test_task_sleep_wakeup_table_selftest();
    }
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
    crate::kernel::task::table::mark_task_started(task_id);
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

    let return_context = TaskReturnContext {
        task_sp,
        kernel_sp,
        kernel_return_pc,
    };

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

    uart::write_str("  task: ");
    print_task_name_by_id(task_id);
    uart::write_line("");

    uart::write_str("  captured CPU context:");
    crate::kernel::task::cpu_context::print_cpu_context(cpu_context);
    uart::write_line("");

    let transition_ok = crate::kernel::task::table::apply_task_return_transition(
        task_id,
        kind,
        return_context,
        cpu_context,
    );

    match kind {
        TaskReturnKind::Exit => crate::drivers::uart::write_str("  mark finished: "),
        TaskReturnKind::Yield => crate::drivers::uart::write_str("  mark ready after yield: "),
        TaskReturnKind::Sleep => crate::drivers::uart::write_str("  mark blocked for sleep: "),
        TaskReturnKind::Fault => crate::drivers::uart::write_str("  mark faulted: "),
        TaskReturnKind::None => crate::drivers::uart::write_str("  set return kind: "),
    }
    crate::kernel::task::table::print_yes_no(transition_ok);
    crate::drivers::uart::write_line("");

    if !matches!(kind, TaskReturnKind::None) {
        crate::kernel::task::scheduler::switch_to_idle();
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
    invariants::print_illegal_transition_checks(task_id);

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
