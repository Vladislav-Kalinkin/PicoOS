use crate::drivers::uart;
use crate::kernel::memory;
use crate::kernel::task::context;
use crate::kernel::task::cpu_context::{self, TaskCpuContext};

pub const MAX_TASKS: usize = 4;

#[allow(dead_code)]
pub fn max_tasks() -> usize {
    MAX_TASKS
}

pub const TASK_NAME_LEN: usize = 16;
static mut LAST_RETURNED_TASK_ID: Option<usize> = None;

pub type TaskEntry = fn();

#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq)]
pub enum TaskState {
    Empty,
    Ready,
    Running,
    Blocked,
    Finished,
    Faulted,
}

#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq)]
pub enum TaskReturnKind {
    None,
    Exit,
    Yield,
    Fault,
}

#[allow(dead_code)]
#[cfg(feature = "scheduler_reentry_test")]
#[derive(Clone, Copy)]
pub struct TaskReturnSnapshot {
    pub task_id: usize,
    pub state: TaskState,
    pub last_return: TaskReturnKind,
    pub can_resume: bool,
}

#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TaskFaultReason {
    Breakpoint,
    InstructionAccessFault,
    LoadAccessFault,
    StoreAccessFault,
    IllegalInstruction,
    Unknown(u64),
}

#[allow(dead_code)]
impl TaskFaultReason {
    /// Конвертирует значение mcause в конкретную причину fault.
    /// Согласно RISC-V Privileged Spec, Table 16 (synchronous exceptions).
    pub fn from_mcause(cause: u64) -> Self {
        match cause {
            1 => TaskFaultReason::InstructionAccessFault,
            2 => TaskFaultReason::IllegalInstruction,
            3 => TaskFaultReason::Breakpoint,
            5 => TaskFaultReason::LoadAccessFault,
            7 => TaskFaultReason::StoreAccessFault,
            other => TaskFaultReason::Unknown(other),
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
    pub last_fault_reason: Option<TaskFaultReason>,
    pub last_fault_mcause: Option<u64>,
    pub last_fault_mepc: Option<u64>,
    pub last_fault_mtval: Option<u64>,
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
        }
    }
}

static mut TASKS: [Task; MAX_TASKS] = [Task::empty(); MAX_TASKS];
static mut NEXT_TASK_ID: usize = 0;

#[allow(dead_code)]
pub fn init() {
    unsafe {
        TASKS = [Task::empty(); MAX_TASKS];
        NEXT_TASK_ID = 0;
    }

    uart::write_line("");
    uart::write_line("task system:");
    uart::write_str("max tasks: ");
    uart::write_dec_u64(MAX_TASKS as u64);
    uart::write_line("");
}

#[allow(clippy::needless_range_loop)]
pub fn create_task(name: &str, entry: TaskEntry) -> Option<usize> {
    unsafe {
        for slot in 0..MAX_TASKS {
            if matches!(TASKS[slot].state, TaskState::Empty) {
                let Some(stack_start) = memory::allocate_page() else {
                    uart::write_str("failed to allocate stack for task: ");
                    uart::write_line(name);
                    return None;
                };

                let stack_top = stack_start + memory::PAGE_SIZE;
                let initial_sp = stack_top;
                let initial_pc = entry as *const () as usize as u64;

                let prepared_sp = context::prepare_initial_stack(stack_top, initial_pc);

                let saved_sp = prepared_sp;
                let saved_pc = initial_pc;

                let id = NEXT_TASK_ID;
                NEXT_TASK_ID += 1;

                TASKS[slot].id = id;
                TASKS[slot].state = TaskState::Ready;
                TASKS[slot].stack_start = stack_start;
                TASKS[slot].stack_top = stack_top;
                TASKS[slot].entry = Some(entry);
                TASKS[slot].initial_sp = initial_sp;
                TASKS[slot].initial_pc = initial_pc;
                TASKS[slot].saved_sp = saved_sp;
                TASKS[slot].saved_pc = saved_pc;
                TASKS[slot].cpu_context = TaskCpuContext::initial(saved_sp, saved_pc);
                TASKS[slot].last_kernel_sp = 0;
                TASKS[slot].last_kernel_return_pc = 0;
                TASKS[slot].last_task_sp = 0;
                TASKS[slot].has_started = false;
                TASKS[slot].can_resume = false;
                TASKS[slot].last_return_kind = TaskReturnKind::None;

                copy_name(&mut TASKS[slot].name, name);

                uart::write_str("created task: ");
                write_name(&TASKS[slot].name);

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

                return Some(id);
            }
        }
    }

    None
}

