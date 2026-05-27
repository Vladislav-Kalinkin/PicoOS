use crate::drivers::uart;
#[cfg(feature = "scheduler_dispatch_test")]
use crate::kernel::task::cpu_context::TaskCpuContext;
use crate::kernel::task::table as task;
#[cfg(feature = "scheduler_reentry_test")]
use crate::kernel::task::table::TaskReturnSnapshot;
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

#[allow(dead_code)]
#[derive(Clone, Copy)]
pub struct NoRunnableSchedulerSnapshot {
    pub dispatchable_count: usize,
    pub has_dispatchable_tasks: bool,
    pub no_runnable: bool,
    pub result: bool,
}

#[allow(dead_code)]
pub fn get_no_runnable_scheduler_snapshot() -> NoRunnableSchedulerSnapshot {
    let dispatchable_count = crate::kernel::task::table::count_dispatchable_tasks();
    let has_dispatchable_tasks = crate::kernel::task::table::has_dispatchable_tasks();
    let no_runnable = !has_dispatchable_tasks;

    NoRunnableSchedulerSnapshot {
        dispatchable_count,
        has_dispatchable_tasks,
        no_runnable,
        result: no_runnable && dispatchable_count == 0,
    }
}

#[allow(unused_unsafe)]
#[cfg(feature = "scheduler_dispatch_test")]
fn find_next_dispatchable_task_after(start_after: Option<usize>) -> Option<usize> {
    let max_tasks = task::max_tasks();
    if max_tasks == 0 {
        return None;
    }

    let start_slot = match start_after {
        Some(id) => (id + 1) % max_tasks,
        None => 0,
    };

    #[cfg(feature = "scheduler_verbose_dispatch_trace")]
    {
        scheduler_log_str("  find_next_dispatchable_task_after: start_slot=");
        crate::drivers::uart::write_dec_u64(start_slot as u64);
        scheduler_log_line("");
    }

    for offset in 0..max_tasks {
        let slot = (start_slot + offset) % max_tasks;
        unsafe {
            // Прямой доступ к TASKS через table-функции
            let Some(task_id) = task::get_task_id_at_slot(slot) else {
                continue;
            };
            let Some(state) = task::get_task_state(task_id) else {
                continue;
            };

            #[cfg(feature = "scheduler_verbose_dispatch_trace")]
            {
                scheduler_log_str("    checking slot=");
                crate::drivers::uart::write_dec_u64(slot as u64);
                scheduler_log_str(" task_id=");
                crate::drivers::uart::write_dec_u64(task_id as u64);
                scheduler_log_str(" state=");
                task::print_task_state_by_id(task_id);
                scheduler_log_line("");
            }

            if matches!(state, task::TaskState::Empty) {
                #[cfg(feature = "scheduler_verbose_dispatch_trace")]
                scheduler_log_line("      -> empty, skip");

                continue;
            }

            let resumable = task::is_resumable_task(task_id);
            let fresh = task::is_fresh_ready_task(task_id);
            let dispatchable = resumable || fresh;

            #[cfg(feature = "scheduler_verbose_dispatch_trace")]
            {
                scheduler_log_str("      is_resumable=");
                task::print_yes_no(resumable);
                scheduler_log_str(" is_fresh_ready=");
                task::print_yes_no(fresh);
                scheduler_log_str(" is_dispatchable=");
                task::print_yes_no(dispatchable);
                scheduler_log_line("");
            }

            if dispatchable {
                #[cfg(feature = "scheduler_verbose_dispatch_trace")]
                {
                    scheduler_log_str("      -> FOUND: task_id=");
                    crate::drivers::uart::write_dec_u64(task_id as u64);
                    scheduler_log_line("");
                }
                return Some(task_id);
            }
        }
    }
    #[cfg(feature = "scheduler_verbose_dispatch_trace")]
    scheduler_log_line("  find_next_dispatchable_task_after: none found");

    None
}

#[cfg(feature = "scheduler_dispatch_test")]
#[derive(Clone, Copy, PartialEq, Eq)]
enum DispatchDecision {
    StartFresh { task_id: usize },
    ResumeSaved { task_id: usize },
    NoRunnableTask,
    Failed,
}

#[cfg(feature = "scheduler_dispatch_test")]
#[derive(Clone, Copy, PartialEq, Eq)]
enum DispatchDecisionKind {
    StartFresh,
    ResumeSaved,
    NoRunnableTask,
    Failed,
}

#[cfg(feature = "scheduler_dispatch_test")]
#[derive(Clone, Copy, PartialEq, Eq)]
enum DispatchDecisionOutcome {
    Dispatchable,
    NoRunnableTask,
    Failed,
}

