#[cfg(feature = "two_task_resume_handoff_test")]
use crate::kernel::cpu::{set_current, set_current_stack_bounds};

#[cfg(feature = "two_task_resume_handoff_test")]
pub fn handoff_worker_a() {
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
pub fn handoff_worker_b() {
    crate::drivers::uart::write_line("handoff_worker_b: step 1");
    crate::kernel::task::yield_now();

    crate::drivers::uart::write_line("handoff_worker_b: resumed after yield");
    crate::drivers::uart::write_line("handoff_worker_b: step 2");
    crate::kernel::task::task_exit();
}

#[cfg(feature = "two_task_resume_handoff_test")]
static mut TWO_TASK_HANDOFF_PHASE: usize = 0;

#[cfg(feature = "two_task_resume_handoff_test")]
pub fn get_two_task_handoff_phase() -> usize {
    unsafe { TWO_TASK_HANDOFF_PHASE }
}

#[cfg(feature = "two_task_resume_handoff_test")]
pub fn advance_two_task_handoff_phase() {
    unsafe {
        TWO_TASK_HANDOFF_PHASE += 1;
    }
}

#[cfg(feature = "two_task_resume_handoff_test")]
pub fn prepare_debug_context_for_task(task_id: usize) {
    let Some(stack_start) = crate::kernel::task::table::get_task_stack_start(task_id) else {
        crate::drivers::uart::write_line("two-task handoff error: missing task stack start");
        crate::arch::halt();
    };

    let Some(stack_top) = crate::kernel::task::table::get_task_stack_top(task_id) else {
        crate::drivers::uart::write_line("two-task handoff error: missing task stack top");
        crate::arch::halt();
    };

    set_current(task_id);
    set_current_stack_bounds(stack_start, stack_top);
}
