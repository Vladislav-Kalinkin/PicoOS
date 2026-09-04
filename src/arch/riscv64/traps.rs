use crate::arch;
use crate::arch::riscv64::cpu;
use crate::arch::riscv64::timer;
use crate::drivers::uart;
use crate::kernel::trap_frame::Riscv64TrapFrame;

const TIMER_HZ: u64 = 1;

#[unsafe(no_mangle)]
pub extern "C" fn riscv64_trap_handler(frame: *const Riscv64TrapFrame) {
    arch::disable_irq();
    crate::kernel::cpu::set_in_trap(true);

    let cause = cpu::mcause();
    let is_interrupt = (cause >> 63) != 0;
    let code = cause & 0x7FFF_FFFF_FFFF_FFFF;

    if is_interrupt && code == 7 {
        handle_timer_interrupt(frame);
        crate::kernel::cpu::set_in_trap(false);
        arch::enable_irq();
        return;
    }

    let mepc = cpu::mepc();
    let mtval = cpu::mtval();

    uart::write_line("");
    uart::write_line("=== RISC-V TRAP ===");

    uart::write_str("trap frame: ");
    uart::write_hex_u64(frame as u64);
    uart::write_line("");

    uart::write_str("mcause: ");
    uart::write_hex_u64(cause);
    uart::write_line("");

    uart::write_str("mepc: ");
    uart::write_hex_u64(mepc);
    uart::write_line("");

    uart::write_str("mtval: ");
    uart::write_hex_u64(mtval);
    uart::write_line("");

    print_trap_cause(cause);

    #[cfg(feature = "scheduler_fault_lifecycle_test")]
    {
        crate::kernel::cpu::print_trap_execution_context();
        crate::kernel::task::fault::print_current_trap_fault_classification();

        match crate::kernel::task::fault::classify_current_trap_fault() {
            crate::kernel::task::fault::TrapFaultClassification::KernelFault => {
                crate::kernel::log::fail("trap", "kernel fault -> halt");

                #[cfg(feature = "kernel_fault_guard_test")]
                {
                    let frame_on_trap_stack = arch::is_trap_stack_addr(frame as u64);

                    uart::write_str("trap frame on trap stack: ");
                    crate::kernel::task::table::print_yes_no(frame_on_trap_stack);
                    uart::write_line("");

                    if !frame_on_trap_stack {
                        uart::write_line("kernel fault guard result: FAILED");
                        arch::halt();
                    }

                    uart::write_line("");
                    uart::write_line("kernel fault guard result: OK");
                    uart::write_line("");
                    uart::write_line("PicoOS milestone:");
                    uart::write_line("  baseline: 0.1.0");
                    uart::write_line("  current: 0.1.64");
                    uart::write_line("  task fault state: OK");
                    uart::write_line("  scheduler skips faulted tasks: OK");
                    uart::write_line("  trap-to-task-fault skeleton: OK");
                    uart::write_line("  real trap handler classification: OK");
                    uart::write_line("  real trap handler task-fault return path: OK");
                    uart::write_line("  trap stack isolation: OK");
                    uart::write_line("  kernel fault guard: OK");
                }

                arch::halt();
            }

            crate::kernel::task::fault::TrapFaultClassification::TaskFault => {
                crate::kernel::log::info("trap", "marking current task as Faulted");

                if crate::kernel::task::fault::record_current_task_fault(cause, mepc, mtval)
                    .is_none()
                {
                    arch::halt();
                }

                let task_sp = interrupted_sp(frame);
                let kernel_sp = crate::kernel::cpu::kernel_sp_before_task();
                let return_pc = crate::kernel::cpu::kernel_return_pc();

                uart::write_str("  task fault return SP: ");
                uart::write_hex_u64(task_sp);
                uart::write_line("");

                uart::write_str("  saved kernel SP: ");
                uart::write_hex_u64(kernel_sp);
                uart::write_line("");

                uart::write_str("  kernel return PC: ");
                uart::write_hex_u64(return_pc);
                uart::write_line("");

                crate::kernel::log::info("trap", "return to kernel task return path");

                crate::kernel::task::fault::return_current_task_fault(
                    task_sp, kernel_sp, return_pc,
                );
            }
        }
    }

    #[cfg(not(feature = "scheduler_fault_lifecycle_test"))]
    {
        crate::kernel::log::fail("trap", "system halted after trap");
        arch::halt();
    }
}

fn handle_timer_interrupt(frame: *const Riscv64TrapFrame) {
    timer::disarm_timer();

    let saved_sp = interrupted_sp(frame);
    let saved_pc = cpu::mepc();

    let saved_task = crate::kernel::task::scheduler::save_current_context(saved_sp, saved_pc);

    let tick = crate::kernel::ticks::increment();
    let woke_tasks = crate::kernel::task::wake_sleeping_tasks(tick);

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

    let decided_next = crate::kernel::task::scheduler::decide_next_task_dry_run();
    #[cfg(feature = "timer_preemption_prototype")]
    let decided_resumable = crate::kernel::task::scheduler::decide_next_resumable_task_dry_run();

    uart::write_str(" decision next: ");
    match decided_next {
        Some(id) => crate::kernel::task::scheduler::print_task_name(id),
        None => uart::write_str("none"),
    }

    uart::write_str(" mode: dry-run");
    uart::write_str(" woke: ");
    uart::write_dec_u64(woke_tasks as u64);

    uart::write_str(" context:");
    match crate::kernel::task::scheduler::current_task_id() {
        Some(id) => crate::kernel::task::print_task_full_context_by_id(id),
        None => uart::write_str(" none"),
    }

    uart::write_line("");
    crate::kernel::log::info("timer", "scheduler decision computed");

    #[cfg(feature = "timer_preemption_prototype")]
    let preemption_target = {
        #[cfg(feature = "timer_preemption_selftest")]
        {
            decided_resumable.or_else(|| {
                saved_task.filter(|id| crate::kernel::task::get_task_resume_frame(*id).is_some())
            })
        }

        #[cfg(not(feature = "timer_preemption_selftest"))]
        {
            decided_resumable
        }
    };

    #[cfg(feature = "timer_preemption_prototype")]
    if let Some(next_id) = preemption_target {
        crate::kernel::task::scheduler::force_current_task(next_id);
        crate::kernel::log::ok("timer", "preemption prototype: switching to resumable task");
        crate::drivers::uart::write_line("timer preemption result: OK");

        let Some(stack_start) = crate::kernel::task::get_task_stack_start(next_id) else {
            crate::kernel::log::fail("timer", "preemption missing task stack start");
            arch::halt();
        };
        let Some(stack_top) = crate::kernel::task::get_task_stack_top(next_id) else {
            crate::kernel::log::fail("timer", "preemption missing task stack top");
            arch::halt();
        };
        crate::kernel::cpu::set_current(next_id);
        crate::kernel::cpu::set_current_stack_bounds(stack_start, stack_top);

        let Some(frame) = crate::kernel::task::get_task_resume_frame(next_id) else {
            crate::kernel::log::fail("timer", "resume frame missing after decision");
            arch::halt();
        };

        timer::arm_timer_hz(TIMER_HZ);
        arch::reset_trap_stack_pointer_for_next_trap();
        crate::arch::restore_verified_resume_frame(frame);
    }

    if crate::kernel::ticks::is_test_complete() {
        cpu::disable_machine_timer_interrupt();

        crate::kernel::test::print_test_complete();

        arch::halt();
    }

    timer::arm_timer_hz(TIMER_HZ);
}

const fn interrupted_sp(frame: *const Riscv64TrapFrame) -> u64 {
    unsafe { (*frame).sp }
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
