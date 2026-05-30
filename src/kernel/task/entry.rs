use crate::drivers::uart;
use crate::kernel::task::debug::{
    debug_kernel_return_pc, debug_kernel_sp_before_task, set_debug_last_task_sp,
    set_debug_task_return_kind,
};
use crate::kernel::task::table::TaskEntry;
use crate::kernel::task::table::TaskReturnKind;

pub fn task_trampoline(entry: TaskEntry) -> ! {
    uart::write_line("");
    uart::write_line("task trampoline:");
    uart::write_str("calling entry: ");
    uart::write_hex_u64(entry as usize as u64);
    uart::write_line("");

    entry();

    task_exit();
}

#[no_mangle]
pub extern "C" fn task_trampoline_raw(entry_addr: usize) -> ! {
    let entry: TaskEntry = unsafe { core::mem::transmute(entry_addr) };

    task_trampoline(entry);
}

pub fn task_exit() -> ! {
    uart::write_line("task returned; task_exit called");

    let current_sp = crate::arch::stack_pointer();
    let kernel_sp = debug_kernel_sp_before_task();
    let return_pc = debug_kernel_return_pc();

    set_debug_last_task_sp(current_sp);

    uart::write_str("task_exit current SP: ");
    uart::write_hex_u64(current_sp);
    uart::write_line("");

    uart::write_str("task_exit saved kernel SP: ");
    uart::write_hex_u64(kernel_sp);
    uart::write_line("");

    uart::write_str("task_exit return PC: ");
    uart::write_hex_u64(return_pc);
    uart::write_line("");

    uart::write_line("returning to kernel stack...");

    set_debug_task_return_kind(TaskReturnKind::Exit);

    unsafe {
        crate::arch::return_to_kernel_stack(kernel_sp, return_pc);
    }
}

#[allow(dead_code)]
pub fn task_fault() -> ! {
    uart::write_line("task fault requested");

    let current_sp = crate::arch::stack_pointer();
    let kernel_sp = debug_kernel_sp_before_task();
    let return_pc = debug_kernel_return_pc();

    set_debug_last_task_sp(current_sp);

    uart::write_str("task_fault current SP: ");
    uart::write_hex_u64(current_sp);
    uart::write_line("");

    uart::write_str("task_fault saved kernel SP: ");
    uart::write_hex_u64(kernel_sp);
    uart::write_line("");

    uart::write_str("task_fault return PC: ");
    uart::write_hex_u64(return_pc);
    uart::write_line("");

    uart::write_line("returning to kernel stack after task fault...");

    set_debug_task_return_kind(TaskReturnKind::Fault);

    unsafe {
        crate::arch::return_to_kernel_stack(kernel_sp, return_pc);
    }
}

#[allow(dead_code)]
pub fn simulated_task_trap_fault() -> ! {
    uart::write_line("simulated task trap fault requested");

    let current_sp = crate::arch::stack_pointer();
    let kernel_sp = crate::kernel::task::debug::debug_kernel_sp_before_task();
    let return_pc = crate::kernel::task::debug::debug_kernel_return_pc();

    crate::kernel::task::debug::set_debug_last_task_sp(current_sp);

    uart::write_str("simulated trap current SP: ");
    uart::write_hex_u64(current_sp);
    uart::write_line("");

    uart::write_str("simulated trap saved kernel SP: ");
    uart::write_hex_u64(kernel_sp);
    uart::write_line("");

    uart::write_str("simulated trap return PC: ");
    uart::write_hex_u64(return_pc);
    uart::write_line("");

    uart::write_line("simulated trap classified as task fault");
    uart::write_line("returning to kernel stack after simulated task trap...");

    crate::kernel::task::debug::set_debug_task_return_kind(
        crate::kernel::task::table::TaskReturnKind::Fault,
    );

    unsafe {
        crate::arch::return_to_kernel_stack(kernel_sp, return_pc);
    }
}

#[allow(dead_code)]
pub fn yield_now() {
    crate::drivers::uart::write_line("task yield requested");

    let kernel_sp = crate::kernel::task::debug::debug_kernel_sp_before_task();
    let return_pc = crate::kernel::task::debug::debug_kernel_return_pc();

    crate::drivers::uart::write_str("yield saved kernel SP: ");
    crate::drivers::uart::write_hex_u64(kernel_sp);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("yield return PC: ");
    crate::drivers::uart::write_hex_u64(return_pc);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_line("yielding to kernel via RISC-V boundary...");

    unsafe {
        crate::arch::task_yield_boundary(kernel_sp, return_pc);
    }
}

