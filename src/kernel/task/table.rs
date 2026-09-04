use crate::drivers::uart;
use crate::kernel::irq_cell::IrqCell;
use crate::kernel::memory;
use crate::kernel::task::context;
use crate::kernel::task::cpu_context::{self, TaskCpuContext};

pub const MAX_TASKS: usize = 8;

#[allow(dead_code)]
pub const fn max_tasks() -> usize {
    MAX_TASKS
}

pub const TASK_NAME_LEN: usize = 16;
static LAST_RETURNED_TASK_ID: IrqCell<Option<usize>> = IrqCell::new(None);

pub type TaskEntry = fn();

#[derive(Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum TaskState {
    Empty,
    Ready,
    Running,
    Blocked,
    Finished,
    Faulted,
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum TaskReturnKind {
    None,
    Exit,
    Yield,
    Sleep,
    Fault,
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum TaskLifecycleTransition {
    Start,
    Yield,
    Sleep,
    Exit,
    Fault,
}

#[derive(Clone, Copy)]
pub struct TaskReturnContext {
    pub task_sp: u64,
    pub kernel_sp: u64,
    pub kernel_return_pc: u64,
}

#[cfg(any(
    feature = "scheduler_reentry_test",
    feature = "scheduler_dispatch_test"
))]
#[derive(Clone, Copy)]
pub struct TaskReturnSnapshot {
    pub task_id: usize,
    pub state: TaskState,
    pub last_return: TaskReturnKind,
    pub can_resume: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum TaskFaultReason {
    Breakpoint,
    InstructionAccessFault,
    LoadAccessFault,
    StoreAccessFault,
    IllegalInstruction,
    Unknown(u64),
}

impl TaskFaultReason {
    #[allow(dead_code)]
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
    pub saved_sp: u64,
    pub saved_pc: u64,
    pub cpu_context: TaskCpuContext,
    pub last_kernel_sp: u64,
    pub last_kernel_return_pc: u64,
    pub last_task_sp: u64,
    pub has_started: bool,
    pub can_resume: bool,
    pub last_return_kind: TaskReturnKind,
    #[allow(dead_code)]
    pub last_fault_reason: Option<TaskFaultReason>,
    #[allow(dead_code)]
    pub last_fault_mcause: Option<u64>,
    #[allow(dead_code)]
    pub last_fault_mepc: Option<u64>,
    #[allow(dead_code)]
    pub last_fault_mtval: Option<u64>,
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
            saved_sp: 0,
            saved_pc: 0,
            cpu_context: TaskCpuContext::empty(),
            last_kernel_sp: 0,
            last_kernel_return_pc: 0,
            last_task_sp: 0,
            has_started: false,
            can_resume: false,
            last_return_kind: TaskReturnKind::None,
            last_fault_reason: None,
            last_fault_mcause: None,
            last_fault_mepc: None,
            last_fault_mtval: None,
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
        uart::write_str("invalid stack range for task: ");
        uart::write_line(name);
        return None;
    };
    let initial_sp = stack_top;
    let initial_pc = entry as *const () as usize as u64;

    let Some(prepared_sp) = context::prepare_initial_stack(stack_top, initial_pc) else {
        uart::write_str("failed to prepare stack for task: ");
        uart::write_line(name);
        return None;
    };

    let saved_sp = prepared_sp;
    let saved_pc = initial_pc;
    let id = slot;
    let mut printed_name = [0u8; TASK_NAME_LEN];

    TASKS.with(|tasks| {
        let task = &mut tasks[slot];
        task.id = id;
        task.state = TaskState::Ready;
        task.stack_start = stack_start;
        task.stack_top = stack_top;
        task.entry = Some(entry);
        task.initial_sp = initial_sp;
        task.initial_pc = initial_pc;
        task.saved_sp = saved_sp;
        task.saved_pc = saved_pc;
        task.cpu_context = TaskCpuContext::initial(saved_sp, saved_pc);
        task.last_kernel_sp = 0;
        task.last_kernel_return_pc = 0;
        task.last_task_sp = 0;
        task.has_started = false;
        task.can_resume = false;
        task.last_return_kind = TaskReturnKind::None;
        task.sleep_until_tick = None;
        copy_name(&mut task.name, name);
        printed_name = task.name;
    });

    uart::write_str("created task: ");
    write_name(&printed_name);

    uart::write_str(" stack: ");
    uart::write_hex_u64(stack_start);

