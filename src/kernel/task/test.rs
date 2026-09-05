use crate::drivers::uart;
mod bootstrap;
use crate::kernel::task::table::{print_tasks, spawn};

#[cfg(feature = "scenario_reap")]
pub fn test_tasks() {
    crate::kernel::task::table::init();

    let _ = spawn("idle", idle_task, 0);
    bootstrap::print_task_zero_context_guard();
    let _ = spawn("worker-a", worker_a_task, 0);
    let _ = spawn("worker-b", worker_b_task, 0);

    print_tasks();
}

extern "C" fn idle_task(_arg: u64) {
    uart::write_line("idle_task: running");
}

extern "C" fn worker_a_task(_arg: u64) {
    uart::write_line("worker_a_task: running");
}

extern "C" fn worker_b_task(_arg: u64) {
    uart::write_line("worker_b_task: running");
}

#[cfg(not(any(feature = "scenario_reap", feature = "scenario_kernel_fault")))]
pub fn spawn_default_image() {
    crate::kernel::task::table::init();

    #[cfg(feature = "scenario_sleep")]
    bootstrap::test_task_sleep_wakeup_table_selftest();

    #[cfg(feature = "scenario_resume")]
    {
        let _ = spawn("worker-resume", crate::user::worker_two_yield, 0);
        uart::write_line("resume image: U-mode two-yield worker");
    }

    #[cfg(feature = "scenario_handoff")]
    {
        let _ = spawn("worker-a", crate::user::worker_handoff_a, 0);
        let _ = spawn("worker-b", crate::user::worker_handoff_b, 0);
        uart::write_line("handoff image: two U-mode yield workers");
    }

    #[cfg(feature = "scenario_fault")]
    {
        let _ = spawn("worker-a", crate::user::worker_clean_exit, 0);
        let _ = spawn("trap-worker", crate::user::worker_ebreak, 0);
        let _ = spawn("fetch-probe", crate::user::worker_kernel_fetch, 0);
        uart::write_line("fault image: exit worker + U-mode ebreak");
    }

    #[cfg(feature = "scenario_sleep")]
    {
        let _ = spawn("worker-sleep", crate::user::worker_sleep_e2e, 0);
        uart::write_line("sleep image: U-mode sleep then exit");
    }

    #[cfg(feature = "scenario_preempt")]
    {
        let _ = spawn("worker-yield", crate::user::worker_yield_main, 0);
        uart::write_line("preempt image: U-mode yield loop");
    }

    #[cfg(not(any(
        feature = "scenario_resume",
        feature = "scenario_handoff",
        feature = "scenario_fault",
        feature = "scenario_sleep",
        feature = "scenario_preempt"
    )))]
    {
        let _ = spawn("worker-yield", crate::user::worker_yield_main, 0);
        let _ = spawn("worker-sleep", crate::user::worker_sleep_main, 0);
        let _ = spawn("worker-pmp-deny", crate::user::worker_pmp_deny, 0);
        let _ = spawn("worker-spawn", crate::user::worker_spawn_main, 0);
        uart::write_line("default image: idle + yield + sleep + pmp-deny + spawn");
    }

    crate::kernel::task::scheduler::capture_default_join_baseline();
    print_tasks();
}

const _: &[extern "C" fn(u64)] = &[idle_task, worker_a_task, worker_b_task];