#[allow(clippy::needless_range_loop)]
pub fn print_tasks() {
    uart::write_line("task list:");

    unsafe {
        for slot in 0..MAX_TASKS {
            let task = TASKS[slot];

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
}

#[allow(dead_code)]
#[allow(clippy::needless_range_loop)]
pub fn task_count() -> usize {
    let mut count = 0;

    unsafe {
        for slot in 0..MAX_TASKS {
            if !matches!(TASKS[slot].state, TaskState::Empty) {
                count += 1;
            }
        }
    }

    count
}

#[allow(clippy::needless_range_loop)]
pub fn get_task_name(id: usize) -> Option<[u8; TASK_NAME_LEN]> {
    unsafe {
        for slot in 0..MAX_TASKS {
            if !matches!(TASKS[slot].state, TaskState::Empty) && TASKS[slot].id == id {
                return Some(TASKS[slot].name);
            }
        }
    }

    None
}

#[allow(clippy::needless_range_loop)]
pub fn get_task_entry(id: usize) -> Option<TaskEntry> {
    unsafe {
        for slot in 0..MAX_TASKS {
            if !matches!(TASKS[slot].state, TaskState::Empty) && TASKS[slot].id == id {
                return TASKS[slot].entry;
            }
        }
    }

    None
}

#[allow(dead_code)]
#[allow(clippy::needless_range_loop)]
pub fn get_task_stack_start(id: usize) -> Option<u64> {
    unsafe {
        for slot in 0..MAX_TASKS {
            if !matches!(TASKS[slot].state, TaskState::Empty) && TASKS[slot].id == id {
                return Some(TASKS[slot].stack_start);
            }
        }
    }

    None
}

#[allow(dead_code)]
#[allow(clippy::needless_range_loop)]
pub fn get_task_stack_top(id: usize) -> Option<u64> {
    unsafe {
        for slot in 0..MAX_TASKS {
            if !matches!(TASKS[slot].state, TaskState::Empty) && TASKS[slot].id == id {
                return Some(TASKS[slot].stack_top);
            }
        }
    }

    None
}

#[allow(clippy::needless_range_loop)]
pub fn get_task_initial_sp(id: usize) -> Option<u64> {
    unsafe {
        for slot in 0..MAX_TASKS {
            if !matches!(TASKS[slot].state, TaskState::Empty) && TASKS[slot].id == id {
                return Some(TASKS[slot].initial_sp);
            }
        }
    }

    None
}

#[allow(clippy::needless_range_loop)]
pub fn get_task_initial_pc(id: usize) -> Option<u64> {
    unsafe {
        for slot in 0..MAX_TASKS {
            if !matches!(TASKS[slot].state, TaskState::Empty) && TASKS[slot].id == id {
                return Some(TASKS[slot].initial_pc);
            }
        }
    }

    None
}

#[allow(clippy::needless_range_loop)]
pub fn get_task_saved_sp(id: usize) -> Option<u64> {
    unsafe {
        for slot in 0..MAX_TASKS {
            if !matches!(TASKS[slot].state, TaskState::Empty) && TASKS[slot].id == id {
                return Some(TASKS[slot].saved_sp);
            }
        }
    }

    None
}

#[allow(clippy::needless_range_loop)]
pub fn get_task_saved_pc(id: usize) -> Option<u64> {
    unsafe {
        for slot in 0..MAX_TASKS {
            if !matches!(TASKS[slot].state, TaskState::Empty) && TASKS[slot].id == id {
                return Some(TASKS[slot].saved_pc);
            }
        }
    }

    None
}

#[allow(clippy::needless_range_loop)]
pub fn find_next_ready_after(current_id: Option<usize>) -> Option<usize> {
    let start_slot = match current_id {
        Some(id) => find_slot_by_id(id).map(|slot| slot + 1).unwrap_or(0),

        None => 0,
    };

    for offset in 0..MAX_TASKS {
        let slot = (start_slot + offset) % MAX_TASKS;

        unsafe {
            if matches!(TASKS[slot].state, TaskState::Ready) {
                return Some(TASKS[slot].id);
            }
        }
    }

    None
}

#[allow(clippy::needless_range_loop)]
pub fn set_running(id: usize) {
    unsafe {
        for slot in 0..MAX_TASKS {
            if matches!(TASKS[slot].state, TaskState::Running) && TASKS[slot].id != id {
                TASKS[slot].state = TaskState::Ready;
            }
        }

        for slot in 0..MAX_TASKS {
            if !matches!(TASKS[slot].state, TaskState::Empty) && TASKS[slot].id == id {
                TASKS[slot].state = TaskState::Running;
                return;
            }
        }
    }
}

#[allow(clippy::needless_range_loop)]
pub fn update_context(id: usize, saved_sp: u64, saved_pc: u64) -> bool {
    unsafe {
        for slot in 0..MAX_TASKS {
            if !matches!(TASKS[slot].state, TaskState::Empty) && TASKS[slot].id == id {
                TASKS[slot].saved_sp = saved_sp;
                TASKS[slot].saved_pc = saved_pc;
                TASKS[slot].has_started = true;
                return true;
            }
        }
    }

    false
}

#[allow(dead_code)]
#[allow(clippy::needless_range_loop)]
pub fn mark_started(id: usize) -> bool {
    unsafe {
        for slot in 0..MAX_TASKS {
            if !matches!(TASKS[slot].state, TaskState::Empty) && TASKS[slot].id == id {
                TASKS[slot].has_started = true;
                return true;
            }
        }
    }

    false
}

#[allow(clippy::needless_range_loop)]
pub fn has_started(id: usize) -> bool {
    unsafe {
        for slot in 0..MAX_TASKS {
            if !matches!(TASKS[slot].state, TaskState::Empty) && TASKS[slot].id == id {
                return TASKS[slot].has_started;
            }
        }
    }

    false
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
    let Some(slot) = find_slot_by_id(id) else {
        uart::write_line("  fault info: task not found");
        return;
    };

    let task = unsafe { TASKS[slot] };

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

#[allow(clippy::needless_range_loop)]
fn find_slot_by_id(id: usize) -> Option<usize> {
    unsafe {
        for slot in 0..MAX_TASKS {
            if !matches!(TASKS[slot].state, TaskState::Empty) && TASKS[slot].id == id {
                return Some(slot);
            }
        }
    }

    None
}

// Keep this manual copy for early bare-metal safety.
// Slice copy/fill caused an early ARM64 exception during task creation.
#[allow(clippy::manual_memcpy)]
fn copy_name(dst: &mut [u8; TASK_NAME_LEN], name: &str) {
    let mut i = 0;

    while i < TASK_NAME_LEN {
        dst[i] = 0;
        i += 1;
    }

    let bytes = name.as_bytes();
    let len = min(bytes.len(), TASK_NAME_LEN - 1);

    i = 0;

    while i < len {
        dst[i] = bytes[i];
        i += 1;
    }
}

fn min(a: usize, b: usize) -> usize {
    if a < b {
        a
    } else {
        b
    }
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

#[allow(clippy::needless_range_loop)]
pub fn set_task_state(id: usize, state: TaskState) -> bool {
    unsafe {
        for slot in 0..MAX_TASKS {
            if !matches!(TASKS[slot].state, TaskState::Empty) && TASKS[slot].id == id {
                TASKS[slot].state = state;
                return true;
            }
        }
    }

    false
}

#[allow(clippy::needless_range_loop)]
pub fn get_task_state(id: usize) -> Option<TaskState> {
    unsafe {
        for slot in 0..MAX_TASKS {
            if !matches!(TASKS[slot].state, TaskState::Empty) && TASKS[slot].id == id {
                return Some(TASKS[slot].state);
            }
        }
    }

    None
}

pub fn print_task_state_by_id(id: usize) {
    match get_task_state(id) {
        Some(state) => print_state(state),
        None => uart::write_str("unknown"),
    }
}

#[allow(clippy::needless_range_loop)]
pub fn set_task_return_kind(id: usize, kind: TaskReturnKind) -> bool {
    unsafe {
        for slot in 0..MAX_TASKS {
            if !matches!(TASKS[slot].state, TaskState::Empty) && TASKS[slot].id == id {
                TASKS[slot].last_return_kind = kind;
                return true;
            }
        }
    }

    false
}

#[allow(dead_code)]
#[allow(clippy::needless_range_loop)]
pub fn get_task_return_kind(id: usize) -> Option<TaskReturnKind> {
    unsafe {
        for slot in 0..MAX_TASKS {
            if !matches!(TASKS[slot].state, TaskState::Empty) && TASKS[slot].id == id {
                return Some(TASKS[slot].last_return_kind);
            }
        }
    }

    None
}

pub fn print_task_return_kind(kind: TaskReturnKind) {
    match kind {
        TaskReturnKind::None => uart::write_str("None"),
        TaskReturnKind::Exit => uart::write_str("Exit"),
        TaskReturnKind::Yield => uart::write_str("Yield"),
        TaskReturnKind::Fault => uart::write_str("Fault"),
    }
}

pub fn print_task_return_kind_by_id(id: usize) {
    match get_task_return_kind(id) {
        Some(kind) => print_task_return_kind(kind),
        None => uart::write_str("unknown"),
    }
}

#[allow(clippy::needless_range_loop)]
pub fn set_task_last_return_context(
    id: usize,
    task_sp: u64,
    kernel_sp: u64,
    kernel_return_pc: u64,
) -> bool {
    unsafe {
        for slot in 0..MAX_TASKS {
            if !matches!(TASKS[slot].state, TaskState::Empty) && TASKS[slot].id == id {
                TASKS[slot].last_task_sp = task_sp;
                TASKS[slot].last_kernel_sp = kernel_sp;
                TASKS[slot].last_kernel_return_pc = kernel_return_pc;
                return true;
            }
        }
    }

    false
}

#[allow(clippy::needless_range_loop)]
pub fn is_sp_inside_task_stack(id: usize, sp: u64) -> Option<bool> {
    unsafe {
        for slot in 0..MAX_TASKS {
            if !matches!(TASKS[slot].state, TaskState::Empty) && TASKS[slot].id == id {
                return Some(sp >= TASKS[slot].stack_start && sp < TASKS[slot].stack_top);
            }
        }
    }

    None
}

#[allow(clippy::needless_range_loop)]
pub fn set_task_can_resume(id: usize, can_resume: bool) -> bool {
    unsafe {
        for slot in 0..MAX_TASKS {
            if !matches!(TASKS[slot].state, TaskState::Empty) && TASKS[slot].id == id {
                TASKS[slot].can_resume = can_resume;
                return true;
            }
        }
    }

    false
}

#[allow(dead_code)]
#[allow(clippy::needless_range_loop)]
pub fn can_task_resume(id: usize) -> Option<bool> {
    unsafe {
        for slot in 0..MAX_TASKS {
            if !matches!(TASKS[slot].state, TaskState::Empty) && TASKS[slot].id == id {
                return Some(TASKS[slot].can_resume);
            }
        }
    }

    None
}

#[allow(dead_code)]
#[allow(clippy::needless_range_loop)]
pub fn get_task_last_task_sp(id: usize) -> Option<u64> {
    unsafe {
        for slot in 0..MAX_TASKS {
            if !matches!(TASKS[slot].state, TaskState::Empty) && TASKS[slot].id == id {
                return Some(TASKS[slot].last_task_sp);
            }
        }
    }

    None
}

#[allow(dead_code)]
#[allow(clippy::needless_range_loop)]
pub fn get_task_last_kernel_sp(id: usize) -> Option<u64> {
    unsafe {
        for slot in 0..MAX_TASKS {
            if !matches!(TASKS[slot].state, TaskState::Empty) && TASKS[slot].id == id {
                return Some(TASKS[slot].last_kernel_sp);
            }
        }
    }

    None
}

#[allow(dead_code)]
#[allow(clippy::needless_range_loop)]
pub fn get_task_last_kernel_return_pc(id: usize) -> Option<u64> {
    unsafe {
        for slot in 0..MAX_TASKS {
            if !matches!(TASKS[slot].state, TaskState::Empty) && TASKS[slot].id == id {
                return Some(TASKS[slot].last_kernel_return_pc);
            }
        }
    }

    None
}

#[allow(dead_code)]
#[allow(clippy::needless_range_loop)]
pub fn find_first_resumable_task() -> Option<usize> {
    unsafe {
        for slot in 0..MAX_TASKS {
            let task = TASKS[slot];

            if matches!(task.state, TaskState::Ready)
                && task.can_resume
                && matches!(task.last_return_kind, TaskReturnKind::Yield)
                && task.last_task_sp >= task.stack_start
                && task.last_task_sp < task.stack_top
            {
                return Some(task.id);
            }
        }
    }

    None
}

#[allow(dead_code)]
pub fn print_yes_no(value: bool) {
    if value {
        uart::write_str("yes");
    } else {
        uart::write_str("no");
    }
}

#[allow(clippy::needless_range_loop)]
pub fn set_task_cpu_context(id: usize, context: TaskCpuContext) -> bool {
    unsafe {
        for slot in 0..MAX_TASKS {
            if !matches!(TASKS[slot].state, TaskState::Empty) && TASKS[slot].id == id {
                TASKS[slot].cpu_context = context;
                TASKS[slot].saved_sp = context.sp;
                TASKS[slot].saved_pc = context.return_pc;
                return true;
            }
        }
    }

    false
}

#[allow(dead_code)]
#[allow(clippy::needless_range_loop)]
pub fn get_task_cpu_context(id: usize) -> Option<TaskCpuContext> {
    unsafe {
        for slot in 0..MAX_TASKS {
            if !matches!(TASKS[slot].state, TaskState::Empty) && TASKS[slot].id == id {
                return Some(TASKS[slot].cpu_context);
            }
        }
    }

    None
}

#[allow(dead_code)]
pub fn get_task_resume_pc(id: usize) -> Option<u64> {
    get_task_cpu_context(id).map(|context| context.resume_pc)
}

#[allow(dead_code)]
pub fn get_task_entry_addr(id: usize) -> Option<u64> {
    get_task_entry(id).map(|entry| entry as *const () as usize as u64)
}

pub fn get_task_resume_frame(
    id: usize,
) -> Option<crate::kernel::task::cpu_context::TaskCpuContext> {
    get_task_cpu_context(id)
}

#[allow(dead_code)]
pub fn print_task_resume_frame_by_id(id: usize) {
    match get_task_resume_frame(id) {
        Some(frame) => crate::kernel::task::cpu_context::print_cpu_context(frame),
        None => uart::write_str("none"),
    }
}

#[allow(dead_code)]
pub fn set_last_returned_task_id(id: usize) {
    unsafe {
        LAST_RETURNED_TASK_ID = Some(id);
    }
}

#[allow(dead_code)]
pub fn get_last_returned_task_id() -> Option<usize> {
    unsafe { LAST_RETURNED_TASK_ID }
}

#[cfg(feature = "scheduler_reentry_test")]
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

#[cfg(feature = "scheduler_reentry_test")]
pub fn get_last_returned_task_snapshot() -> Option<TaskReturnSnapshot> {
    let id = get_last_returned_task_id()?;
    get_task_return_snapshot(id)
}

#[allow(dead_code)]
pub fn is_resumable_task(id: usize) -> bool {
    matches!(get_task_state(id), Some(TaskState::Ready))
        && matches!(can_task_resume(id), Some(true))
        && matches!(get_task_return_kind(id), Some(TaskReturnKind::Yield))
        && get_task_resume_frame(id)
            .map(|frame| frame.is_valid())
            .unwrap_or(false)
}

#[allow(dead_code)]
pub fn is_fresh_ready_task(id: usize) -> bool {
    matches!(get_task_state(id), Some(TaskState::Ready))
        && !has_started(id)
        && matches!(can_task_resume(id), Some(false))
}

#[allow(dead_code)]
pub fn is_dispatchable_task(id: usize) -> bool {
    is_resumable_task(id) || is_fresh_ready_task(id)
}

#[allow(dead_code)]
#[allow(clippy::needless_range_loop)]
pub fn set_task_fault_info(
    id: usize,
    reason: TaskFaultReason,
    mcause: u64,
    mepc: u64,
    mtval: u64,
) -> bool {
    unsafe {
        for slot in 0..MAX_TASKS {
            if !matches!(TASKS[slot].state, TaskState::Empty) && TASKS[slot].id == id {
                TASKS[slot].last_fault_reason = Some(reason);
                TASKS[slot].last_fault_mcause = Some(mcause);
                TASKS[slot].last_fault_mepc = Some(mepc);
                TASKS[slot].last_fault_mtval = Some(mtval);
                return true;
            }
        }
    }
    false
}

#[allow(dead_code)]
#[allow(clippy::needless_range_loop)]
pub fn get_task_fault_reason(id: usize) -> Option<TaskFaultReason> {
    unsafe {
        for slot in 0..MAX_TASKS {
            if !matches!(TASKS[slot].state, TaskState::Empty) && TASKS[slot].id == id {
                return TASKS[slot].last_fault_reason;
            }
        }
    }
    None
}

#[allow(dead_code)]
#[allow(clippy::needless_range_loop)]
pub fn get_task_fault_mcause(id: usize) -> Option<u64> {
    unsafe {
        for slot in 0..MAX_TASKS {
            if !matches!(TASKS[slot].state, TaskState::Empty) && TASKS[slot].id == id {
                return TASKS[slot].last_fault_mcause;
            }
        }
    }

    None
}

#[allow(dead_code)]
#[allow(clippy::needless_range_loop)]
pub fn get_task_fault_mepc(id: usize) -> Option<u64> {
    unsafe {
        for slot in 0..MAX_TASKS {
            if !matches!(TASKS[slot].state, TaskState::Empty) && TASKS[slot].id == id {
                return TASKS[slot].last_fault_mepc;
            }
        }
    }

    None
}

#[allow(dead_code)]
#[allow(clippy::needless_range_loop)]
pub fn get_task_fault_mtval(id: usize) -> Option<u64> {
    unsafe {
        for slot in 0..MAX_TASKS {
            if !matches!(TASKS[slot].state, TaskState::Empty) && TASKS[slot].id == id {
                return TASKS[slot].last_fault_mtval;
            }
        }
    }

    None
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

#[allow(dead_code)]
pub fn get_task_id_at_slot(slot: usize) -> Option<usize> {
    if slot >= MAX_TASKS {
        return None;
    }
    unsafe {
        if matches!(TASKS[slot].state, TaskState::Empty) {
            None
        } else {
            Some(TASKS[slot].id)
        }
    }
}

#[allow(dead_code)]
pub fn is_task_finished(id: usize) -> bool {
    matches!(get_task_state(id), Some(TaskState::Finished))
}

#[allow(dead_code)]
pub fn is_task_faulted(id: usize) -> bool {
    matches!(get_task_state(id), Some(TaskState::Faulted))
}

#[allow(dead_code)]
pub fn is_terminal_task(id: usize) -> bool {
    is_task_finished(id) || is_task_faulted(id)
}

#[allow(dead_code)]
#[allow(clippy::needless_range_loop)]
pub fn count_dispatchable_tasks() -> usize {
    let mut count = 0;

    unsafe {
        for slot in 0..MAX_TASKS {
            if matches!(TASKS[slot].state, TaskState::Empty) {
                continue;
            }

            if is_dispatchable_task(TASKS[slot].id) {
                count += 1;
            }
        }
    }

    count
}

#[allow(dead_code)]
pub fn has_dispatchable_tasks() -> bool {
    count_dispatchable_tasks() > 0
}