    uart::write_str(" - ");
    uart::write_hex_u64(stack_top);

    uart::write_str(" entry: ");
    uart::write_hex_u64(initial_pc);

    uart::write_str(" initial_sp: ");
    uart::write_hex_u64(initial_sp);

    uart::write_str(" initial_pc: ");
    uart::write_hex_u64(initial_pc);

    uart::write_str(" prepared_sp: ");
    uart::write_hex_u64(prepared_sp);

    context::print_initial_context(prepared_sp);

    uart::write_line("");

    Some(id)
}

pub fn print_tasks() {
    uart::write_line("task list:");

    for task in snapshot_tasks() {
        if matches!(task.state, TaskState::Empty) {
            continue;
        }

        uart::write_str("id: ");
        uart::write_dec_u64(task.id as u64);

        uart::write_str(" state: ");
        print_state(task.state);

        uart::write_str(" name: ");
        write_name(&task.name);

        uart::write_str(" stack: ");
        uart::write_hex_u64(task.stack_start);

        uart::write_str(" - ");
        uart::write_hex_u64(task.stack_top);

        uart::write_str(" entry: ");
        match task.entry {
            Some(entry) => uart::write_hex_u64(entry as *const () as usize as u64),
            None => uart::write_str("none"),
        }

        uart::write_str(" initial_sp: ");
        uart::write_hex_u64(task.initial_sp);

        uart::write_str(" initial_pc: ");
        uart::write_hex_u64(task.initial_pc);

        uart::write_str(" saved_sp: ");
        uart::write_hex_u64(task.saved_sp);

        uart::write_str(" saved_pc: ");
        uart::write_hex_u64(task.saved_pc);

        cpu_context::print_cpu_context(task.cpu_context);

        if !task.has_started && matches!(task.state, TaskState::Ready) {
            uart::write_str(" initial_frame:");
            context::print_initial_context(task.saved_sp);
        }

        uart::write_str(" started: ");
        if task.has_started {
            uart::write_str("yes");
        } else {
            uart::write_str("no");
        }

        uart::write_str(" can_resume: ");
        if task.can_resume {
            uart::write_str("yes");
        } else {
            uart::write_str("no");
        }

        uart::write_str(" last_return: ");
        print_task_return_kind(task.last_return_kind);

        uart::write_str(" last_task_sp: ");
        uart::write_hex_u64(task.last_task_sp);

        uart::write_str(" last_kernel_sp: ");
        uart::write_hex_u64(task.last_kernel_sp);

        uart::write_str(" last_kernel_return_pc: ");
        uart::write_hex_u64(task.last_kernel_return_pc);

        uart::write_line("");
    }
}

pub fn get_task_name(id: usize) -> Option<[u8; TASK_NAME_LEN]> {
    with_task(id, |task| task.name)
}

pub fn get_task_entry(id: usize) -> Option<TaskEntry> {
    with_task(id, |task| task.entry)?
}

#[allow(dead_code)]
pub fn get_task_stack_start(id: usize) -> Option<u64> {
    with_task(id, |task| task.stack_start)
}

#[allow(dead_code)]
pub fn get_task_stack_top(id: usize) -> Option<u64> {
    with_task(id, |task| task.stack_top)
}

pub fn get_task_initial_sp(id: usize) -> Option<u64> {
    with_task(id, |task| task.initial_sp)
}

pub fn get_task_initial_pc(id: usize) -> Option<u64> {
    with_task(id, |task| task.initial_pc)
}

pub fn get_task_saved_sp(id: usize) -> Option<u64> {
    with_task(id, |task| task.saved_sp)
}

pub fn get_task_saved_pc(id: usize) -> Option<u64> {
    with_task(id, |task| task.saved_pc)
}

pub fn find_next_ready_after(current_id: Option<usize>) -> Option<usize> {
    find_next_task_after(current_id, |task| matches!(task.state, TaskState::Ready))
}

#[allow(dead_code)]
pub fn find_next_dispatchable_after(current_id: Option<usize>) -> Option<usize> {
    find_next_task_after(current_id, |task| is_dispatchable_task(task.id))
}

