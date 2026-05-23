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
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RunOnceResult {
    NoRunnableTask,
    Failed,
}

#[cfg(feature = "scheduler_run_test")]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RunResult {
    NoRunnableTask,
    Failed,
}

#[cfg(feature = "scheduler_reentry_test")]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TaskReturnHandleResult {
    NoRunnableTask,
    Failed,
}

#[cfg(feature = "scheduler_dispatch_test")]
pub fn dispatch_next() -> DispatchResult {
    scheduler_log_line("");
    scheduler_log_line("scheduler dispatch_next:");

    let Some(task_id) = crate::kernel::task::table::find_first_resumable_task() else {
        scheduler_log_line("  selected task: none");
        return DispatchResult::NoRunnableTask;
    };

    print_dispatch_task_summary(task_id);

    scheduler_log_line("  dispatch action: resume task");

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
    scheduler_log_line("  scheduler validate resume frame:");

    let Some(frame) = crate::kernel::task::table::get_task_resume_frame(task_id) else {
        scheduler_log_line("    frame present: no");
        scheduler_log_line("    result: FAILED");
        return None;
    };

    let sp_inside = crate::kernel::task::table::is_sp_inside_task_stack(task_id, frame.sp);
    let resume_pc_inside_text = crate::kernel::memory::is_inside_kernel_text(frame.resume_pc);
    let return_pc_inside_text = crate::kernel::memory::is_inside_kernel_text(frame.return_pc);

    let context_consistent = match (
        crate::kernel::task::table::get_task_last_task_sp(task_id),
        crate::kernel::task::table::get_task_last_kernel_return_pc(task_id),
    ) {
        (Some(last_sp), Some(kernel_pc)) => frame.sp == last_sp && frame.return_pc == kernel_pc,
        _ => false,
    };

    print_resume_frame_summary(
        frame,
        sp_inside,
        resume_pc_inside_text,
        return_pc_inside_text,
        context_consistent,
    );

    let ok = frame.is_valid()
        && matches!(sp_inside, Some(true))
        && resume_pc_inside_text
        && return_pc_inside_text
        && context_consistent;

    scheduler_log_str("    result: ");
    if ok {
        scheduler_log_line("OK");
        Some(frame)
    } else {
        scheduler_log_line("FAILED");
        None
    }
}

#[cfg(feature = "scheduler_dispatch_test")]
fn restore_selected_task_checked(task_id: usize, frame: TaskCpuContext) -> ! {
    scheduler_log_line("  scheduler restore path: checked restore");

    scheduler_log_str("  restore task: ");
    crate::kernel::task::table::print_task_name_by_id(task_id);
    scheduler_log_line("");

    scheduler_log_str("  restore sp: ");
    scheduler_log_hex(frame.sp);
    scheduler_log_line("");

    scheduler_log_str("  restore resume_pc: ");
    scheduler_log_hex(frame.resume_pc);
    scheduler_log_line("");

    scheduler_log_str("  restore return_pc: ");
    scheduler_log_hex(frame.return_pc);
    scheduler_log_line("");

    scheduler_log_line("  scheduler restore path result: OK");
    scheduler_log_line("  calling arch restore from scheduler path...");

    unsafe {
        crate::arch::restore_verified_resume_frame(frame);
    }
}

#[cfg(feature = "scheduler_dispatch_test")]
fn print_dispatch_task_summary(task_id: usize) {
    scheduler_log_str("  selected task: ");
    crate::kernel::task::table::print_task_name_by_id(task_id);
    scheduler_log_line("");

    scheduler_log_str("  state: ");
    crate::kernel::task::table::print_task_state_by_id(task_id);
    scheduler_log_line("");

    scheduler_log_str("  can_resume: ");
    match crate::kernel::task::table::can_task_resume(task_id) {
        Some(value) => {
            scheduler_log_yes_no(value);
            scheduler_log_line("");
        }
        None => scheduler_log_line("unknown"),
    }
}

