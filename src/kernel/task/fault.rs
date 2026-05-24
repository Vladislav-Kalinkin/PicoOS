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
