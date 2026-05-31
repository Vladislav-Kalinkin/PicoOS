#[cfg(feature = "scheduler_fault_lifecycle_test")]
use crate::kernel::task::test::{
    check_faulted_task_dispatch_guard, check_finished_task_dispatch_guard,
};

#[cfg(feature = "task_fault_test")]
pub fn faulty_worker() {
    crate::drivers::uart::write_line("faulty_worker: step 1");
    crate::drivers::uart::write_line("faulty_worker: intentional fault");
    crate::kernel::task::task_fault();
}

#[cfg(feature = "task_fault_test")]
pub fn task_fault_completion_check() -> bool {
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
pub fn print_task_fault_completion_snapshot(
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
pub fn find_finished_task_for_completion_check() -> Option<usize> {
    crate::kernel::task::table::find_first_finished_task()
}

#[cfg(feature = "scheduler_fault_lifecycle_test")]
pub fn check_no_runnable_scheduler_policy() -> bool {
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
pub fn find_faulted_task_for_completion_check() -> Option<usize> {
    crate::kernel::task::table::find_first_faulted_task()
}

#[cfg(feature = "scheduler_fault_lifecycle_test")]
pub fn check_task_fault_metadata_assertions(id: usize) -> bool {
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
pub fn real_trap_handler_worker() {
    crate::drivers::uart::write_line("real_trap_handler_worker: step 1");
    crate::drivers::uart::write_line("real_trap_handler_worker: triggering ebreak");

    unsafe {
        core::arch::asm!("ebreak", options(nomem, nostack, preserves_flags));
    }

    crate::drivers::uart::write_line("real_trap_handler_worker: after ebreak (should not reach)");
}

#[cfg(feature = "scheduler_fault_lifecycle_test")]
pub fn real_trap_handler_worker_a() {
    crate::drivers::uart::write_line("real_trap_handler_worker_a: step 1");
    crate::kernel::task::yield_now();

    crate::drivers::uart::write_line("real_trap_handler_worker_a: resumed after yield");
    crate::drivers::uart::write_line("real_trap_handler_worker_a: step 2");

    crate::kernel::task::task_exit();
}
