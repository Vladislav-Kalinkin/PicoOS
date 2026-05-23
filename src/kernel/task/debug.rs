use crate::drivers::uart;

static mut DEBUG_KERNEL_RETURN_PC: u64 = 0;
static mut DEBUG_KERNEL_SP_BEFORE_TASK: u64 = 0;
static mut DEBUG_CURRENT_STACK_START: u64 = 0;
static mut DEBUG_CURRENT_STACK_TOP: u64 = 0;
static mut DEBUG_TASK_RUN_STAGE: u64 = 0;
static mut DEBUG_TASK_RETURN_KIND: crate::kernel::task::table::TaskReturnKind =
    crate::kernel::task::table::TaskReturnKind::None;
static mut DEBUG_CURRENT_TASK_ID: usize = 0;
static mut DEBUG_LAST_TASK_SP: u64 = 0;
static mut DEBUG_TASK_RESUME_PC: u64 = 0;


pub fn set_debug_last_task_sp(sp: u64) {
    unsafe {
        DEBUG_LAST_TASK_SP = sp;
    }
}

pub fn debug_last_task_sp() -> u64 {
    unsafe { DEBUG_LAST_TASK_SP }
}
#[allow(dead_code)]
pub fn set_debug_current_stack_bounds(start: u64, top: u64) {
    unsafe {
        DEBUG_CURRENT_STACK_START = start;
        DEBUG_CURRENT_STACK_TOP = top;
    }
}

pub fn debug_current_stack_start() -> u64 {
    unsafe { DEBUG_CURRENT_STACK_START }
}

pub fn debug_current_stack_top() -> u64 {
    unsafe { DEBUG_CURRENT_STACK_TOP }
}

#[allow(dead_code)]
pub fn set_debug_kernel_sp_before_task(sp: u64) {
    unsafe {
        DEBUG_KERNEL_SP_BEFORE_TASK = sp;
    }
}

pub fn debug_kernel_sp_before_task() -> u64 {
    unsafe { DEBUG_KERNEL_SP_BEFORE_TASK }
}

#[allow(dead_code)]
pub fn set_debug_kernel_return_pc(pc: u64) {
    unsafe {
        DEBUG_KERNEL_RETURN_PC = pc;
    }
}

pub fn debug_kernel_return_pc() -> u64 {
    unsafe { DEBUG_KERNEL_RETURN_PC }
}

#[no_mangle]
pub extern "C" fn task_return_point() -> ! {
    uart::write_line("");
    uart::write_line("task return:");

    uart::write_str("  reason: ");
    crate::kernel::task::table::print_task_return_kind(debug_task_return_kind());
    uart::write_line("");

    crate::kernel::task::test::handle_task_return_for_debug_test();

    match debug_task_run_stage() {
        #[cfg(feature = "sequential_task_test")]
        1 => crate::kernel::task::test::continue_sequential_task_test_after_worker_a(),

        #[cfg(feature = "sequential_task_test")]
        2 => {
            uart::write_line("all sequential task tests complete");
            crate::kernel::task::test::print_final_task_list();
            crate::arch::halt();
        }

        #[cfg(feature = "task_yield_test")]
        10 => {
            uart::write_line("back in kernel after yield test");
            uart::write_line("yield test complete");

            #[cfg(feature = "resume_candidate_test")]
            {
                crate::kernel::task::test::test_resume_candidate_selection();
            }

            #[cfg(feature = "resume_preflight_test")]
            {
                crate::kernel::task::test::test_resume_preflight_check();
            }

            #[cfg(feature = "resume_dry_run_test")]
            {
                crate::kernel::task::test::test_resume_dry_run();
            }

            #[cfg(feature = "resume_restore_test")]
            {
                crate::kernel::task::test::test_resume_restore();
            }

            crate::kernel::task::test::print_final_task_list();
            crate::arch::halt();
        }

        #[cfg(feature = "scheduler_driven_task_test")]
        20 => crate::kernel::task::test::continue_scheduler_driven_task_runner(),

        _ => {
            uart::write_line("unknown task return stage");
            crate::arch::halt();
        }
    }
}

#[allow(dead_code)]
pub fn set_debug_task_run_stage(stage: u64) {
    unsafe {
        DEBUG_TASK_RUN_STAGE = stage;
    }
}

pub fn debug_task_run_stage() -> u64 {
    unsafe { DEBUG_TASK_RUN_STAGE }
}

pub fn set_debug_task_return_kind(kind: crate::kernel::task::table::TaskReturnKind) {
    unsafe {
        DEBUG_TASK_RETURN_KIND = kind;
    }
}

pub fn debug_task_return_kind() -> crate::kernel::task::table::TaskReturnKind {
    unsafe { DEBUG_TASK_RETURN_KIND }
}

#[allow(dead_code)]
pub fn set_debug_current_task_id(id: usize) {
    unsafe {
        DEBUG_CURRENT_TASK_ID = id;
    }
}

pub fn debug_current_task_id() -> usize {
    unsafe { DEBUG_CURRENT_TASK_ID }
}

#[allow(dead_code)]
pub fn set_debug_task_resume_pc(pc: u64) {
    unsafe {
        DEBUG_TASK_RESUME_PC = pc;
    }
}

pub fn debug_task_resume_pc() -> u64 {
    unsafe { DEBUG_TASK_RESUME_PC }
}

#[allow(dead_code)]
pub fn set_debug_task_resume_context(task_sp: u64, resume_pc: u64) {
    set_debug_last_task_sp(task_sp);
    set_debug_task_resume_pc(resume_pc);
}

#[allow(dead_code)]
pub fn print_debug_task_resume_context() {
    let task_sp = debug_last_task_sp();
    let resume_pc = debug_task_resume_pc();

    crate::drivers::uart::write_str("yield resume PC: ");
    crate::drivers::uart::write_hex_u64(resume_pc);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("yield current SP: ");
    crate::drivers::uart::write_hex_u64(task_sp);
    crate::drivers::uart::write_line("");
}
