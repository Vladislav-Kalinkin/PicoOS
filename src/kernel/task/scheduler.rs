use crate::drivers::uart;
#[cfg(feature = "scheduler_dispatch_test")]
use crate::kernel::task::cpu_context::TaskCpuContext;
use crate::kernel::task::table as task;

static mut CURRENT_TASK_ID: Option<usize> = None;

pub fn init() {
    unsafe {
        CURRENT_TASK_ID = None;
    }

    uart::write_line("");
    uart::write_line("scheduler:");

    schedule_next();

    uart::write_str("current task: ");
    print_current_task_name();

    uart::write_str(" entry: ");
    print_current_task_entry();

    uart::write_str(" context:");
    match current_task_id() {
        Some(id) => task::print_task_full_context_by_id(id),
        None => uart::write_str(" none"),
    }

    uart::write_line("");
}

pub fn schedule_next() -> Option<usize> {
    let current = unsafe { CURRENT_TASK_ID };
    let next = task::find_next_ready_after(current)?;

    task::set_running(next);

    unsafe {
        CURRENT_TASK_ID = Some(next);
    }

    Some(next)
}

pub fn current_task_id() -> Option<usize> {
    unsafe { CURRENT_TASK_ID }
}

pub fn print_current_task_name() {
    match current_task_id() {
        Some(id) => task::print_task_name_by_id(id),
        None => uart::write_str("none"),
    }
}

pub fn print_current_task_entry() {
    match current_task_id() {
        Some(id) => task::print_task_entry_by_id(id),
        None => uart::write_str("none"),
    }
}

#[allow(dead_code)]
pub fn on_timer_tick(tick: u64) {
    let next = schedule_next();

    uart::write_str("tick: ");
    uart::write_dec_u64(tick);

    uart::write_str(" task: ");

    match next {
        Some(id) => {
            task::print_task_name_by_id(id);

            uart::write_str(" entry: ");
            task::print_task_entry_by_id(id);

            uart::write_str(" context:");
            task::print_task_full_context_by_id(id);
        }
        None => uart::write_str("none"),
    }
}

pub fn save_current_context(saved_sp: u64, saved_pc: u64) -> Option<usize> {
    let current = current_task_id()?;

    task::update_context(current, saved_sp, saved_pc);

    Some(current)
}

pub fn print_task_name(id: usize) {
    task::print_task_name_by_id(id);
}

#[allow(dead_code)]
pub fn print_task_context(id: usize) {
    task::print_task_context_by_id(id);
}

pub fn force_current_task(id: usize) {
    task::set_running(id);

    unsafe {
        CURRENT_TASK_ID = Some(id);
    }
}

pub fn switch_to_idle() {
    force_current_task(0);
}

#[allow(dead_code)]
pub fn set_current_task(id: usize) {
    unsafe {
        CURRENT_TASK_ID = Some(id);
    }
}

#[cfg(feature = "scheduler_dispatch_test")]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DispatchResult {
    NoRunnableTask,
    Failed,
}

#[cfg(feature = "scheduler_dispatch_test")]
pub fn dispatch_next() -> DispatchResult {
    crate::drivers::uart::write_line("");
    crate::drivers::uart::write_line("scheduler dispatch_next:");

    let Some(task_id) = crate::kernel::task::table::find_first_resumable_task() else {
        crate::drivers::uart::write_line("  selected task: none");
        return DispatchResult::NoRunnableTask;
    };

    crate::drivers::uart::write_str("  selected task: ");
    crate::kernel::task::table::print_task_name_by_id(task_id);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("  state: ");
    crate::kernel::task::table::print_task_state_by_id(task_id);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("  can_resume: ");
    match crate::kernel::task::table::can_task_resume(task_id) {
        Some(value) => {
            crate::kernel::task::table::print_yes_no(value);
            crate::drivers::uart::write_line("");
        }
        None => {
            crate::drivers::uart::write_line("unknown");
            return DispatchResult::Failed;
        }
    }

    crate::drivers::uart::write_line("  dispatch action: resume task");

    resume_selected_task_checked(task_id)
}

