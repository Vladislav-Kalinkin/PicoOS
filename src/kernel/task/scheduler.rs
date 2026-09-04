use crate::drivers::uart;
use crate::kernel::irq_cell::IrqCell;
use crate::kernel::task::cpu_context::TaskCpuContext;
use crate::kernel::task::table as task;
use crate::kernel::task::table::TaskReturnSnapshot;
use crate::kernel::trap_frame::TrapImage;

static CURRENT_TASK_ID: IrqCell<Option<usize>> = IrqCell::new(None);
static DEFAULT_SEEN_YIELD: IrqCell<bool> = IrqCell::new(false);
static DEFAULT_SEEN_SLEEP: IrqCell<bool> = IrqCell::new(false);
static DEFAULT_MARKER_PRINTED: IrqCell<bool> = IrqCell::new(false);
#[cfg(feature = "scenario_resume")]
static U_RESUME_YIELDS: IrqCell<u32> = IrqCell::new(0);
#[cfg(feature = "scenario_resume")]
static U_RESUME_MARKER_PRINTED: IrqCell<bool> = IrqCell::new(false);

#[allow(dead_code)]
pub fn init() {
    CURRENT_TASK_ID.with(|current| *current = None);

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

#[allow(dead_code)]
pub fn schedule_next() -> Option<usize> {
    let current = CURRENT_TASK_ID.with(|id| *id);
    let next = task::find_next_ready_after(current)?;

    if !task::mark_task_running(next) {
        return None;
    }

    CURRENT_TASK_ID.with(|id| *id = Some(next));

    Some(next)
}

pub fn current_task_id() -> Option<usize> {
    CURRENT_TASK_ID.with(|id| *id)
}

#[allow(dead_code)]
pub fn print_current_task_name() {
    match current_task_id() {
        Some(id) => task::print_task_name_by_id(id),
        None => uart::write_str("none"),
    }
}

#[allow(dead_code)]
pub fn print_current_task_entry() {
    match current_task_id() {
        Some(id) => task::print_task_entry_by_id(id),
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

pub fn force_current_task(id: usize) {
    if !task::mark_task_running(id) {
        return;
    }

    set_round_robin_cursor(id);
}

fn set_round_robin_cursor(id: usize) {
    CURRENT_TASK_ID.with(|current| *current = Some(id));
}

pub fn switch_to_idle() {
    CURRENT_TASK_ID.with(|id| *id = None);
    crate::kernel::cpu::clear_current();
}

pub fn idle_loop() -> ! {
    switch_to_idle();
    loop {
        crate::arch::wait_for_interrupt();
    }
}

pub fn build_fresh_trap_image(task_id: usize) -> Option<TrapImage> {
    let entry = task::get_task_entry(task_id)?;
    let stack_top = task::get_task_stack_top(task_id)?;
    let mut image = TrapImage::empty();
    image.gpr.sp = stack_top;
    image.gpr.a0 = entry as *const () as usize as u64;
    image.mepc = crate::kernel::task::entry::task_trampoline_raw as *const () as usize as u64;
    Some(image)
}

pub fn trap_image_for_resume(task_id: usize) -> Option<TrapImage> {
    if let Some(image) = task::get_task_trap_image(task_id) {
        return Some(image);
    }
    task::get_task_cpu_context(task_id).map(TrapImage::from_yield_context)
}

/// Pick the next worker for a timer interrupt. `after` is the interrupted
/// worker (`Cpu.current`), not the idle WFI context.
pub fn prepare_timer_switch(after: Option<usize>) -> Option<usize> {
    find_next_dispatchable_task_after(after)
}

pub fn arm_worker_for_mret(task_id: usize, fresh: bool) {
    force_current_task(task_id);

    let Some(stack_start) = task::get_task_stack_start(task_id) else {
        crate::arch::halt();
    };
    let Some(stack_top) = task::get_task_stack_top(task_id) else {
        crate::arch::halt();
    };

    crate::kernel::cpu::set_current(task_id);
    crate::kernel::cpu::set_current_stack_bounds(stack_start, stack_top);
    crate::arch::riscv64::pmp::set_current_stack(stack_start);

    if fresh {
        let _ = task::mark_task_started(task_id);
        crate::kernel::cpu::set_kernel_sp_before_task(crate::kernel::memory::stack_top());
        crate::kernel::cpu::set_kernel_return_pc(
            crate::kernel::task::debug::task_return_point as *const () as usize as u64,
        );
    }
}

pub fn note_default_image_return(kind: crate::kernel::task::table::TaskReturnKind) {
    match kind {
        crate::kernel::task::table::TaskReturnKind::Yield => {
            DEFAULT_SEEN_YIELD.with(|seen| *seen = true);
            #[cfg(feature = "scenario_resume")]
            U_RESUME_YIELDS.with(|count| *count = count.saturating_add(1));
        }
        crate::kernel::task::table::TaskReturnKind::Sleep => {
            DEFAULT_SEEN_SLEEP.with(|seen| *seen = true);
        }
        crate::kernel::task::table::TaskReturnKind::Exit => {
            #[cfg(feature = "scenario_resume")]
            {
                let yields = U_RESUME_YIELDS.with(|count| *count);
                let already = U_RESUME_MARKER_PRINTED.with(|printed| *printed);
                if yields >= 2 && !already {
                    U_RESUME_MARKER_PRINTED.with(|printed| *printed = true);
                    uart::write_line("scheduler resume loop result: OK");
                }
            }
        }
        _ => {}
    }

    let yield_seen = DEFAULT_SEEN_YIELD.with(|seen| *seen);
    let sleep_seen = DEFAULT_SEEN_SLEEP.with(|seen| *seen);
    let already = DEFAULT_MARKER_PRINTED.with(|printed| *printed);
    if yield_seen && sleep_seen && !already {
        DEFAULT_MARKER_PRINTED.with(|printed| *printed = true);
        uart::write_line("default scheduler: yield and sleep OK");
    }
}

#[allow(dead_code)]
pub fn set_current_task(id: usize) {
    force_current_task(id);
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DispatchResult {
    NoRunnableTask,
    Failed,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RunOnceResult {
    NoRunnableTask,
    Failed,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RunResult {
    NoRunnableTask,
    Failed,
}

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

fn find_next_dispatchable_task_after(start_after: Option<usize>) -> Option<usize> {
    #[cfg(feature = "scheduler_verbose_dispatch_trace")]
    {
        scheduler_log_str("  find_next_dispatchable_task_after: after_task_id=");
        match start_after {
            Some(id) => crate::drivers::uart::write_dec_u64(id as u64),
            None => scheduler_log_str("none"),
        }
        scheduler_log_line("");
    }

    let selected = task::find_next_dispatchable_after(start_after);

    #[cfg(feature = "scheduler_verbose_dispatch_trace")]
    match selected {
        Some(task_id) => {
            scheduler_log_str("      -> FOUND: task_id=");
            crate::drivers::uart::write_dec_u64(task_id as u64);
            scheduler_log_line("");
        }
        None => scheduler_log_line("  find_next_dispatchable_task_after: none found"),
    }

    selected
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DispatchDecision {
    StartFresh { task_id: usize },
    ResumeSaved { task_id: usize },
    NoRunnableTask,
    Failed,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DispatchDecisionKind {
    StartFresh,
    ResumeSaved,
    NoRunnableTask,
    Failed,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DispatchDecisionOutcome {
    Dispatchable,
    NoRunnableTask,
    Failed,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DispatchCandidate {
    Task { task_id: usize },
    None,
}

#[derive(Clone, Copy)]
struct DispatchPipeline {
    current: Option<usize>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DispatchCandidateKind {
    Task,
    None,
}

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

impl DispatchCandidate {
    fn kind(self) -> DispatchCandidateKind {
        match self {
            DispatchCandidate::Task { .. } => DispatchCandidateKind::Task,
            DispatchCandidate::None => DispatchCandidateKind::None,
        }
    }

    fn task_id(self) -> Option<usize> {
        match self {
            DispatchCandidate::Task { task_id } => Some(task_id),
            DispatchCandidate::None => None,
        }
    }

    fn decision(self) -> DispatchDecision {
        match self.task_id() {
            Some(task_id) => build_dispatch_decision_for_task(task_id),
            None => DispatchDecision::NoRunnableTask,
        }
    }
}

impl DispatchCandidateKind {
    fn label(self) -> &'static str {
        match self {
            DispatchCandidateKind::Task => "Task",
            DispatchCandidateKind::None => "None",
        }
    }
}

impl DispatchPipeline {
    fn new(current: Option<usize>) -> Self {
        Self { current }
    }

    fn candidate(self) -> DispatchCandidate {
        select_dispatch_candidate_after(self.current)
    }

    fn decision_from_candidate(self, candidate: DispatchCandidate) -> DispatchDecision {
        select_dispatch_decision_from_candidate(candidate)
    }

    fn decision(self) -> DispatchDecision {
        let candidate = self.candidate();

        self.decision_from_candidate(candidate)
    }

    fn run(self) -> DispatchResult {
        let decision = self.decision();

        execute_dispatch_decision(decision)
    }
}

fn build_dispatch_decision_for_task(task_id: usize) -> DispatchDecision {
    if task::is_resumable_task(task_id) {
        DispatchDecision::ResumeSaved { task_id }
    } else if task::is_fresh_ready_task(task_id) {
        DispatchDecision::StartFresh { task_id }
    } else {
        DispatchDecision::Failed
    }
}

fn print_dispatch_candidate(candidate: DispatchCandidate) {
    scheduler_log_str("  dispatch candidate: ");
    scheduler_log_str(candidate.kind().label());

    if let Some(task_id) = candidate.task_id() {
        scheduler_log_str("(");
        crate::drivers::uart::write_dec_u64(task_id as u64);
        scheduler_log_str(")");
    }

    scheduler_log_line("");
}

fn select_dispatch_decision_from_candidate(candidate: DispatchCandidate) -> DispatchDecision {
    print_dispatch_candidate(candidate);

    match candidate.task_id() {
        Some(task_id) => {
            print_dispatch_task_summary(task_id);
        }
        None => {
            scheduler_log_line("  selected task: none");
        }
    }

    candidate.decision()
}

fn select_dispatch_candidate_after(current: Option<usize>) -> DispatchCandidate {
    match find_next_dispatchable_task_after(current) {
        Some(task_id) => DispatchCandidate::Task { task_id },
        None => DispatchCandidate::None,
    }
}

fn execute_dispatch_decision(decision: DispatchDecision) -> DispatchResult {
    print_dispatch_decision_model(decision);

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

fn run_dispatch_pipeline_after(current: Option<usize>) -> DispatchResult {
    DispatchPipeline::new(current).run()
}

fn print_dispatch_decision_model(decision: DispatchDecision) {
    print_dispatch_decision(decision);

    scheduler_log_str("  dispatchable decision: ");
    task::print_yes_no(decision.is_dispatchable());
    scheduler_log_line("");

    scheduler_log_str("  dispatch outcome: ");
    scheduler_log_line(decision.outcome().label());
}

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

fn print_dispatch_decision_kind(kind: DispatchDecisionKind) {
    match kind {
        DispatchDecisionKind::StartFresh => scheduler_log_str("StartFresh"),
        DispatchDecisionKind::ResumeSaved => scheduler_log_str("ResumeSaved"),
        DispatchDecisionKind::NoRunnableTask => scheduler_log_str("NoRunnableTask"),
        DispatchDecisionKind::Failed => scheduler_log_str("Failed"),
    }
}

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

    run_dispatch_pipeline_after(current)
}

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

    let Some(stack_start) = crate::kernel::task::table::get_task_stack_start(task_id) else {
        scheduler_log_line("  restore result: failed; missing task stack start");
        crate::arch::halt();
    };

    let Some(stack_top) = crate::kernel::task::table::get_task_stack_top(task_id) else {
        scheduler_log_line("  restore result: failed; missing task stack top");
        crate::arch::halt();
    };

    crate::kernel::cpu::set_current(task_id);
    crate::kernel::cpu::set_current_stack_bounds(stack_start, stack_top);

    scheduler_log_line("  calling arch restore from scheduler path...");

    if let Some(image) = task::get_task_trap_image(task_id) {
        crate::arch::mret_to_trap_image(&image);
    }

    crate::arch::restore_verified_resume_frame(frame);
}

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

fn scheduler_log_line(message: &str) {
    crate::kernel::log::trace("scheduler", message);
}

const fn scheduler_uart() -> bool {
    cfg!(any(
        feature = "task_yield_test",
        feature = "scheduler_verbose_dispatch_trace"
    ))
}

fn scheduler_log_str(message: &str) {
    if scheduler_uart() {
        crate::drivers::uart::write_str(message);
    }
}

fn scheduler_log_hex(value: u64) {
    if scheduler_uart() {
        crate::drivers::uart::write_hex_u64(value);
    }
}

fn scheduler_log_yes_no(value: bool) {
    if scheduler_uart() {
        crate::kernel::task::table::print_yes_no(value);
    }
}

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

pub fn handle_task_return(snapshot: TaskReturnSnapshot) -> TaskReturnHandleResult {
    scheduler_log_line("");
    scheduler_log_line("scheduler handle_task_return:");

    print_task_return_snapshot(snapshot);

    match snapshot.last_return {
        crate::kernel::task::table::TaskReturnKind::Yield => handle_task_yield(snapshot),
        crate::kernel::task::table::TaskReturnKind::Sleep => handle_task_sleep(snapshot),
        crate::kernel::task::table::TaskReturnKind::Exit => handle_task_exit(snapshot),
        crate::kernel::task::table::TaskReturnKind::Fault => handle_task_fault(snapshot),
        crate::kernel::task::table::TaskReturnKind::None => handle_task_return_none(snapshot),
    }
}

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

fn handle_task_yield(snapshot: TaskReturnSnapshot) -> TaskReturnHandleResult {
    scheduler_log_line("  return action: yield -> scheduler::run");

    if !snapshot.can_resume {
        scheduler_log_line("  yield result: failed; returned task is not resumable");
        return TaskReturnHandleResult::Failed;
    }

    set_round_robin_cursor(snapshot.task_id);

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

fn handle_task_exit(snapshot: TaskReturnSnapshot) -> TaskReturnHandleResult {
    scheduler_log_line("  return action: exit -> no resume for returned task");

    if snapshot.can_resume {
        scheduler_log_line("  exit result: failed; finished task is still resumable");
        return TaskReturnHandleResult::Failed;
    }

    set_round_robin_cursor(snapshot.task_id);

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

fn handle_task_sleep(snapshot: TaskReturnSnapshot) -> TaskReturnHandleResult {
    scheduler_log_line("  return action: sleep -> no resume until wake tick");

    if snapshot.can_resume {
        scheduler_log_line("  sleep result: failed; sleeping task is still resumable");
        return TaskReturnHandleResult::Failed;
    }

    set_round_robin_cursor(snapshot.task_id);

    scheduler_log_str("  round-robin cursor set to sleeping task: ");
    task::print_task_name_by_id(snapshot.task_id);
    scheduler_log_line("");

    scheduler_log_line("  sleep action: try next dispatchable task");

    match find_next_dispatchable_task_after(Some(snapshot.task_id)) {
        Some(task_id) => {
            scheduler_log_str("  next dispatchable task after sleep: ");
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
            scheduler_log_line("  next dispatchable task after sleep: none");
            scheduler_log_line("  result: no runnable task");
            TaskReturnHandleResult::NoRunnableTask
        }
    }
}

fn handle_task_return_none(_snapshot: TaskReturnSnapshot) -> TaskReturnHandleResult {
    scheduler_log_line("  return action: none -> failed");
    TaskReturnHandleResult::Failed
}

fn start_selected_task_checked(task_id: usize) -> ! {
    scheduler_log_line("  scheduler start path: checked start");

    scheduler_log_str("  start task: ");
    task::print_task_name_by_id(task_id);
    scheduler_log_line("");

    if !task::is_fresh_ready_task(task_id)
        || !task::is_ready_running_faulted_finished_invariant_ok(task_id)
    {
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

    crate::kernel::cpu::set_current(task_id);
    crate::kernel::cpu::set_current_stack_bounds(stack_start, stack_top);

    let Some(image) = build_fresh_trap_image(task_id) else {
        scheduler_start_failed();
    };
    arm_worker_for_mret(task_id, true);
    crate::arch::mret_to_trap_image(&image);
}

fn scheduler_start_failed() -> ! {
    scheduler_log_line("  scheduler start path result: FAILED");
    crate::arch::halt();
}

fn handle_task_fault(snapshot: TaskReturnSnapshot) -> TaskReturnHandleResult {
    scheduler_log_line("  return action: fault -> no resume for faulted task");

    if snapshot.can_resume
        || !task::is_task_faulted(snapshot.task_id)
        || !task::is_ready_running_faulted_finished_invariant_ok(snapshot.task_id)
    {
        scheduler_log_line("  fault result: failed; faulted task is still resumable");
        return TaskReturnHandleResult::Failed;
    }

    set_round_robin_cursor(snapshot.task_id);

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