#[cfg(feature = "scheduler_dispatch_test")]
fn print_resume_frame_summary(
    frame: TaskCpuContext,
    sp_inside: Option<bool>,
    resume_pc_inside_text: bool,
    return_pc_inside_text: bool,
    context_consistent: bool,
) {
    scheduler_log_str("    frame present: ");
    scheduler_log_yes_no(true);
    scheduler_log_line("");

    scheduler_log_str("    frame valid: ");
    scheduler_log_yes_no(frame.is_valid());
    scheduler_log_line("");

    scheduler_log_str("    frame SP: ");
    scheduler_log_hex(frame.sp);
    scheduler_log_line("");

    scheduler_log_str("    frame resume_pc: ");
    scheduler_log_hex(frame.resume_pc);
    scheduler_log_line("");

    scheduler_log_str("    frame return_pc: ");
    scheduler_log_hex(frame.return_pc);
    scheduler_log_line("");

    scheduler_log_str("    frame SP inside task stack: ");
    match sp_inside {
        Some(value) => {
            scheduler_log_yes_no(value);
            scheduler_log_line("");
        }
        None => scheduler_log_line("unknown"),
    }

    scheduler_log_str("    frame resume_pc inside kernel text: ");
    scheduler_log_yes_no(resume_pc_inside_text);
    scheduler_log_line("");

    scheduler_log_str("    frame return_pc inside kernel text: ");
    scheduler_log_yes_no(return_pc_inside_text);
    scheduler_log_line("");

    scheduler_log_str("    frame consistent with task record: ");
    scheduler_log_yes_no(context_consistent);
    scheduler_log_line("");
}

#[cfg(feature = "scheduler_dispatch_test")]
fn scheduler_log_line(message: &str) {
    crate::drivers::uart::write_line(message);
}

#[cfg(feature = "scheduler_dispatch_test")]
fn scheduler_log_str(message: &str) {
    crate::drivers::uart::write_str(message);
}

#[cfg(feature = "scheduler_dispatch_test")]
fn scheduler_log_hex(value: u64) {
    crate::drivers::uart::write_hex_u64(value);
}

#[cfg(feature = "scheduler_dispatch_test")]
fn scheduler_log_yes_no(value: bool) {
    crate::kernel::task::table::print_yes_no(value);
}

#[cfg(feature = "scheduler_dispatch_test")]
pub fn run_once() -> RunOnceResult {
    scheduler_log_line("");

    scheduler_log_line("scheduler run_once:");

    match dispatch_next() {
        DispatchResult::NoRunnableTask => {
            scheduler_log_line("  run_once result: no runnable task");

            RunOnceResult::NoRunnableTask
        }

        DispatchResult::Failed => {
            scheduler_log_line("  run_once result: failed");

            RunOnceResult::Failed
        }
    }
}

#[cfg(feature = "scheduler_run_test")]
pub fn run() -> RunResult {
    scheduler_log_line("");
    scheduler_log_line("scheduler run:");

    match run_once() {
        RunOnceResult::NoRunnableTask => {
            scheduler_log_line("  run result: no runnable task");
            RunResult::NoRunnableTask
        }
        RunOnceResult::Failed => {
            scheduler_log_line("  run result: failed");
            RunResult::Failed
        }
    }
}

#[cfg(feature = "scheduler_reentry_test")]
pub fn handle_task_return() -> TaskReturnHandleResult {
    scheduler_log_line("");
    scheduler_log_line("scheduler handle_task_return:");

    let Some(task_id) = crate::kernel::task::table::find_first_resumable_task() else {
        scheduler_log_line("  next resumable task: none");
        scheduler_log_line("  result: no runnable task");
        return TaskReturnHandleResult::NoRunnableTask;
    };

    scheduler_log_str("  next resumable task: ");
    crate::kernel::task::table::print_task_name_by_id(task_id);
    scheduler_log_line("");

    scheduler_log_str("  state: ");
    crate::kernel::task::table::print_task_state_by_id(task_id);
    scheduler_log_line("");

    scheduler_log_str("  last_return: ");
    crate::kernel::task::table::print_task_return_kind_by_id(task_id);
    scheduler_log_line("");

    scheduler_log_line("  action: scheduler::run");

    match run() {
        RunResult::NoRunnableTask => {
            scheduler_log_line("  scheduler run returned: no runnable task");
            TaskReturnHandleResult::NoRunnableTask
        }
        RunResult::Failed => {
            scheduler_log_line("  scheduler run returned: failed");
            TaskReturnHandleResult::Failed
        }
    }
}