#[cfg(feature = "scheduler_dispatch_test")]
fn resume_selected_task_checked(task_id: usize) -> DispatchResult {
    crate::drivers::uart::write_line("  scheduler resume path: checked resume");

    crate::drivers::uart::write_str("  resume task: ");
    crate::kernel::task::table::print_task_name_by_id(task_id);
    crate::drivers::uart::write_line("");

    match crate::kernel::task::table::can_task_resume(task_id) {
        Some(true) => {}
        Some(false) => {
            crate::drivers::uart::write_line("  resume blocked: task cannot resume");
            return DispatchResult::Failed;
        }
        None => {
            crate::drivers::uart::write_line("  resume blocked: unknown task");
            return DispatchResult::Failed;
        }
    }

    let Some(frame) = validate_resume_frame(task_id) else {
        crate::drivers::uart::write_line("  scheduler resume path result: FAILED");
        return DispatchResult::Failed;
    };

    crate::drivers::uart::write_line("  scheduler resume path result: OK");

    restore_selected_task_checked(task_id, frame);
}

#[cfg(feature = "scheduler_dispatch_test")]
fn validate_resume_frame(task_id: usize) -> Option<TaskCpuContext> {
    crate::drivers::uart::write_line("  scheduler validate resume frame:");

    let Some(frame) = crate::kernel::task::table::get_task_resume_frame(task_id) else {
        crate::drivers::uart::write_line("    frame present: no");
        crate::drivers::uart::write_line("    result: FAILED");
        return None;
    };

    let sp_inside = crate::kernel::task::table::is_sp_inside_task_stack(task_id, frame.sp);
    let resume_pc_inside_text = crate::kernel::memory::is_inside_kernel_text(frame.resume_pc);
    let return_pc_inside_text = crate::kernel::memory::is_inside_kernel_text(frame.return_pc);

    crate::drivers::uart::write_str("    frame present: ");
    crate::kernel::task::table::print_yes_no(true);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("    frame valid: ");
    crate::kernel::task::table::print_yes_no(frame.is_valid());
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("    frame SP: ");
    crate::drivers::uart::write_hex_u64(frame.sp);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("    frame resume_pc: ");
    crate::drivers::uart::write_hex_u64(frame.resume_pc);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("    frame return_pc: ");
    crate::drivers::uart::write_hex_u64(frame.return_pc);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("    frame SP inside task stack: ");
    match sp_inside {
        Some(value) => {
            crate::kernel::task::table::print_yes_no(value);
            crate::drivers::uart::write_line("");
        }
        None => crate::drivers::uart::write_line("unknown"),
    }

    crate::drivers::uart::write_str("    frame resume_pc inside kernel text: ");
    crate::kernel::task::table::print_yes_no(resume_pc_inside_text);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("    frame return_pc inside kernel text: ");
    crate::kernel::task::table::print_yes_no(return_pc_inside_text);
    crate::drivers::uart::write_line("");

    let context_consistent = match (
        crate::kernel::task::table::get_task_last_task_sp(task_id),
        crate::kernel::task::table::get_task_last_kernel_return_pc(task_id),
    ) {
        (Some(last_sp), Some(kernel_pc)) => frame.sp == last_sp && frame.return_pc == kernel_pc,
        _ => false,
    };

    crate::drivers::uart::write_str("    frame consistent with task record: ");
    crate::kernel::task::table::print_yes_no(context_consistent);
    crate::drivers::uart::write_line("");

    let ok = frame.is_valid()
        && matches!(sp_inside, Some(true))
        && resume_pc_inside_text
        && return_pc_inside_text
        && context_consistent;

    crate::drivers::uart::write_str("    result: ");
    if ok {
        crate::drivers::uart::write_line("OK");
        Some(frame)
    } else {
        crate::drivers::uart::write_line("FAILED");
        None
    }
}

#[cfg(feature = "scheduler_dispatch_test")]
fn restore_selected_task_checked(task_id: usize, frame: TaskCpuContext) -> ! {
    crate::drivers::uart::write_line("  scheduler restore path: checked restore");

    crate::drivers::uart::write_str("  restore task: ");
    crate::kernel::task::table::print_task_name_by_id(task_id);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("  restore sp: ");
    crate::drivers::uart::write_hex_u64(frame.sp);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("  restore resume_pc: ");
    crate::drivers::uart::write_hex_u64(frame.resume_pc);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("  restore return_pc: ");
    crate::drivers::uart::write_hex_u64(frame.return_pc);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_line("  scheduler restore path result: OK");
    crate::drivers::uart::write_line("  calling arch restore from scheduler path...");

    unsafe {
        crate::arch::restore_verified_resume_frame(frame);
    }
}
