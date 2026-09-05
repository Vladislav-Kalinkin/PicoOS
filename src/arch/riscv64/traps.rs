use crate::arch;
use crate::arch::riscv64::cpu;
use crate::arch::riscv64::timer;
use crate::drivers::uart;
use crate::kernel::trap_frame::{Riscv64TrapFrame, TrapImage};

const TIMER_HZ: u64 = 1;

#[unsafe(no_mangle)]
pub extern "C" fn riscv64_trap_handler(frame: *const Riscv64TrapFrame) {
    arch::disable_irq();

    // SAFETY: `trap.S` stored a full `Riscv64TrapFrame` on the trap stack and
    // passed that address to this handler.
    let frame = unsafe { &*frame };

    let cause = cpu::mcause();
    let is_interrupt = (cause >> 63) != 0;
    let code = cause & 0x7FFF_FFFF_FFFF_FFFF;

    if is_interrupt && code == 7 {
        handle_timer_interrupt(frame);
    }

    if !is_interrupt && code == 8 {
        crate::kernel::sys::handle_ecall(frame);
        return;
    }

    let mepc = cpu::mepc();
    let mtval = cpu::mtval();

    uart::write_line("");
    uart::write_line("=== RISC-V TRAP ===");

    uart::write_str("trap frame: ");
    uart::write_hex_u64(core::ptr::from_ref(frame) as u64);
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

    match crate::kernel::task::fault::classify_current_trap_fault() {
        crate::kernel::task::fault::TrapFaultClassification::TaskFault => {
            crate::kernel::task::fault::record_and_switch_user_fault(cause, mepc, mtval);
        }
        crate::kernel::task::fault::TrapFaultClassification::KernelFault => {
            #[cfg(feature = "scenario_kernel_fault")]
            {
                crate::kernel::cpu::print_trap_execution_context();
                crate::kernel::task::fault::print_current_trap_fault_classification();
                crate::kernel::log::fail("trap", "kernel fault -> halt");

                let frame_on_trap_stack =
                    arch::is_trap_stack_addr(core::ptr::from_ref(frame) as u64);
                uart::write_str("trap frame on trap stack: ");
                crate::kernel::task::table::print_yes_no(frame_on_trap_stack);
                uart::write_line("");

                if !frame_on_trap_stack {
                    uart::write_line("kernel fault guard result: FAILED");
                    arch::halt();
                }

                uart::write_line("kernel fault guard result: OK");
                arch::halt();
            }

            #[cfg(not(feature = "scenario_kernel_fault"))]
            {
                crate::kernel::log::fail("trap", "system halted after trap");
                arch::halt();
            }
        }
    }
}

fn handle_timer_interrupt(frame: &Riscv64TrapFrame) -> ! {
    timer::disarm_timer();

    let saved_sp = frame.sp;
    let saved_pc = cpu::mepc();
    let saved_mstatus = cpu::mstatus();

    let interrupted_worker = crate::kernel::cpu::current();
    if let Some(id) = interrupted_worker {
        let image = TrapImage::from_frame(frame, saved_pc, saved_mstatus);
        let _ = crate::kernel::task::table::save_preempted_trap_image(id, &image);
    }

    let tick = crate::kernel::ticks::increment();
    let woke_tasks = crate::kernel::task::wake_sleeping_tasks(tick);

    uart::write_str("tick: ");
    uart::write_dec_u64(tick);

    uart::write_str(" saved current: ");
    match interrupted_worker {
        Some(id) => {
            crate::kernel::task::scheduler::print_task_name(id);
            crate::kernel::task::print_task_context_values(saved_sp, saved_pc);
        }
        None => uart::write_str("none"),
    }

    let next = crate::kernel::task::scheduler::next_after(interrupted_worker);

    uart::write_str(" decision next: ");
    match next {
        Some(id) => crate::kernel::task::scheduler::print_task_name(id),
        None => uart::write_str("idle"),
    }

    uart::write_str(" mode: mret");
    uart::write_str(" woke: ");
    uart::write_dec_u64(woke_tasks as u64);

    uart::write_str(" context:");
    match crate::kernel::task::scheduler::current_task_id() {
        Some(id) => crate::kernel::task::print_task_full_context_by_id(id),
        None => uart::write_str(" none"),
    }

    uart::write_line("");
    crate::kernel::log::info("timer", "scheduler decision computed");

    timer::arm_timer_hz(TIMER_HZ);

    #[cfg(feature = "scenario_preempt")]
    if next.is_some() {
        crate::kernel::log::ok("timer", "preemption: mret to worker");
        uart::write_line("timer preemption result: OK");
    }

    crate::kernel::task::scheduler::switch_to(next);
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