#[cfg(feature = "scheduler_dispatch_test")]
impl DispatchDecision {
    fn kind(self) -> DispatchDecisionKind {
        match self {
            DispatchDecision::StartFresh { .. } => DispatchDecisionKind::StartFresh,
            DispatchDecision::ResumeSaved { .. } => DispatchDecisionKind::ResumeSaved,
            DispatchDecision::NoRunnableTask => DispatchDecisionKind::NoRunnableTask,
            DispatchDecision::Failed => DispatchDecisionKind::Failed,
        }
    }

    fn task_id(self) -> Option<usize> {
        match self {
            DispatchDecision::StartFresh { task_id }
            | DispatchDecision::ResumeSaved { task_id } => Some(task_id),
            DispatchDecision::NoRunnableTask | DispatchDecision::Failed => None,
        }
    }

    fn is_dispatchable(self) -> bool {
        self.kind().is_dispatchable()
    }

    fn outcome(self) -> DispatchDecisionOutcome {
        self.kind().outcome()
    }
}

#[cfg(feature = "scheduler_dispatch_test")]
impl DispatchDecisionKind {
    fn is_dispatchable(self) -> bool {
        self.outcome().is_dispatchable()
    }

    fn outcome(self) -> DispatchDecisionOutcome {
        match self {
            DispatchDecisionKind::StartFresh | DispatchDecisionKind::ResumeSaved => {
                DispatchDecisionOutcome::Dispatchable
            }
            DispatchDecisionKind::NoRunnableTask => DispatchDecisionOutcome::NoRunnableTask,
            DispatchDecisionKind::Failed => DispatchDecisionOutcome::Failed,
        }
    }
}

#[cfg(feature = "scheduler_dispatch_test")]
impl DispatchDecisionOutcome {
    fn label(self) -> &'static str {
        match self {
            DispatchDecisionOutcome::Dispatchable => "Dispatchable",
            DispatchDecisionOutcome::NoRunnableTask => "NoRunnableTask",
            DispatchDecisionOutcome::Failed => "Failed",
        }
    }

    fn is_dispatchable(self) -> bool {
        matches!(self, DispatchDecisionOutcome::Dispatchable)
    }
}

#[cfg(feature = "scheduler_dispatch_test")]
fn choose_dispatch_decision(task_id: usize) -> DispatchDecision {
    if task::is_resumable_task(task_id) {
        DispatchDecision::ResumeSaved { task_id }
    } else if task::is_fresh_ready_task(task_id) {
        DispatchDecision::StartFresh { task_id }
    } else {
        DispatchDecision::Failed
    }
}

#[cfg(feature = "scheduler_dispatch_test")]
fn select_dispatch_decision_after(current: Option<usize>) -> DispatchDecision {
    match find_next_dispatchable_task_after(current) {
        Some(task_id) => {
            print_dispatch_task_summary(task_id);
            choose_dispatch_decision(task_id)
        }
        None => {
            scheduler_log_line("  selected task: none");
            DispatchDecision::NoRunnableTask
        }
    }
}

#[cfg(feature = "scheduler_dispatch_test")]
fn execute_dispatch_decision(decision: DispatchDecision) -> DispatchResult {
    print_dispatch_decision(decision);

    scheduler_log_str("  dispatchable decision: ");
    task::print_yes_no(decision.is_dispatchable());
    scheduler_log_line("");
    scheduler_log_str("  dispatch outcome: ");
    scheduler_log_line(decision.outcome().label());

    match decision {
        DispatchDecision::ResumeSaved { task_id } => {
            scheduler_log_line("  dispatch action: resume task");

            force_current_task(task_id);

            resume_selected_task_checked(task_id)
        }
        DispatchDecision::StartFresh { task_id } => {
            scheduler_log_line("  dispatch action: start fresh task");

            start_selected_task_checked(task_id)
        }
        DispatchDecision::NoRunnableTask => DispatchResult::NoRunnableTask,
        DispatchDecision::Failed => {
            scheduler_log_line("  dispatch action: failed; task is not dispatchable");
            DispatchResult::Failed
        }
    }
}

#[cfg(feature = "scheduler_dispatch_test")]
fn print_dispatch_decision(decision: DispatchDecision) {
    scheduler_log_str("  dispatch decision: ");

    print_dispatch_decision_kind(decision.kind());

    if let Some(task_id) = decision.task_id() {
        scheduler_log_str("(");
        crate::drivers::uart::write_dec_u64(task_id as u64);
        scheduler_log_str(")");
    }

    scheduler_log_line("");
}