#[allow(dead_code)]
fn print_returning_yield_task_layer_precheck(
    task_sp: u64,
    resume_pc: u64,
    kernel_sp: u64,
    return_pc: u64,
) -> bool {
    crate::drivers::uart::write_line("returning yield task-layer precheck:");

    let task_id = crate::kernel::task::debug::debug_current_task_id();

    let task_sp_inside_stack = matches!(
        crate::kernel::task::table::is_sp_inside_task_stack(task_id, task_sp),
        Some(true)
    );

    let resume_pc_inside_text = crate::kernel::memory::is_inside_kernel_text(resume_pc);
    let kernel_sp_nonzero = kernel_sp != 0;
    let return_pc_inside_text = crate::kernel::memory::is_inside_kernel_text(return_pc);

    crate::drivers::uart::write_str("  task: ");
    crate::kernel::task::table::print_task_name_by_id(task_id);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("  task_sp inside current task stack: ");
    crate::kernel::task::table::print_yes_no(task_sp_inside_stack);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("  resume_pc inside kernel text: ");
    crate::kernel::task::table::print_yes_no(resume_pc_inside_text);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("  kernel_sp non-zero: ");
    crate::kernel::task::table::print_yes_no(kernel_sp_nonzero);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("  return_pc inside kernel text: ");
    crate::kernel::task::table::print_yes_no(return_pc_inside_text);
    crate::drivers::uart::write_line("");

    let ok =
        task_sp_inside_stack && resume_pc_inside_text && kernel_sp_nonzero && return_pc_inside_text;

    crate::drivers::uart::write_str("  result: ");
    if ok {
        crate::drivers::uart::write_line("OK");
    } else {
        crate::drivers::uart::write_line("FAILED");
    }

    ok
}

#[allow(dead_code)]
pub fn simulated_real_trap_fault() -> ! {
    uart::write_line("simulated real trap fault requested");

    crate::kernel::task::debug::print_trap_execution_context();

    crate::kernel::task::fault::print_current_trap_fault_classification();

    match crate::kernel::task::fault::classify_current_trap_fault() {
        crate::kernel::task::fault::TrapFaultClassification::KernelFault => {
            uart::write_line("simulated real trap result: kernel fault");

            uart::write_line("kernel fault action: halt");

            crate::arch::halt();
        }

        crate::kernel::task::fault::TrapFaultClassification::TaskFault => {
            uart::write_line("simulated real trap result: task fault");

            let task_id = crate::kernel::task::debug::debug_current_task_id();

            // === ЗАЩИТА ОТ INFINITE TRAP LOOP ===
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

            // === НОВОЕ: читаем регистры CPU и сохраняем причину fault ===
            // В симуляции mcause/mepc/mtval могут быть нулевыми или синтетическими,
            // но мы обязаны пройти тот же путь, что и реальный trap handler.
            let fault_mcause = crate::arch::cpu::mcause();
            let fault_mepc = crate::arch::cpu::mepc();
            let fault_mtval = crate::arch::cpu::mtval();

            let Some(fault_reason) = crate::kernel::task::table::record_task_fault(
                task_id,
                fault_mcause,
                fault_mepc,
                fault_mtval,
            ) else {
                uart::write_line("  record task fault: FAILED");
                crate::arch::halt();
            };

            // Печатаем детали fault
            uart::write_str("  fault reason: ");
            crate::kernel::task::table::print_task_fault_reason(fault_reason);
            uart::write_line("");
            uart::write_str("  fault mcause: ");
            uart::write_hex_u64(fault_mcause);
            uart::write_line("");
            uart::write_str("  fault mepc:   ");
            uart::write_hex_u64(fault_mepc);
            uart::write_line("");
            uart::write_str("  fault mtval:  ");
            uart::write_hex_u64(fault_mtval);
            uart::write_line("");

            uart::write_line("  record task fault: OK");

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

            let current_sp = crate::arch::stack_pointer();
            let kernel_sp = crate::kernel::task::debug::debug_kernel_sp_before_task();
            let return_pc = crate::kernel::task::debug::debug_kernel_return_pc();
            crate::kernel::task::debug::set_debug_last_task_sp(current_sp);

            uart::write_str("simulated real trap current SP: ");
            uart::write_hex_u64(current_sp);
            uart::write_line("");
            uart::write_str("simulated real trap saved kernel SP: ");
            uart::write_hex_u64(kernel_sp);
            uart::write_line("");
            uart::write_str("simulated real trap return PC: ");
            uart::write_hex_u64(return_pc);
            uart::write_line("");
            uart::write_line("simulated real trap classified as task fault");
            uart::write_line("returning to kernel stack after simulated real trap...");

            crate::kernel::task::debug::set_debug_task_return_kind(
                crate::kernel::task::table::TaskReturnKind::Fault,
            );

            unsafe {
                crate::arch::return_to_kernel_stack(kernel_sp, return_pc);
            }
        }
    }
}