#[cfg(feature = "scheduler_reentry_test")]
pub fn set_running(id: usize) {
    let _ = mark_task_running(id);
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

pub fn can_apply_task_transition(id: usize, transition: TaskLifecycleTransition) -> bool {
    get_task_state(id).is_some_and(|state| can_transition_from(state, transition))
}

#[allow(dead_code)]
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

pub fn update_context(id: usize, saved_sp: u64, saved_pc: u64) -> bool {
    with_task_mut(id, |task| {
        task.saved_sp = saved_sp;
        task.saved_pc = saved_pc;
        task.has_started = true;
    })
    .is_some()
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

pub fn print_task_entry_by_id(id: usize) {
    match get_task_entry(id) {
        Some(entry) => uart::write_hex_u64(entry as *const () as usize as u64),
        None => uart::write_str("none"),
    }
}

#[allow(dead_code)]
pub fn print_task_context_by_id(id: usize) {
    uart::write_str(" saved_sp: ");

    match get_task_saved_sp(id) {
        Some(sp) => uart::write_hex_u64(sp),
        None => uart::write_str("none"),
    }

    uart::write_str(" saved_pc: ");

    match get_task_saved_pc(id) {
        Some(pc) => uart::write_hex_u64(pc),
        None => uart::write_str("none"),
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

    uart::write_str(" saved_sp: ");

    match get_task_saved_sp(id) {
        Some(sp) => uart::write_hex_u64(sp),
        None => uart::write_str("none"),
    }

    uart::write_str(" saved_pc: ");

    match get_task_saved_pc(id) {
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

#[allow(dead_code)]
pub fn print_task_fault_info_by_id(id: usize) {
    let Some(task) = with_task(id, |task| *task) else {
        uart::write_line("  fault info: task not found");
        return;
    };

    uart::write_line("  fault info:");

    uart::write_str("    reason: ");
    match task.last_fault_reason {
        Some(reason) => print_task_fault_reason(reason),
        None => uart::write_str("none"),
    }
    uart::write_line("");

    uart::write_str("    mcause: ");
    match task.last_fault_mcause {
        Some(value) => uart::write_hex_u64(value),
        None => uart::write_str("none"),
    }
    uart::write_line("");

    uart::write_str("    mepc:   ");
    match task.last_fault_mepc {
        Some(value) => uart::write_hex_u64(value),
        None => uart::write_str("none"),
    }
    uart::write_line("");

    uart::write_str("    mtval:  ");
    match task.last_fault_mtval {
        Some(value) => uart::write_hex_u64(value),
        None => uart::write_str("none"),
    }
    uart::write_line("");
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

pub fn print_task_state_by_id(id: usize) {
    match get_task_state(id) {
        Some(state) => print_state(state),
        None => uart::write_str("unknown"),
    }
}

pub fn set_task_return_kind(id: usize, kind: TaskReturnKind) -> bool {
    with_task_mut(id, |task| task.last_return_kind = kind).is_some()
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

pub fn print_task_return_kind_by_id(id: usize) {
    match get_task_return_kind(id) {
        Some(kind) => print_task_return_kind(kind),
        None => uart::write_str("unknown"),
    }
}

pub fn set_task_last_return_context(
    id: usize,
    task_sp: u64,
    kernel_sp: u64,
    kernel_return_pc: u64,
) -> bool {
    with_task_mut(id, |task| {
        task.last_task_sp = task_sp;
        task.last_kernel_sp = kernel_sp;
        task.last_kernel_return_pc = kernel_return_pc;
    })
    .is_some()
}

pub fn apply_task_return_transition(
    task_id: usize,
    kind: TaskReturnKind,
    context: TaskReturnContext,
    cpu_context: TaskCpuContext,
) -> bool {
    if !set_task_last_return_context(
        task_id,
        context.task_sp,
        context.kernel_sp,
        context.kernel_return_pc,
    ) {
        return false;
    }

    if !set_task_cpu_context(task_id, cpu_context) {
        return false;
    }

    set_last_returned_task_id(task_id);

    match kind {
        TaskReturnKind::Yield => mark_task_ready_after_yield(task_id),
        TaskReturnKind::Sleep => mark_task_blocked_for_sleep(task_id),
        TaskReturnKind::Exit => mark_task_finished(task_id),
        TaskReturnKind::Fault => {
            if matches!(get_task_state(task_id), Some(TaskState::Faulted)) {
                set_task_return_kind(task_id, TaskReturnKind::Fault)
                    && set_task_can_resume(task_id, false)
            } else {
                mark_task_faulted(task_id)
            }
        }
        TaskReturnKind::None => set_task_return_kind(task_id, kind),
    }
}

pub fn is_sp_inside_task_stack(id: usize, sp: u64) -> Option<bool> {
    with_task(id, |task| sp >= task.stack_start && sp < task.stack_top)
}

pub fn set_task_can_resume(id: usize, can_resume: bool) -> bool {
    with_task_mut(id, |task| task.can_resume = can_resume).is_some()
}

pub fn can_task_resume(id: usize) -> Option<bool> {
    with_task(id, |task| task.can_resume)
}

pub fn get_task_last_task_sp(id: usize) -> Option<u64> {
    with_task(id, |task| task.last_task_sp)
}

#[allow(dead_code)]
pub fn get_task_last_kernel_sp(id: usize) -> Option<u64> {
    with_task(id, |task| task.last_kernel_sp)
}

pub fn get_task_last_kernel_return_pc(id: usize) -> Option<u64> {
    with_task(id, |task| task.last_kernel_return_pc)
}

#[allow(dead_code)]
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

pub fn set_task_cpu_context(id: usize, context: TaskCpuContext) -> bool {
    with_task_mut(id, |task| {
        task.cpu_context = context;
        task.saved_sp = context.sp;
        task.saved_pc = context.return_pc;
    })
    .is_some()
}

pub fn get_task_cpu_context(id: usize) -> Option<TaskCpuContext> {
    with_task(id, |task| task.cpu_context)
}

pub fn get_task_resume_frame(
    id: usize,
) -> Option<crate::kernel::task::cpu_context::TaskCpuContext> {
    get_task_cpu_context(id)
}

pub fn set_last_returned_task_id(id: usize) {
    LAST_RETURNED_TASK_ID.with(|last| *last = Some(id));
}

#[allow(dead_code)]
pub fn get_last_returned_task_id() -> Option<usize> {
    LAST_RETURNED_TASK_ID.with(|last| *last)
}

pub fn is_task_ready(id: usize) -> bool {
    matches!(get_task_state(id), Some(TaskState::Ready))
}

#[allow(dead_code)]
pub fn is_task_running(id: usize) -> bool {
    matches!(get_task_state(id), Some(TaskState::Running))
}

pub fn is_ready_running_faulted_finished_invariant_ok(id: usize) -> bool {
    match get_task_state(id) {
        Some(TaskState::Ready) => {
            !is_task_running(id) && !is_task_faulted(id) && !is_task_finished(id)
        }
        Some(TaskState::Running) => {
            !is_task_ready(id) && !is_task_faulted(id) && !is_task_finished(id)
        }
        Some(TaskState::Faulted) => {
            !is_task_ready(id) && !is_task_running(id) && !is_task_finished(id)
        }
        Some(TaskState::Finished) => {
            !is_task_ready(id) && !is_task_running(id) && !is_task_faulted(id)
        }
        _ => false,
    }
}

#[allow(dead_code)]
pub fn can_dispatch_from_ready(id: usize) -> bool {
    is_task_ready(id) && is_ready_running_faulted_finished_invariant_ok(id)
}

#[cfg(any(
    feature = "scheduler_reentry_test",
    feature = "scheduler_dispatch_test"
))]
pub fn get_task_return_snapshot(id: usize) -> Option<TaskReturnSnapshot> {
    let state = get_task_state(id)?;
    let last_return = get_task_return_kind(id)?;
    let can_resume = can_task_resume(id)?;

    Some(TaskReturnSnapshot {
        task_id: id,
        state,
        last_return,
        can_resume,
    })
}

#[cfg(any(
    feature = "scheduler_reentry_test",
    feature = "scheduler_dispatch_test"
))]
pub fn get_last_returned_task_snapshot() -> Option<TaskReturnSnapshot> {
    let id = get_last_returned_task_id()?;
    get_task_return_snapshot(id)
}

#[allow(dead_code)]
pub fn is_resumable_task(id: usize) -> bool {
    can_dispatch_from_ready(id)
        && matches!(can_task_resume(id), Some(true))
        && matches!(
            get_task_return_kind(id),
            Some(TaskReturnKind::Yield | TaskReturnKind::Sleep)
        )
        && is_resume_frame_safe_for_task(id)
}

pub fn is_resume_frame_safe_for_task(id: usize) -> bool {
    let Some(frame) = get_task_resume_frame(id) else {
        return false;
    };

    frame.is_valid()
        && matches!(is_sp_inside_task_stack(id, frame.sp), Some(true))
        && memory::is_inside_kernel_text(frame.resume_pc)
        && memory::is_inside_kernel_text(frame.return_pc)
        && matches!(
            (get_task_last_task_sp(id), get_task_last_kernel_return_pc(id)),
            (Some(last_sp), Some(return_pc)) if frame.sp == last_sp && frame.return_pc == return_pc
        )
}

#[allow(dead_code)]
pub fn is_fresh_ready_task(id: usize) -> bool {
    can_dispatch_from_ready(id) && !has_started(id) && matches!(can_task_resume(id), Some(false))
}

pub fn is_dispatchable_task(id: usize) -> bool {
    is_resumable_task(id) || is_fresh_ready_task(id)
}

#[allow(dead_code)]
pub fn set_task_fault_info(
    id: usize,
    reason: TaskFaultReason,
    mcause: u64,
    mepc: u64,
    mtval: u64,
) -> bool {
    with_task_mut(id, |task| {
        task.last_fault_reason = Some(reason);
        task.last_fault_mcause = Some(mcause);
        task.last_fault_mepc = Some(mepc);
        task.last_fault_mtval = Some(mtval);
    })
    .is_some()
}

#[allow(dead_code)]
pub fn record_task_fault(id: usize, mcause: u64, mepc: u64, mtval: u64) -> Option<TaskFaultReason> {
    let reason = TaskFaultReason::from_mcause(mcause);

    if set_task_fault_info(id, reason, mcause, mepc, mtval) && mark_task_faulted(id) {
        Some(reason)
    } else {
        None
    }
}

#[allow(dead_code)]
pub fn get_task_fault_reason(id: usize) -> Option<TaskFaultReason> {
    with_task(id, |task| task.last_fault_reason)?
}

#[allow(dead_code)]
pub fn get_task_fault_mcause(id: usize) -> Option<u64> {
    with_task(id, |task| task.last_fault_mcause)?
}

#[allow(dead_code)]
pub fn get_task_fault_mepc(id: usize) -> Option<u64> {
    with_task(id, |task| task.last_fault_mepc)?
}

#[allow(dead_code)]
pub fn get_task_fault_mtval(id: usize) -> Option<u64> {
    with_task(id, |task| task.last_fault_mtval)?
}

#[allow(dead_code)]
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

#[cfg(feature = "scheduler_dispatch_test")]
pub fn get_task_id_at_slot(slot: usize) -> Option<usize> {
    snapshot_tasks().get(slot).and_then(|task| {
        if matches!(task.state, TaskState::Empty) {
            None
        } else {
            Some(task.id)
        }
    })
}

#[allow(dead_code)]
pub fn is_task_finished(id: usize) -> bool {
    matches!(get_task_state(id), Some(TaskState::Finished))
}

pub fn is_task_faulted(id: usize) -> bool {
    matches!(get_task_state(id), Some(TaskState::Faulted))
}

#[allow(dead_code)]
pub fn is_terminal_task(id: usize) -> bool {
    is_task_finished(id) || is_task_faulted(id)
}

pub fn count_dispatchable_tasks() -> usize {
    let mut count = 0;

    for task in snapshot_tasks() {
        if matches!(task.state, TaskState::Empty) {
            continue;
        }

        if is_dispatchable_task(task.id) {
            count += 1;
        }
    }

    count
}

#[allow(dead_code)]
pub fn has_dispatchable_tasks() -> bool {
    count_dispatchable_tasks() > 0
}

pub fn find_first_task_by_state(state: TaskState) -> Option<usize> {
    for task in snapshot_tasks() {
        if matches!(task.state, TaskState::Empty) {
            continue;
        }

        if task.state == state {
            return Some(task.id);
        }
    }

    None
}

#[allow(dead_code)]
pub fn find_first_finished_task() -> Option<usize> {
    find_first_task_by_state(TaskState::Finished)
}

#[allow(dead_code)]
pub fn find_first_faulted_task() -> Option<usize> {
    find_first_task_by_state(TaskState::Faulted)
}

#[cfg(feature = "scheduler_dispatch_test")]
#[derive(Clone, Copy)]
pub struct TerminalTaskDispatchInvariantSnapshot {
    pub terminal: bool,
    pub resumable: bool,
    pub fresh_ready: bool,
    pub dispatchable: bool,
    pub result: bool,
}

#[cfg(feature = "scheduler_dispatch_test")]
pub fn get_terminal_task_dispatch_invariants(id: usize) -> TerminalTaskDispatchInvariantSnapshot {
    let terminal = is_terminal_task(id);
    let resumable = is_resumable_task(id);
    let fresh_ready = is_fresh_ready_task(id);
    let dispatchable = is_dispatchable_task(id);

    let result = terminal && !resumable && !fresh_ready && !dispatchable;

    TerminalTaskDispatchInvariantSnapshot {
        terminal,
        resumable,
        fresh_ready,
        dispatchable,
        result,
    }
}

#[cfg(feature = "scheduler_dispatch_test")]
pub fn validate_terminal_task_dispatch_invariants(id: usize) -> bool {
    get_terminal_task_dispatch_invariants(id).result
}

#[cfg(feature = "scheduler_dispatch_test")]
#[derive(Clone, Copy)]
pub struct BreakpointFaultMetadataAssertionSnapshot {
    pub reason_breakpoint: bool,
    pub mcause_breakpoint: bool,
    pub mepc_nonzero: bool,
    pub mtval_nonzero: bool,
    pub result: bool,
}

#[cfg(feature = "scheduler_dispatch_test")]
pub fn get_breakpoint_fault_metadata_assertions(
    id: usize,
) -> BreakpointFaultMetadataAssertionSnapshot {
    let reason_breakpoint = matches!(get_task_fault_reason(id), Some(TaskFaultReason::Breakpoint));

    let mcause_breakpoint = matches!(get_task_fault_mcause(id), Some(3));

    let mepc_nonzero = get_task_fault_mepc(id)
        .map(|value| value != 0)
        .unwrap_or(false);

    let mtval_nonzero = get_task_fault_mtval(id)
        .map(|value| value != 0)
        .unwrap_or(false);

    let result = reason_breakpoint && mcause_breakpoint && mepc_nonzero && mtval_nonzero;

    BreakpointFaultMetadataAssertionSnapshot {
        reason_breakpoint,
        mcause_breakpoint,
        mepc_nonzero,
        mtval_nonzero,
        result,
    }
}

#[cfg(feature = "scheduler_reentry_test")]
#[derive(Clone, Copy)]
pub struct TaskFaultCompletionSnapshot {
    pub finished_task_id: Option<usize>,
    pub faulted_task_id: Option<usize>,

    pub finished_task_finished: bool,
    pub finished_task_last_return_exit: bool,

    pub faulted_task_faulted: bool,
    pub faulted_task_last_return_fault: bool,
    pub faulted_task_resume_disabled: bool,

    pub result: bool,
}

#[cfg(feature = "scheduler_reentry_test")]
pub fn get_task_fault_completion_snapshot() -> TaskFaultCompletionSnapshot {
    let finished_task_id = find_first_finished_task();
    let faulted_task_id = find_first_faulted_task();

    let finished_task_finished = finished_task_id.map(is_task_finished).unwrap_or(false);

    let finished_task_last_return_exit = finished_task_id
        .map(|id| matches!(get_task_return_kind(id), Some(TaskReturnKind::Exit)))
        .unwrap_or(false);

    let faulted_task_faulted = faulted_task_id.map(is_task_faulted).unwrap_or(false);

    let faulted_task_last_return_fault = faulted_task_id
        .map(|id| matches!(get_task_return_kind(id), Some(TaskReturnKind::Fault)))
        .unwrap_or(false);

    let faulted_task_resume_disabled = faulted_task_id
        .map(|id| !can_task_resume(id).unwrap_or(true))
        .unwrap_or(false);

    let result = finished_task_finished
        && finished_task_last_return_exit
        && faulted_task_faulted
        && faulted_task_last_return_fault
        && faulted_task_resume_disabled;

    TaskFaultCompletionSnapshot {
        finished_task_id,
        faulted_task_id,

        finished_task_finished,
        finished_task_last_return_exit,

        faulted_task_faulted,
        faulted_task_last_return_fault,
        faulted_task_resume_disabled,

        result,
    }
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

#[allow(dead_code)]
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

pub fn mark_task_blocked_for_sleep(id: usize) -> bool {
    with_task_mut(id, |task| {
        if !matches!(task.state, TaskState::Blocked) {
            return false;
        }

        task.last_return_kind = TaskReturnKind::Sleep;
        task.can_resume = false;
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