#[cfg(feature = "scheduler_dispatch_test")]
fn print_dispatch_decision_kind(kind: DispatchDecisionKind) {
    match kind {
        DispatchDecisionKind::StartFresh => scheduler_log_str("StartFresh"),
        DispatchDecisionKind::ResumeSaved => scheduler_log_str("ResumeSaved"),
        DispatchDecisionKind::NoRunnableTask => scheduler_log_str("NoRunnableTask"),
        DispatchDecisionKind::Failed => scheduler_log_str("Failed"),
    }
}

#[cfg(feature = "scheduler_dispatch_test")]
pub fn dispatch_next() -> DispatchResult {
    scheduler_log_line("");
    scheduler_log_line("scheduler dispatch_next:");

    let current = current_task_id();

    scheduler_log_str("  round-robin after: ");
    match current {
        Some(id) => task::print_task_name_by_id(id),
        None => scheduler_log_str("none"),
    }
    scheduler_log_line("");

    scheduler_log_str("  task table capacity: ");
    crate::drivers::uart::write_dec_u64(task::max_tasks() as u64);
    scheduler_log_line("");

    let decision = select_dispatch_decision_after(current);

    execute_dispatch_decision(decision)
}

#[cfg(feature = "scheduler_dispatch_test")]
fn resume_selected_task_checked(task_id: usize) -> DispatchResult {
    scheduler_log_line("  scheduler resume path: checked resume");

    scheduler_log_str("  resume task: ");
    crate::kernel::task::table::print_task_name_by_id(task_id);
    scheduler_log_line("");

    match crate::kernel::task::table::can_task_resume(task_id) {
        Some(true) => {}
        Some(false) => {
            scheduler_log_line("  resume blocked: task cannot resume");
            return DispatchResult::Failed;
        }
        None => {
            scheduler_log_line("  resume blocked: unknown task");
            return DispatchResult::Failed;
        }
    }

    let Some(frame) = validate_resume_frame(task_id) else {
        scheduler_log_line("  scheduler resume path result: FAILED");
        return DispatchResult::Failed;
    };

    scheduler_log_line("  scheduler resume path result: OK");

    restore_selected_task_checked(task_id, frame)
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

    #[cfg(any(
        feature = "two_task_resume_handoff_test",
        feature = "task_fault_test",
        feature = "scheduler_fault_lifecycle_test"
    ))]
    {
        let Some(stack_start) = crate::kernel::task::table::get_task_stack_start(task_id) else {
            scheduler_log_line("  restore result: failed; missing task stack start");
            crate::arch::halt();
        };

        let Some(stack_top) = crate::kernel::task::table::get_task_stack_top(task_id) else {
            scheduler_log_line("  restore result: failed; missing task stack top");
            crate::arch::halt();
        };

        crate::kernel::task::debug::set_debug_current_task_id(task_id);
        crate::kernel::task::debug::set_debug_current_stack_bounds(stack_start, stack_top);
    }

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
pub fn handle_task_return(snapshot: TaskReturnSnapshot) -> TaskReturnHandleResult {
    scheduler_log_line("");
    scheduler_log_line("scheduler handle_task_return:");

    print_task_return_snapshot(snapshot);

    match snapshot.last_return {
        crate::kernel::task::table::TaskReturnKind::Yield => handle_task_yield(snapshot),
        crate::kernel::task::table::TaskReturnKind::Exit => handle_task_exit(snapshot),
        crate::kernel::task::table::TaskReturnKind::Fault => handle_task_fault(snapshot),
        crate::kernel::task::table::TaskReturnKind::None => handle_task_return_none(snapshot),
    }
}

#[cfg(feature = "scheduler_reentry_test")]
fn print_task_return_snapshot(snapshot: TaskReturnSnapshot) {
    scheduler_log_str("  return snapshot task: ");
    crate::kernel::task::table::print_task_name_by_id(snapshot.task_id);
    scheduler_log_line("");

    scheduler_log_str("  return snapshot state: ");
    crate::kernel::task::table::print_task_state_by_id(snapshot.task_id);
    scheduler_log_line("");

    scheduler_log_str("  return snapshot reason: ");
    crate::kernel::task::table::print_task_return_kind_by_id(snapshot.task_id);
    scheduler_log_line("");

    scheduler_log_str("  return snapshot can_resume: ");
    crate::kernel::task::table::print_yes_no(snapshot.can_resume);
    scheduler_log_line("");
}

