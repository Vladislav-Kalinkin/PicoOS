#[cfg(feature = "scheduler_fault_lifecycle_test")]
use crate::drivers::uart;
#[cfg(feature = "two_task_resume_handoff_test")]
use crate::drivers::uart;

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
