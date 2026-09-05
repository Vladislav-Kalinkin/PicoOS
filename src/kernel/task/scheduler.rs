use crate::drivers::uart;
use crate::kernel::irq_cell::IrqCell;
use crate::kernel::task::table as task;
use crate::kernel::trap_frame::TrapImage;

static CURRENT_TASK_ID: IrqCell<Option<usize>> = IrqCell::new(None);
static DEFAULT_SEEN_YIELD: IrqCell<bool> = IrqCell::new(false);
static DEFAULT_SEEN_SLEEP: IrqCell<bool> = IrqCell::new(false);
static DEFAULT_MARKER_PRINTED: IrqCell<bool> = IrqCell::new(false);
static U_YIELDS: IrqCell<u32> = IrqCell::new(0);
static U_EXITS: IrqCell<u32> = IrqCell::new(0);
static SEEN_FAULT: IrqCell<bool> = IrqCell::new(false);
static SCENARIO_MARKER_PRINTED: IrqCell<bool> = IrqCell::new(false);

pub fn current_task_id() -> Option<usize> {
    CURRENT_TASK_ID.with(|id| *id)
}

pub fn print_task_name(id: usize) {
    task::print_task_name_by_id(id);
}

fn force_current_task(id: usize) {
    if !task::mark_task_running(id) {
        return;
    }

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

fn build_fresh_trap_image(task_id: usize) -> Option<TrapImage> {
    let entry = task::get_task_entry(task_id)?;
    let stack_top = task::get_task_stack_top(task_id)?;
    let mut image = TrapImage::empty();
    image.gpr.sp = stack_top;
    image.gpr.a0 = entry as *const () as usize as u64;
    image.mepc = crate::kernel::task::entry::task_trampoline_raw as *const () as usize as u64;
    Some(image)
}

/// Next dispatchable worker after `after` (`Cpu.current`, not idle WFI).
pub fn next_after(after: Option<usize>) -> Option<usize> {
    task::find_next_dispatchable_after(after)
}

fn arm_worker_for_mret(task_id: usize, fresh: bool) {
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
    }
}

fn image_for_dispatch(task_id: usize) -> Option<(TrapImage, bool)> {
    let fresh = task::is_fresh_ready_task(task_id);
    let image = if fresh {
        build_fresh_trap_image(task_id)?
    } else {
        task::get_task_trap_image(task_id)?
    };
    Some((image, fresh))
}

fn mret_to_task(task_id: usize) -> ! {
    let Some((image, fresh)) = image_for_dispatch(task_id) else {
        crate::arch::halt();
    };
    arm_worker_for_mret(task_id, fresh);
    crate::arch::mret_to_trap_image(&image);
}

/// Trap-context switch: `mret` to `next`, or idle-exit if none.
pub fn switch_to(next: Option<usize>) -> ! {
    match next {
        Some(id) => mret_to_task(id),
        None => crate::arch::idle_exit_from_trap(),
    }
}

/// Pick the next worker after `after`, reap `after` if it is terminal, then
/// `mret` or idle-exit. Call only from a trap (ecall / fault / timer).
pub fn switch_after(after: Option<usize>) -> ! {
    let next = next_after(after);
    if let Some(id) = after
        && task::is_terminal_task(id)
    {
        let _ = task::destroy(id);
    }
    switch_to(next);
}

/// Boot entry: first dispatch from kernel context (not a trap frame).
pub fn run() -> ! {
    match next_after(current_task_id()) {
        None => idle_loop(),
        Some(id) => mret_to_task(id),
    }
}

pub fn note_default_image_return(kind: crate::kernel::task::table::TaskReturnKind) {
    match kind {
        crate::kernel::task::table::TaskReturnKind::Yield => {
            DEFAULT_SEEN_YIELD.with(|seen| *seen = true);
            U_YIELDS.with(|count| *count = count.saturating_add(1));
        }
        crate::kernel::task::table::TaskReturnKind::Sleep => {
            DEFAULT_SEEN_SLEEP.with(|seen| *seen = true);
        }
        crate::kernel::task::table::TaskReturnKind::Exit => {
            U_EXITS.with(|count| *count = count.saturating_add(1));
            try_print_scenario_markers();
        }
        crate::kernel::task::table::TaskReturnKind::Fault => {
            SEEN_FAULT.with(|seen| *seen = true);
            try_print_scenario_markers();
        }
        crate::kernel::task::table::TaskReturnKind::None => {}
    }

    let yield_seen = DEFAULT_SEEN_YIELD.with(|seen| *seen);
    let sleep_seen = DEFAULT_SEEN_SLEEP.with(|seen| *seen);
    let already = DEFAULT_MARKER_PRINTED.with(|printed| *printed);
    if yield_seen && sleep_seen && !already {
        DEFAULT_MARKER_PRINTED.with(|printed| *printed = true);
        uart::write_line("default scheduler: yield and sleep OK");
    }
}

fn try_print_scenario_markers() {
    if SCENARIO_MARKER_PRINTED.with(|printed| *printed) {
        return;
    }

    let yields = U_YIELDS.with(|count| *count);
    let exits = U_EXITS.with(|count| *count);
    let faulted = SEEN_FAULT.with(|seen| *seen);
    let slept = DEFAULT_SEEN_SLEEP.with(|seen| *seen);

    let marker = if (cfg!(feature = "scenario_resume") && yields >= 2 && exits >= 1)
        || (cfg!(feature = "scenario_handoff") && exits >= 2)
    {
        Some("scheduler resume loop result: OK")
    } else if cfg!(feature = "scenario_sleep") && slept && exits >= 1 {
        Some("task sleep runtime e2e result: OK")
    } else if cfg!(feature = "scenario_fault")
        && faulted
        && exits >= 1
        && !task::has_dispatchable_tasks()
    {
        Some("task fault scheduler result: OK")
    } else {
        None
    };

    if let Some(line) = marker {
        SCENARIO_MARKER_PRINTED.with(|printed| *printed = true);
        uart::write_line(line);
    }
}
