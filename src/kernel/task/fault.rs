use crate::drivers::uart;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TrapFaultClassification {
    KernelFault,
    TaskFault,
}

#[allow(dead_code)]
pub fn classify_current_trap_fault() -> TrapFaultClassification {
    match crate::kernel::task::debug::current_trap_execution_context() {
        crate::kernel::task::debug::TrapExecutionContext::Kernel => {
            TrapFaultClassification::KernelFault
        }

        crate::kernel::task::debug::TrapExecutionContext::Task => {
            TrapFaultClassification::TaskFault
        }
    }
}

#[allow(dead_code)]
pub fn print_current_trap_fault_classification() {
    uart::write_line("trap fault classification:");

    match classify_current_trap_fault() {
        TrapFaultClassification::KernelFault => {
            uart::write_line("  context: kernel");
            uart::write_line("  classification: kernel fault");
            uart::write_line("  action: halt");
        }

        TrapFaultClassification::TaskFault => {
            uart::write_line("  context: task");
            uart::write_line("  classification: task fault");
            uart::write_line("  action: mark current task faulted and return to scheduler");
        }
    }
}

#[allow(dead_code)]
pub fn record_current_task_fault(mcause: u64, mepc: u64, mtval: u64) -> Option<usize> {
    let task_id = crate::kernel::task::debug::debug_current_task_id();

    if matches!(
        crate::kernel::task::table::get_task_state(task_id),
        Some(crate::kernel::task::table::TaskState::Faulted)
    ) {
        uart::write_line("!!! DOUBLE TRAP DETECTED (simulated) !!!");
        uart::write_str("  task already Faulted: ");
        crate::kernel::task::table::print_task_name_by_id(task_id);
        uart::write_line("");
        uart::write_line("  system halted to prevent infinite trap loop");
        crate::arch::halt();
    }

    let Some(fault_reason) =
        crate::kernel::task::table::record_task_fault(task_id, mcause, mepc, mtval)
    else {
        uart::write_line("  record task fault: FAILED");
        return None;
    };

    uart::write_str("  fault reason: ");
    crate::kernel::task::table::print_task_fault_reason(fault_reason);
    uart::write_line("");

    uart::write_str("  fault mcause: ");
    uart::write_hex_u64(mcause);
    uart::write_line("");

    uart::write_str("  fault mepc:   ");
    uart::write_hex_u64(mepc);
    uart::write_line("");

    uart::write_str("  fault mtval:  ");
    uart::write_hex_u64(mtval);
    uart::write_line("");

    uart::write_line("  record task fault: OK");

    Some(task_id)
}

#[allow(dead_code)]
pub fn return_current_task_fault(task_sp: u64, kernel_sp: u64, return_pc: u64) -> ! {
    crate::kernel::task::debug::set_debug_last_task_sp(task_sp);
    crate::kernel::task::debug::set_debug_task_return_kind(
        crate::kernel::task::table::TaskReturnKind::Fault,
    );

    crate::arch::return_to_kernel_stack_checked(kernel_sp, return_pc);
}
