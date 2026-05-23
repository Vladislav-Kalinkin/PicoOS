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

#[no_mangle]
pub extern "C" fn task_trampoline_raw(entry_addr: usize) -> ! {
    let entry: TaskEntry = unsafe { core::mem::transmute(entry_addr) };

    task_trampoline(entry);
}

#[cfg(feature = "task_yield_test")]
pub fn yield_now() {
    let (task_sp, resume_pc) = crate::arch::capture_yield_context();

    uart::write_line("task yield requested");

    let kernel_sp = crate::kernel::task::debug::debug_kernel_sp_before_task();
    let return_pc = crate::kernel::task::debug::debug_kernel_return_pc();

    uart::write_str("yield saved kernel SP: ");
    uart::write_hex_u64(kernel_sp);
    uart::write_line("");

    uart::write_str("yield return PC: ");
    uart::write_hex_u64(return_pc);
    uart::write_line("");

    crate::drivers::uart::write_line("yielding to kernel via arch stub...");

    if !print_returning_yield_task_layer_precheck(task_sp, resume_pc, kernel_sp, return_pc) {
        crate::drivers::uart::write_line("returning yield task-layer precheck failed");
        crate::arch::halt();
    }

    unsafe {
        crate::arch::yield_to_kernel_and_return(task_sp, resume_pc, kernel_sp, return_pc);
    }

    crate::drivers::uart::write_line("yield_now: resumed after arch yield");
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
