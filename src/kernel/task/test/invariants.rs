#[cfg(feature = "task_yield_test")]
use crate::drivers::uart;

#[cfg(feature = "task_yield_test")]
pub fn print_resume_eligibility_check(task_id: usize) {
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
        (Some(crate::kernel::task::table::TaskState::Blocked), Some(false)) => {
            uart::write_line("    task is blocked; resume disabled");
        }
        _ => {
            uart::write_line("    task resume state is inconsistent");
        }
    }
}

#[cfg(feature = "task_yield_test")]
pub fn print_cpu_context_consistency_check(task_id: usize) {
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

#[cfg(feature = "task_yield_test")]
pub fn print_illegal_transition_checks(task_id: usize) {
    use crate::kernel::task::table::TaskLifecycleTransition::{Exit, Fault, Yield};
    use crate::kernel::task::table::TaskState;

    uart::write_line("  lifecycle transition guard check:");

    let Some(state) = crate::kernel::task::table::get_task_state(task_id) else {
        uart::write_line("    state unknown");
        return;
    };

    let yield_allowed = crate::kernel::task::table::can_apply_task_transition(task_id, Yield);
    let exit_allowed = crate::kernel::task::table::can_apply_task_transition(task_id, Exit);
    let fault_allowed = crate::kernel::task::table::can_apply_task_transition(task_id, Fault);

    let guard_ok = match state {
        TaskState::Finished | TaskState::Faulted => {
            !yield_allowed && !exit_allowed && !fault_allowed
        }
        _ => true,
    };

    uart::write_str("    illegal transitions blocked: ");
    if guard_ok {
        uart::write_line("OK");
    } else {
        uart::write_line("FAILED");
    }
}
