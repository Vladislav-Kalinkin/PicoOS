use crate::drivers::uart;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TrapFaultClassification {
    KernelFault,
    TaskFault,
}

pub fn classify_current_trap_fault() -> TrapFaultClassification {
    use crate::arch;
    use crate::arch::riscv64::cpu as hart;
    use crate::kernel::cpu;

    if cpu::current().is_none() {
        return TrapFaultClassification::KernelFault;
    }

    if hart::trapped_from_user() {
        return TrapFaultClassification::TaskFault;
    }

    let mtval = hart::mtval();
    if arch::is_trap_stack_addr(mtval) || arch::is_kernel_stack_addr(mtval) {
        return TrapFaultClassification::KernelFault;
    }

    TrapFaultClassification::TaskFault
}

#[cfg(feature = "scenario_kernel_fault")]
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

pub fn record_current_task_fault(mcause: u64, mepc: u64, mtval: u64) -> Option<usize> {
    let Some(task_id) = crate::kernel::cpu::current() else {
        crate::kernel::log::fail("fault", "record task fault with no current task");
        return None;
    };

    if matches!(
        crate::kernel::task::table::get_task_state(task_id),
        Some(crate::kernel::task::table::TaskState::Faulted)
    ) {
        crate::kernel::log::fail("fault", "DOUBLE TRAP DETECTED (simulated)");
        uart::write_str("  task already Faulted: ");
        crate::kernel::task::table::print_task_name_by_id(task_id);
        uart::write_line("");
        uart::write_line("  system halted to prevent infinite trap loop");
        crate::arch::halt();
    }

    let Some(fault_reason) = crate::kernel::task::table::record_task_fault(task_id, mcause) else {
        crate::kernel::log::fail("fault", "record task fault: FAILED");
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

    crate::kernel::log::ok("fault", "record task fault: OK");

    Some(task_id)
}

/// U-mode fault: mark the current frame Faulted and `mret` to the next
/// worker (or idle-exit). Used by the default image PMP-deny path.
pub fn record_and_switch_user_fault(mcause: u64, mepc: u64, mtval: u64) -> ! {
    crate::kernel::log::info("trap", "marking current task as Faulted");

    if record_current_task_fault(mcause, mepc, mtval).is_none() {
        crate::arch::halt();
    }

    if matches!(
        crate::kernel::task::table::TaskFaultReason::from_mcause(mcause & 0x7FFF_FFFF_FFFF_FFFF),
        crate::kernel::task::table::TaskFaultReason::StoreAccessFault
            | crate::kernel::task::table::TaskFaultReason::LoadAccessFault
    ) {
        crate::drivers::uart::write_line("pmp deny: task fault OK");
    }

    crate::kernel::task::scheduler::note_default_image_return(
        crate::kernel::task::table::TaskReturnKind::Fault,
    );

    let after = crate::kernel::cpu::current();
    crate::kernel::cpu::clear_current();
    crate::kernel::task::scheduler::switch_after(after);
}
