use crate::drivers::uart;
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
