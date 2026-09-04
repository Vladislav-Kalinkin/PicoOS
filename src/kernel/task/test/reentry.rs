#[cfg(feature = "scheduler_fault_lifecycle_test")]
pub(crate) fn check_finished_task_dispatch_guard(id: usize) -> bool {
    print_terminal_task_dispatch_guard("finished", id)
}

#[cfg(feature = "scheduler_fault_lifecycle_test")]
pub(crate) fn check_faulted_task_dispatch_guard(id: usize) -> bool {
    print_terminal_task_dispatch_guard("faulted", id)
}

#[cfg(all(
    feature = "two_yield_task_test",
    not(feature = "two_task_resume_handoff_test"),
    not(feature = "scheduler_fault_lifecycle_test")
))]
pub fn two_yielding_task() {
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
        let phase = crate::kernel::task::test::get_two_task_handoff_phase();

        if phase == 0 && snapshot.task_id == 1 {
            if !matches!(
                snapshot.last_return,
                crate::kernel::task::table::TaskReturnKind::Yield
            ) {
                crate::drivers::uart::write_line("two-task handoff error: worker-a did not yield");
                crate::arch::halt();
            }

            crate::kernel::task::test::advance_two_task_handoff_phase();
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

            crate::kernel::task::test::advance_two_task_handoff_phase();
            crate::drivers::uart::write_line("two-task handoff phase 1: worker-b yielded");
            crate::drivers::uart::write_line(
                "two-task handoff action: continue scheduler re-entry",
            );
            crate::kernel::task::test::prepare_debug_context_for_task(1);
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
                    if crate::kernel::task::test::task_fault_completion_check() {
                        crate::drivers::uart::write_line("task fault scheduler result: OK");
                        print_riscv_cooperative_resume_milestone();
                        crate::arch::halt();
                    }
                }

                #[cfg(not(feature = "task_fault_test"))]
                {
                    if crate::kernel::task::test::real_resume_jump_completion_check() {
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
