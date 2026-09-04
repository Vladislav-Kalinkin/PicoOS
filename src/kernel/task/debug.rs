use crate::drivers::uart;
use crate::kernel::cpu;

#[unsafe(no_mangle)]
pub extern "C" fn task_return_point() -> ! {
    uart::write_line("");
    uart::write_line("task return:");

    uart::write_str("  reason: ");
    crate::kernel::task::table::print_task_return_kind(cpu::task_return_kind());
    uart::write_line("");

    crate::kernel::task::test::handle_task_return_for_debug_test();
    cpu::clear_current();
    #[cfg(feature = "task_sleep_runtime_e2e_test")]
    let return_kind = cpu::task_return_kind();

    match cpu::task_run_stage() {
        #[cfg(feature = "task_yield_test")]
        10 => {
            #[cfg(feature = "task_sleep_runtime_e2e_test")]
            if matches!(
                return_kind,
                crate::kernel::task::table::TaskReturnKind::Sleep
            ) {
                use crate::arch::riscv64::{cpu as hart, timer};

                const SLEEP_TEST_TIMER_HZ: u64 = 1;

                uart::write_line("sleep runtime e2e: task blocked, waiting for timer wake");
                crate::kernel::ticks::reset();
                timer::arm_timer_hz(SLEEP_TEST_TIMER_HZ);
                hart::enable_machine_timer_interrupt();
                crate::arch::enable_irq();

                loop {
                    crate::arch::wait_for_interrupt();
                }
            }

            uart::write_line("back in kernel after yield test");
            uart::write_line("yield test complete");

            #[cfg(feature = "scheduler_reentry_test")]
            {
                crate::kernel::task::test::handle_scheduler_reentry_after_task_return();
            }

            #[cfg(all(
                feature = "resume_candidate_test",
                not(feature = "scheduler_reentry_test")
            ))]
            {
                crate::kernel::task::test_resume_candidate_selection();
            }

            #[cfg(feature = "resume_preflight_test")]
            {
                crate::kernel::task::test::test_resume_preflight_check();
            }

            #[cfg(feature = "resume_dry_run_test")]
            {
                crate::kernel::task::test::test_resume_dry_run();
            }

            #[cfg(feature = "resume_restore_test")]
            {
                crate::kernel::task::test::test_resume_restore();
            }

            #[cfg(feature = "task_sleep_runtime_e2e_test")]
            if matches!(
                return_kind,
                crate::kernel::task::table::TaskReturnKind::Exit
            ) {
                let worker_finished = matches!(
                    crate::kernel::task::table::get_task_state(1),
                    Some(crate::kernel::task::table::TaskState::Finished)
                );

                if worker_finished {
                    uart::write_line("task sleep runtime e2e result: OK");
                } else {
                    uart::write_line("task sleep runtime e2e result: FAILED");
                    crate::arch::halt();
                }
            }

            crate::kernel::task::test::print_final_task_list();
            crate::arch::halt();
        }

        _ => {
            uart::write_line("unknown task return stage");
            crate::arch::halt();
        }
    }
}