#[cfg(feature = "scheduler_reentry_test")]
fn handle_task_yield(snapshot: TaskReturnSnapshot) -> TaskReturnHandleResult {
    scheduler_log_line("  return action: yield -> scheduler::run");

    if !snapshot.can_resume {
        scheduler_log_line("  yield result: failed; returned task is not resumable");
        return TaskReturnHandleResult::Failed;
    }

    set_current_task(snapshot.task_id);

    scheduler_log_str("  round-robin cursor set to yielded task: ");
    task::print_task_name_by_id(snapshot.task_id);
    scheduler_log_line("");

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

#[cfg(feature = "scheduler_reentry_test")]
fn handle_task_exit(snapshot: TaskReturnSnapshot) -> TaskReturnHandleResult {
    scheduler_log_line("  return action: exit -> no resume for returned task");

    if snapshot.can_resume {
        scheduler_log_line("  exit result: failed; finished task is still resumable");
        return TaskReturnHandleResult::Failed;
    }

    set_current_task(snapshot.task_id);

    scheduler_log_str("  round-robin cursor set to exited task: ");
    task::print_task_name_by_id(snapshot.task_id);
    scheduler_log_line("");

    scheduler_log_line("  exit action: try next dispatchable task");

    match find_next_dispatchable_task_after(Some(snapshot.task_id)) {
        Some(task_id) => {
            scheduler_log_str("  next dispatchable task after exit: ");
            crate::kernel::task::table::print_task_name_by_id(task_id);
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
        None => {
            scheduler_log_line("  next dispatchable task after exit: none");
            scheduler_log_line("  result: no runnable task");
            TaskReturnHandleResult::NoRunnableTask
        }
    }
}

#[cfg(feature = "scheduler_reentry_test")]
fn handle_task_return_none(_snapshot: TaskReturnSnapshot) -> TaskReturnHandleResult {
    scheduler_log_line("  return action: none -> failed");
    TaskReturnHandleResult::Failed
}

#[cfg(feature = "scheduler_dispatch_test")]
fn start_selected_task_checked(task_id: usize) -> ! {
    scheduler_log_line("  scheduler start path: checked start");

    scheduler_log_str("  start task: ");
    task::print_task_name_by_id(task_id);
    scheduler_log_line("");

    if !task::is_fresh_ready_task(task_id) {
        scheduler_log_line("  start blocked: task is not fresh Ready");
        scheduler_start_failed();
    }

    let Some(stack_start) = task::get_task_stack_start(task_id) else {
        scheduler_log_line("  start blocked: missing stack start");
        scheduler_start_failed();
    };

    let Some(stack_top) = task::get_task_stack_top(task_id) else {
        scheduler_log_line("  start blocked: missing stack top");
        scheduler_start_failed();
    };

    let Some(entry) = task::get_task_entry(task_id) else {
        scheduler_log_line("  start blocked: missing entry");
        scheduler_start_failed();
    };

    scheduler_log_str("  start entry: ");
    crate::drivers::uart::write_hex_u64(entry as *const () as usize as u64);
    scheduler_log_line("");

    scheduler_log_str("  start stack_start: ");
    scheduler_log_hex(stack_start);
    scheduler_log_line("");

    scheduler_log_str("  start stack_top: ");
    scheduler_log_hex(stack_top);
    scheduler_log_line("");

    scheduler_log_line("  scheduler start path result: OK");

    #[cfg(any(
        feature = "two_task_resume_handoff_test",
        feature = "task_fault_test",
        feature = "scheduler_fault_lifecycle_test"
    ))]
    {
        crate::kernel::task::debug::set_debug_current_task_id(task_id);
        crate::kernel::task::debug::set_debug_current_stack_bounds(stack_start, stack_top);
    }

    crate::kernel::task::run_task_on_own_stack(task_id);
}

#[cfg(feature = "scheduler_dispatch_test")]
fn scheduler_start_failed() -> ! {
    scheduler_log_line("  scheduler start path result: FAILED");
    crate::arch::halt();
}

#[cfg(feature = "scheduler_reentry_test")]
fn handle_task_fault(snapshot: TaskReturnSnapshot) -> TaskReturnHandleResult {
    scheduler_log_line("  return action: fault -> no resume for faulted task");

    if snapshot.can_resume {
        scheduler_log_line("  fault result: failed; faulted task is still resumable");
        return TaskReturnHandleResult::Failed;
    }

    set_current_task(snapshot.task_id);

    scheduler_log_str("  round-robin cursor set to faulted task: ");
    task::print_task_name_by_id(snapshot.task_id);
    scheduler_log_line("");

    scheduler_log_line("  fault action: try next dispatchable task");

    match find_next_dispatchable_task_after(Some(snapshot.task_id)) {
        Some(task_id) => {
            scheduler_log_str("  next dispatchable task after fault: ");
            crate::kernel::task::table::print_task_name_by_id(task_id);
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
        None => {
            scheduler_log_line("  next dispatchable task after fault: none");
            scheduler_log_line("  result: no runnable task");
            TaskReturnHandleResult::NoRunnableTask
        }
    }
}
