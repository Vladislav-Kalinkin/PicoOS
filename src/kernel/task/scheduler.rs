use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use crate::drivers::uart;
use crate::kernel::hart_local::HartLocal;
use crate::kernel::task::table::{self as task, TaskId};
use crate::kernel::trap_frame::TrapImage;

static DEFAULT_SEEN_YIELD: AtomicBool = AtomicBool::new(false);
static DEFAULT_SEEN_SLEEP: AtomicBool = AtomicBool::new(false);
static DEFAULT_MARKER_PRINTED: AtomicBool = AtomicBool::new(false);
static U_YIELDS: AtomicU32 = AtomicU32::new(0);
static U_EXITS: AtomicU32 = AtomicU32::new(0);
static SEEN_FAULT: AtomicBool = AtomicBool::new(false);
static SCENARIO_MARKER_PRINTED: AtomicBool = AtomicBool::new(false);
static JOIN_LEAK_BASELINE: HartLocal<Option<u64>> = HartLocal::new(None);
static JOIN_LEAK_PRINTED: AtomicBool = AtomicBool::new(false);

pub fn print_task_name(id: TaskId) {
    task::print_task_name_by_id(id);
}

pub fn switch_to_idle() {
    crate::kernel::cpu::clear_current();
}

pub fn idle_loop() -> ! {
    switch_to_idle();
    loop {
        crate::arch::wait_for_interrupt();
    }
}

fn build_fresh_trap_image(task_id: TaskId) -> Option<TrapImage> {
    let entry_pc = task::get_task_initial_pc(task_id)?;
    if entry_pc == 0 {
        return None;
    }
    let stack_top = task::get_task_stack_top(task_id)?;
    let spawn_arg = task::get_task_spawn_arg(task_id).unwrap_or(0);
    let mut image = TrapImage::empty();
    image.gpr.sp = stack_top;
    image.gpr.a0 = entry_pc;
    image.gpr.a1 = spawn_arg;
    image.mepc = crate::arch::riscv64::user_trampoline_addr();
    Some(image)
}

pub fn next_after(after: Option<TaskId>) -> Option<TaskId> {
    task::find_next_dispatchable_after(after)
}

fn arm_worker_for_mret(task_id: TaskId, fresh: bool) {
    let _ = task::mark_task_running(task_id);

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

fn image_for_dispatch(task_id: TaskId) -> Option<(TrapImage, bool)> {
    let fresh = task::is_fresh_ready_task(task_id);
    let image = if fresh {
        build_fresh_trap_image(task_id)?
    } else {
        task::get_task_trap_image(task_id)?
    };
    Some((image, fresh))
}

fn mret_to_task(task_id: TaskId) -> ! {
    let Some((image, fresh)) = image_for_dispatch(task_id) else {
        crate::kernel::log::fail("sched", "dispatch image missing");
        crate::arch::halt();
    };
    arm_worker_for_mret(task_id, fresh);
    crate::arch::mret_to_trap_image(&image);
}

pub fn switch_to(next: Option<TaskId>) -> ! {
    match next {
        Some(id) => mret_to_task(id),
        None => crate::arch::idle_exit_from_trap(),
    }
}

pub fn switch_after(after: Option<TaskId>) -> ! {
    let next = next_after(after);
    switch_to(next);
}

pub fn run() -> ! {
    match next_after(crate::kernel::cpu::current()) {
        None => idle_loop(),
        Some(id) => mret_to_task(id),
    }
}

pub fn note_default_image_return(kind: task::TaskReturnKind) {
    match kind {
        task::TaskReturnKind::Yield => {
            DEFAULT_SEEN_YIELD.store(true, Ordering::Release);
            let _ = U_YIELDS.fetch_add(1, Ordering::AcqRel);
        }
        task::TaskReturnKind::Sleep => {
            DEFAULT_SEEN_SLEEP.store(true, Ordering::Release);
        }
        task::TaskReturnKind::Exit => {
            let _ = U_EXITS.fetch_add(1, Ordering::AcqRel);
            try_print_scenario_markers();
        }
        task::TaskReturnKind::Fault => {
            SEEN_FAULT.store(true, Ordering::Release);
            try_print_scenario_markers();
        }
        task::TaskReturnKind::None
        | task::TaskReturnKind::Join
        | task::TaskReturnKind::Send
        | task::TaskReturnKind::Recv => {}
    }

    let yield_seen = DEFAULT_SEEN_YIELD.load(Ordering::Acquire);
    let sleep_seen = DEFAULT_SEEN_SLEEP.load(Ordering::Acquire);
    if yield_seen && sleep_seen && !DEFAULT_MARKER_PRINTED.swap(true, Ordering::AcqRel) {
        uart::write_line("default scheduler: yield and sleep OK");
    }
}

fn try_print_scenario_markers() {
    if SCENARIO_MARKER_PRINTED.load(Ordering::Acquire) {
        return;
    }

    let yields = U_YIELDS.load(Ordering::Acquire);
    let exits = U_EXITS.load(Ordering::Acquire);
    let faulted = SEEN_FAULT.load(Ordering::Acquire);
    let slept = DEFAULT_SEEN_SLEEP.load(Ordering::Acquire);

    let plan = crate::kernel::contract::plan();
    let marker =
        if (plan == crate::kernel::contract::BootContract::Resume && yields >= 2 && exits >= 1)
            || (plan == crate::kernel::contract::BootContract::Handoff && exits >= 2)
        {
            Some("scheduler resume loop result: OK")
        } else if plan == crate::kernel::contract::BootContract::Sleep && slept && exits >= 1 {
            Some("task sleep runtime e2e result: OK")
        } else if plan == crate::kernel::contract::BootContract::Fault
            && faulted
            && exits >= 1
            && !task::has_dispatchable_tasks()
        {
            Some("task fault scheduler result: OK")
        } else {
            None
        };

    if let Some(line) = marker
        && !SCENARIO_MARKER_PRINTED.swap(true, Ordering::AcqRel)
    {
        uart::write_line(line);
    }
}

pub fn capture_default_join_baseline() {
    JOIN_LEAK_BASELINE.with(|baseline| {
        *baseline = Some(crate::kernel::memory::stats().used);
    });
}

pub fn note_join_reap() {
    let Some(baseline) = JOIN_LEAK_BASELINE.with(|baseline| *baseline) else {
        return;
    };
    if crate::kernel::memory::stats().used != baseline {
        return;
    }
    let already = JOIN_LEAK_PRINTED.swap(true, Ordering::AcqRel);
    if already {
        return;
    }
    uart::write_line("default spawn join: OK");
    uart::write_line("spawn join leak: OK");
}
