use crate::drivers::uart;
use crate::kernel::irq_cell::IrqCell;
use crate::kernel::memory;
use crate::kernel::trap_frame::TrapImage;

pub const MAX_TASKS: usize = 8;

pub const TASK_NAME_LEN: usize = 16;

pub type TaskEntry = fn();

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Empty,
    Ready,
    Running,
    Blocked,
    Finished,
    Faulted,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TaskReturnKind {
    None,
    Exit,
    Yield,
    Sleep,
    Fault,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TaskLifecycleTransition {
    Start,
    Yield,
    Sleep,
    Exit,
    Fault,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TaskFaultReason {
    Breakpoint,
    InstructionAccessFault,
    LoadAccessFault,
    StoreAccessFault,
    IllegalInstruction,
    Unknown(u64),
}

impl TaskFaultReason {
    /// Map `mcause` to a concrete fault reason.
    /// RISC-V Privileged Spec, Table 16 (synchronous exceptions).
    pub const fn from_mcause(cause: u64) -> Self {
        match cause {
            1 => Self::InstructionAccessFault,
            2 => Self::IllegalInstruction,
            3 => Self::Breakpoint,
            5 => Self::LoadAccessFault,
            7 => Self::StoreAccessFault,
            other => Self::Unknown(other),
        }
    }
}

#[derive(Clone, Copy)]
pub struct Task {
    pub id: usize,
    pub state: TaskState,
    pub name: [u8; TASK_NAME_LEN],
    pub stack_start: u64,
    pub stack_top: u64,
    pub entry: Option<TaskEntry>,
    pub initial_sp: u64,
    pub initial_pc: u64,
    pub trap_image: Option<TrapImage>,
    pub has_started: bool,
    pub can_resume: bool,
    pub last_return_kind: TaskReturnKind,
    pub sleep_until_tick: Option<u64>,
}

impl Task {
    pub const fn empty() -> Self {
        Self {
            id: 0,
            state: TaskState::Empty,
            name: [0; TASK_NAME_LEN],
            stack_start: 0,
            stack_top: 0,
            entry: None,
            initial_sp: 0,
            initial_pc: 0,
            trap_image: None,
            has_started: false,
            can_resume: false,
            last_return_kind: TaskReturnKind::None,
            sleep_until_tick: None,
        }
    }
}

static TASKS: IrqCell<[Task; MAX_TASKS]> = IrqCell::new([Task::empty(); MAX_TASKS]);

fn snapshot_tasks() -> [Task; MAX_TASKS] {
    TASKS.with(|tasks| *tasks)
}

fn write_tasks(tasks: &[Task; MAX_TASKS]) {
    TASKS.with(|slot| *slot = *tasks);
}

/// Occupied slot for `id`. Task ids are slot indices: `id == slot`.
fn find_slot_by_id(id: usize) -> Option<usize> {
    TASKS.with(|tasks| {
        if id >= MAX_TASKS {
            return None;
        }

        if !matches!(tasks[id].state, TaskState::Empty) && tasks[id].id == id {
            Some(id)
        } else {
            None
        }
    })
}

fn with_task<R>(id: usize, f: impl FnOnce(&Task) -> R) -> Option<R> {
    TASKS.with(|tasks| {
        if id >= MAX_TASKS {
            return None;
        }
        if !matches!(tasks[id].state, TaskState::Empty) && tasks[id].id == id {
            Some(f(&tasks[id]))
        } else {
            None
        }
    })
}

fn with_task_mut<R>(id: usize, f: impl FnOnce(&mut Task) -> R) -> Option<R> {
    TASKS.with(|tasks| {
        if id >= MAX_TASKS {
            return None;
        }
        if !matches!(tasks[id].state, TaskState::Empty) && tasks[id].id == id {
            Some(f(&mut tasks[id]))
        } else {
            None
        }
    })
}

pub fn init() {
    let mut tasks = [Task::empty(); MAX_TASKS];
    for (slot, task) in tasks.iter_mut().enumerate() {
        task.id = slot;
    }
    write_tasks(&tasks);

    uart::write_line("");
    uart::write_line("task system:");
    uart::write_str("max tasks: ");
    uart::write_dec_u64(MAX_TASKS as u64);
    uart::write_line("");
}

pub fn create_task(name: &str, entry: TaskEntry) -> Option<usize> {
    let slot = snapshot_tasks()
        .iter()
        .position(|task| matches!(task.state, TaskState::Empty))?;

    let Some(stack_start) = memory::allocate_page() else {
        uart::write_str("failed to allocate stack for task: ");
        uart::write_line(name);
        return None;
    };

    let Some(stack_top) = stack_start.checked_add(memory::PAGE_SIZE) else {
        free_task_stack_page(stack_start);
        uart::write_str("invalid stack range for task: ");
        uart::write_line(name);
        return None;
    };

    let initial_pc = entry as *const () as usize as u64;
    let mut printed_name = [0u8; TASK_NAME_LEN];
    copy_name(&mut printed_name, name);

    TASKS.with(|tasks| {
        tasks[slot] = Task {
            id: slot,
            state: TaskState::Ready,
            name: printed_name,
            stack_start,
            stack_top,
            entry: Some(entry),
            initial_sp: stack_top,
            initial_pc,
            ..Task::empty()
        };
    });

    uart::write_str("created task: ");
    write_name(&printed_name);
    print_hex_field(" stack: ", stack_start);
    print_hex_field(" - ", stack_top);
    print_hex_field(" entry: ", initial_pc);
    print_hex_field(" initial_sp: ", stack_top);
    print_hex_field(" initial_pc: ", initial_pc);
    uart::write_line("");

    Some(slot)
}

pub fn print_tasks() {
    uart::write_line("task list:");

    for task in snapshot_tasks() {
        if matches!(task.state, TaskState::Empty) {
            continue;
        }
        print_task_row(&task);
    }
}

fn print_task_row(task: &Task) {
    uart::write_str("id: ");
    uart::write_dec_u64(task.id as u64);
    uart::write_str(" state: ");
    print_state(task.state);
    uart::write_str(" name: ");
    write_name(&task.name);
    print_hex_field(" stack: ", task.stack_start);
    print_hex_field(" - ", task.stack_top);
    uart::write_str(" entry: ");
    match task.entry {
        Some(entry) => uart::write_hex_u64(entry as *const () as usize as u64),
        None => uart::write_str("none"),
    }
    print_hex_field(" initial_sp: ", task.initial_sp);
    print_hex_field(" initial_pc: ", task.initial_pc);
    uart::write_str(" trap_image: ");
    match task.trap_image {
        Some(image) => {
            print_hex_field("sp: ", image.gpr.sp);
            print_hex_field(" mepc: ", image.mepc);
        }
        None => uart::write_str("none"),
    }
    uart::write_str(" started: ");
    print_yes_no(task.has_started);
    uart::write_str(" can_resume: ");
    print_yes_no(task.can_resume);
    uart::write_str(" last_return: ");
    print_task_return_kind(task.last_return_kind);
    uart::write_line("");
}

fn print_hex_field(label: &str, value: u64) {
    uart::write_str(label);
    uart::write_hex_u64(value);
}

pub fn get_task_name(id: usize) -> Option<[u8; TASK_NAME_LEN]> {
    with_task(id, |task| task.name)
}

pub fn get_task_entry(id: usize) -> Option<TaskEntry> {
    with_task(id, |task| task.entry)?
}

pub fn get_task_stack_start(id: usize) -> Option<u64> {
    with_task(id, |task| task.stack_start)
}

pub fn get_task_stack_top(id: usize) -> Option<u64> {
    with_task(id, |task| task.stack_top)
}

pub fn get_task_initial_sp(id: usize) -> Option<u64> {
    with_task(id, |task| task.initial_sp)
}

pub fn get_task_initial_pc(id: usize) -> Option<u64> {
    with_task(id, |task| task.initial_pc)
}

pub fn find_next_dispatchable_after(current_id: Option<usize>) -> Option<usize> {
    find_next_task_after(current_id, |task| is_dispatchable_task(task.id))
}

pub fn mark_task_running(id: usize) -> bool {
    let Some(target_slot) = find_slot_by_id(id) else {
        return false;
    };

    let mut tasks = snapshot_tasks();
    if !matches!(
        tasks[target_slot].state,
        TaskState::Ready | TaskState::Running
    ) {
        return false;
    }

    for (slot, task) in tasks.iter_mut().enumerate() {
        if slot == target_slot {
            task.state = TaskState::Running;
        } else if matches!(task.state, TaskState::Running) {
            task.state = TaskState::Ready;
        }
    }

    write_tasks(&tasks);
    true
}

const fn can_transition_from(state: TaskState, transition: TaskLifecycleTransition) -> bool {
    match transition {
        TaskLifecycleTransition::Start => matches!(state, TaskState::Ready | TaskState::Running),
        TaskLifecycleTransition::Yield => matches!(state, TaskState::Ready | TaskState::Running),
        TaskLifecycleTransition::Sleep => matches!(state, TaskState::Ready | TaskState::Running),
        TaskLifecycleTransition::Exit => matches!(state, TaskState::Ready | TaskState::Running),
        TaskLifecycleTransition::Fault => matches!(state, TaskState::Ready | TaskState::Running),
    }
}

pub fn mark_task_started(id: usize) -> bool {
    with_task_mut(id, |task| {
        if !can_transition_from(task.state, TaskLifecycleTransition::Start) {
            return false;
        }
        task.has_started = true;
        true
    })
    .unwrap_or(false)
}

pub fn has_started(id: usize) -> bool {
    with_task(id, |task| task.has_started).unwrap_or(false)
}

pub fn print_task_name_by_id(id: usize) {
    match get_task_name(id) {
        Some(name) => write_name(&name),
        None => uart::write_str("unknown"),
    }
}

pub fn print_task_context_values(saved_sp: u64, saved_pc: u64) {
    uart::write_str(" saved_sp: ");
    uart::write_hex_u64(saved_sp);

    uart::write_str(" saved_pc: ");
    uart::write_hex_u64(saved_pc);
}

pub fn print_task_full_context_by_id(id: usize) {
    uart::write_str(" initial_sp: ");

    match get_task_initial_sp(id) {
        Some(sp) => uart::write_hex_u64(sp),
        None => uart::write_str("none"),
    }

    uart::write_str(" initial_pc: ");

    match get_task_initial_pc(id) {
        Some(pc) => uart::write_hex_u64(pc),
        None => uart::write_str("none"),
    }

    uart::write_str(" started: ");
    if has_started(id) {
        uart::write_str("yes");
    } else {
        uart::write_str("no");
    }
}

fn find_next_task_after<F>(current_id: Option<usize>, mut accept: F) -> Option<usize>
where
    F: FnMut(Task) -> bool,
{
    let start_slot = current_id
        .and_then(find_slot_by_id)
        .map_or(0, |slot| slot + 1);
    let snapshot = snapshot_tasks();

    for offset in 0..MAX_TASKS {
        let slot = (start_slot + offset) % MAX_TASKS;
        let task = snapshot[slot];
        if matches!(task.state, TaskState::Empty) {
            continue;
        }

        if accept(task) {
            return Some(task.id);
        }
    }

    None
}

fn copy_name(dst: &mut [u8; TASK_NAME_LEN], name: &str) {
    dst.fill(0);
    let bytes = name.as_bytes();
    let len = core::cmp::min(bytes.len(), TASK_NAME_LEN - 1);
    dst[..len].copy_from_slice(&bytes[..len]);
}

fn write_name(name: &[u8; TASK_NAME_LEN]) {
    for byte in name {
        if *byte == 0 {
            break;
        }

        uart::putc(*byte);
    }
}

fn print_state(state: TaskState) {
    match state {
        TaskState::Empty => uart::write_str("Empty"),
        TaskState::Ready => uart::write_str("Ready"),
        TaskState::Running => uart::write_str("Running"),
        TaskState::Blocked => uart::write_str("Blocked"),
        TaskState::Finished => uart::write_str("Finished"),
        TaskState::Faulted => uart::write_str("Faulted"),
    }
}

pub fn get_task_state(id: usize) -> Option<TaskState> {
    with_task(id, |task| task.state)
}

pub fn get_task_return_kind(id: usize) -> Option<TaskReturnKind> {
    with_task(id, |task| task.last_return_kind)
}

pub fn print_task_return_kind(kind: TaskReturnKind) {
    match kind {
        TaskReturnKind::None => uart::write_str("None"),
        TaskReturnKind::Exit => uart::write_str("Exit"),
        TaskReturnKind::Yield => uart::write_str("Yield"),
        TaskReturnKind::Sleep => uart::write_str("Sleep"),
        TaskReturnKind::Fault => uart::write_str("Fault"),
    }
}

pub fn is_sp_inside_task_stack(id: usize, sp: u64) -> Option<bool> {
    with_task(id, |task| sp >= task.stack_start && sp < task.stack_top)
}

pub fn can_task_resume(id: usize) -> Option<bool> {
    with_task(id, |task| task.can_resume)
}

#[cfg(feature = "scenario_sleep")]
pub fn find_first_resumable_task() -> Option<usize> {
    for task in snapshot_tasks() {
        if matches!(task.state, TaskState::Ready) && is_resumable_task(task.id) {
            return Some(task.id);
        }
    }

    None
}

pub fn print_yes_no(value: bool) {
    if value {
        uart::write_str("yes");
    } else {
        uart::write_str("no");
    }
}

pub fn set_task_trap_image(id: usize, image: &TrapImage) -> bool {
    with_task_mut(id, |task| task.trap_image = Some(*image)).is_some()
}

pub fn get_task_trap_image(id: usize) -> Option<TrapImage> {
    with_task(id, |task| task.trap_image)?
}

/// Save a yielded or timer-preempted worker. `mepc` is the resume PC
/// (`+4` after `ecall`, unchanged after a timer interrupt).
pub fn save_preempted_trap_image(id: usize, image: &TrapImage) -> bool {
    if !set_task_trap_image(id, image) {
        return false;
    }
    mark_task_ready_after_yield(id)
}

pub fn is_task_ready(id: usize) -> bool {
    matches!(get_task_state(id), Some(TaskState::Ready))
}

pub fn is_resumable_task(id: usize) -> bool {
    is_task_ready(id)
        && matches!(can_task_resume(id), Some(true))
        && matches!(
            get_task_return_kind(id),
            Some(TaskReturnKind::Yield | TaskReturnKind::Sleep)
        )
        && is_resume_frame_safe_for_task(id)
}

pub fn is_resume_frame_safe_for_task(id: usize) -> bool {
    let Some(image) = get_task_trap_image(id) else {
        return false;
    };

    image.is_valid()
        && matches!(is_sp_inside_task_stack(id, image.gpr.sp), Some(true))
        && memory::is_inside_kernel_text(image.mepc)
}

pub fn is_fresh_ready_task(id: usize) -> bool {
    is_task_ready(id) && !has_started(id) && matches!(can_task_resume(id), Some(false))
}

pub fn is_dispatchable_task(id: usize) -> bool {
    is_resumable_task(id) || is_fresh_ready_task(id)
}

pub fn record_task_fault(id: usize, mcause: u64) -> Option<TaskFaultReason> {
    if mark_task_faulted(id) {
        Some(TaskFaultReason::from_mcause(mcause))
    } else {
        None
    }
}

pub fn print_task_fault_reason(reason: TaskFaultReason) {
    match reason {
        TaskFaultReason::Breakpoint => uart::write_str("breakpoint"),
        TaskFaultReason::InstructionAccessFault => uart::write_str("instruction access fault"),
        TaskFaultReason::LoadAccessFault => uart::write_str("load access fault"),
        TaskFaultReason::StoreAccessFault => uart::write_str("store access fault"),
        TaskFaultReason::IllegalInstruction => uart::write_str("illegal instruction"),
        TaskFaultReason::Unknown(code) => {
            uart::write_str("unknown (code: ");
            uart::write_hex_u64(code);
            uart::write_str(")");
        }
    }
}

pub fn is_task_finished(id: usize) -> bool {
    matches!(get_task_state(id), Some(TaskState::Finished))
}

pub fn is_task_faulted(id: usize) -> bool {
    matches!(get_task_state(id), Some(TaskState::Faulted))
}

pub fn is_terminal_task(id: usize) -> bool {
    is_task_finished(id) || is_task_faulted(id)
}

pub fn has_dispatchable_tasks() -> bool {
    snapshot_tasks()
        .iter()
        .any(|task| !matches!(task.state, TaskState::Empty) && is_dispatchable_task(task.id))
}

pub fn mark_task_finished(id: usize) -> bool {
    with_task_mut(id, |task| {
        if !can_transition_from(task.state, TaskLifecycleTransition::Exit) {
            return false;
        }
        task.state = TaskState::Finished;
        task.last_return_kind = TaskReturnKind::Exit;
        task.can_resume = false;
        true
    })
    .unwrap_or(false)
}

pub fn mark_task_ready_after_yield(id: usize) -> bool {
    with_task_mut(id, |task| {
        if !can_transition_from(task.state, TaskLifecycleTransition::Yield) {
            return false;
        }
        task.state = TaskState::Ready;
        task.last_return_kind = TaskReturnKind::Yield;
        task.can_resume = true;
        task.sleep_until_tick = None;
        true
    })
    .unwrap_or(false)
}

pub fn mark_task_blocked_until(id: usize, wake_tick: u64) -> bool {
    with_task_mut(id, |task| {
        if !can_transition_from(task.state, TaskLifecycleTransition::Sleep) {
            return false;
        }

        task.state = TaskState::Blocked;
        task.last_return_kind = TaskReturnKind::Sleep;
        task.can_resume = false;
        task.sleep_until_tick = Some(wake_tick);
        true
    })
    .unwrap_or(false)
}

pub fn wake_sleeping_tasks(current_tick: u64) -> usize {
    let mut due = [false; MAX_TASKS];
    let mut tasks = snapshot_tasks();

    for (slot, task) in tasks.iter_mut().enumerate() {
        if !matches!(task.state, TaskState::Blocked) {
            continue;
        }

        let Some(wake_tick) = task.sleep_until_tick else {
            continue;
        };

        if current_tick >= wake_tick {
            task.state = TaskState::Ready;
            task.sleep_until_tick = None;
            due[slot] = true;
        }
    }

    write_tasks(&tasks);

    let mut woke = 0;
    for (slot, is_due) in due.iter().enumerate() {
        if !is_due {
            continue;
        }

        let can_resume = is_resume_frame_safe_for_task(slot);
        with_task_mut(slot, |task| {
            task.last_return_kind = if can_resume {
                TaskReturnKind::Sleep
            } else {
                TaskReturnKind::None
            };
            task.can_resume = can_resume;
        });
        woke += 1;
    }

    woke
}

pub fn mark_task_faulted(id: usize) -> bool {
    with_task_mut(id, |task| {
        if !can_transition_from(task.state, TaskLifecycleTransition::Fault) {
            return false;
        }
        task.state = TaskState::Faulted;
        task.last_return_kind = TaskReturnKind::Fault;
        task.can_resume = false;
        true
    })
    .unwrap_or(false)
}

/// Reap a `Finished` or `Faulted` slot.
///
/// Frees the stack page and returns the slot to `Empty` so `id == slot` can be
/// reused. Idle is not special-cased; callers that keep an idle task simply do
/// not destroy it.
pub fn destroy(id: usize) -> bool {
    let Some(stack_start) = with_task_mut(id, |task| {
        if !matches!(task.state, TaskState::Finished | TaskState::Faulted) {
            return None;
        }
        let stack_start = task.stack_start;
        let slot_id = task.id;
        *task = Task::empty();
        task.id = slot_id;
        Some(stack_start)
    })
    .flatten() else {
        return false;
    };

    free_task_stack_page(stack_start);
    true
}

fn free_task_stack_page(stack_start: u64) {
    if stack_start == 0 {
        return;
    }
    if let Some(page) = memory::PhysPage::new(stack_start) {
        memory::free_pages(page, 1);
    }
}
