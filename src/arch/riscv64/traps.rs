use crate::arch;
use crate::arch::riscv64::cpu;
use crate::arch::riscv64::timer;
use crate::drivers::uart;
use crate::kernel::trap_frame::Riscv64TrapFrame;

const TIMER_HZ: u64 = 1;

#[no_mangle]
pub extern "C" fn riscv64_trap_handler(frame: *const Riscv64TrapFrame) {
    arch::disable_irq();

    let cause = cpu::mcause();
    let is_interrupt = (cause >> 63) != 0;
    let code = cause & 0x7FFF_FFFF_FFFF_FFFF;

    if is_interrupt && code == 7 {
        handle_timer_interrupt(frame);
        arch::enable_irq();
        return;
    }

    uart::write_line("");
    uart::write_line("=== RISC-V TRAP ===");

    uart::write_str("trap frame: ");
    uart::write_hex_u64(frame as u64);
    uart::write_line("");

    uart::write_str("mcause: ");
    uart::write_hex_u64(cause);
    uart::write_line("");

    uart::write_str("mepc: ");
    uart::write_hex_u64(cpu::mepc());
    uart::write_line("");

    uart::write_str("mtval: ");
    uart::write_hex_u64(cpu::mtval());
    uart::write_line("");

    print_trap_cause(cause);

    #[cfg(feature = "real_trap_handler_classification_test")]
    {
        crate::kernel::task::debug::print_trap_execution_context();
        crate::kernel::task::fault::print_current_trap_fault_classification();

        match crate::kernel::task::fault::classify_current_trap_fault() {
            crate::kernel::task::fault::TrapFaultClassification::KernelFault => {
                uart::write_line("trap handler action: kernel fault -> halt");
                arch::halt();
            }
            crate::kernel::task::fault::TrapFaultClassification::TaskFault => {
                uart::write_line("trap handler action: marking current task as Faulted");
                let task_id = crate::kernel::task::debug::debug_current_task_id();

                crate::kernel::task::table::set_task_state(
                    task_id,
                    crate::kernel::task::table::TaskState::Faulted,
                );
                crate::kernel::task::table::set_task_return_kind(
                    task_id,
                    crate::kernel::task::table::TaskReturnKind::Fault,
                );
                crate::kernel::task::table::set_task_can_resume(task_id, false);

                uart::write_str("  task: ");
                crate::kernel::task::table::print_task_name_by_id(task_id);
                uart::write_line("");

                uart::write_str("  new state: ");
                crate::kernel::task::table::print_task_state_by_id(task_id);
                uart::write_line("");

                uart::write_str("  can_resume: ");
                match crate::kernel::task::table::can_task_resume(task_id) {
                    Some(true) => uart::write_line("yes"),
                    Some(false) => uart::write_line("no"),
                    None => uart::write_line("unknown"),
                }

                uart::write_line("trap handler action: returning to kernel stack");

                let current_sp = crate::arch::stack_pointer();
                let kernel_sp = crate::kernel::task::debug::debug_kernel_sp_before_task();
                let return_pc = crate::kernel::task::debug::debug_kernel_return_pc();

                crate::kernel::task::debug::set_debug_last_task_sp(current_sp);
                crate::kernel::task::debug::set_debug_task_return_kind(
                    crate::kernel::task::table::TaskReturnKind::Fault,
                );

                unsafe {
                    crate::arch::return_to_kernel_stack(kernel_sp, return_pc);
                }
            }
        }
    }

    #[cfg(not(feature = "real_trap_handler_classification_test"))]
    {
        uart::write_line("system halted after trap");
        arch::halt();
    }
}

fn handle_timer_interrupt(frame: *const Riscv64TrapFrame) {
    timer::disarm_timer();

    let saved_sp = frame as u64;
    let saved_pc = cpu::mepc();

    let saved_task = crate::kernel::task::scheduler::save_current_context(saved_sp, saved_pc);

    let tick = crate::kernel::ticks::increment();

    uart::write_str("tick: ");
    uart::write_dec_u64(tick);

    uart::write_str(" saved current: ");
    match saved_task {
        Some(id) => {
            crate::kernel::task::scheduler::print_task_name(id);
            crate::kernel::task::print_task_context_values(saved_sp, saved_pc);
        }
        None => uart::write_str("none"),
    }

    uart::write_str(" next task: ");
    crate::kernel::task::scheduler::schedule_next();
    crate::kernel::task::scheduler::print_current_task_name();

    uart::write_str(" context:");
    match crate::kernel::task::scheduler::current_task_id() {
        Some(id) => crate::kernel::task::print_task_full_context_by_id(id),
        None => uart::write_str(" none"),
    }

    uart::write_line("");

    if crate::kernel::ticks::is_test_complete() {
        cpu::disable_machine_timer_interrupt();

        crate::kernel::test::print_test_complete();

        arch::halt();
    }

    timer::arm_timer_hz(TIMER_HZ);
}

fn print_trap_cause(cause: u64) {
    let is_interrupt = (cause >> 63) != 0;
    let code = cause & 0x7FFF_FFFF_FFFF_FFFF;

    uart::write_str("trap type: ");

    if is_interrupt {
        uart::write_line("interrupt");
    } else {
        uart::write_line("exception");
    }

    uart::write_str("trap code: ");
    uart::write_dec_u64(code);
    uart::write_line("");

    uart::write_str("trap name: ");

    match (is_interrupt, code) {
        (false, 0) => uart::write_line("instruction address misaligned"),
        (false, 1) => uart::write_line("instruction access fault"),
        (false, 2) => uart::write_line("illegal instruction"),
        (false, 3) => uart::write_line("breakpoint"),
        (false, 5) => uart::write_line("load access fault"),
        (false, 7) => uart::write_line("store access fault"),
        (false, 8) => uart::write_line("environment call from U-mode"),
        (false, 9) => uart::write_line("environment call from S-mode"),
        (false, 11) => uart::write_line("environment call from M-mode"),
        (true, 3) => uart::write_line("machine software interrupt"),
        (true, 7) => uart::write_line("machine timer interrupt"),
        (true, 11) => uart::write_line("machine external interrupt"),
        _ => uart::write_line("unknown"),
    }
}
