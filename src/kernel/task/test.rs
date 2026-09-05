use crate::drivers::uart;
mod bootstrap;
use crate::kernel::task::table::{create_task, print_tasks};

#[cfg(feature = "scenario_reap")]
pub fn test_tasks() {
    crate::kernel::task::table::init();

    let _ = create_task("idle", idle_task);
    bootstrap::print_task_zero_context_guard();
    let _ = create_task("worker-a", worker_a_task);
    let _ = create_task("worker-b", worker_b_task);

    print_tasks();
}

fn idle_task() {
    uart::write_line("idle_task: running");
}

fn worker_a_task() {
    uart::write_line("worker_a_task: running");
}

fn worker_b_task() {
    uart::write_line("worker_b_task: running");
}

#[cfg(not(any(feature = "scenario_reap", feature = "scenario_kernel_fault")))]
pub fn spawn_default_image() {
    crate::kernel::task::table::init();

    #[cfg(feature = "scenario_sleep")]
    bootstrap::test_task_sleep_wakeup_table_selftest();

    #[cfg(feature = "scenario_resume")]
    {
        let _ = create_task("worker-resume", crate::user::worker_two_yield);
        uart::write_line("resume image: U-mode two-yield worker");
    }

    #[cfg(feature = "scenario_handoff")]
    {
        let _ = create_task("worker-a", crate::user::worker_handoff_a);
        let _ = create_task("worker-b", crate::user::worker_handoff_b);
        uart::write_line("handoff image: two U-mode yield workers");
    }

    #[cfg(feature = "scenario_fault")]
    {
        let _ = create_task("worker-a", crate::user::worker_clean_exit);
        let _ = create_task("trap-worker", crate::user::worker_ebreak);
        let _ = create_task("fetch-probe", crate::user::worker_kernel_fetch);
        uart::write_line("fault image: exit worker + U-mode ebreak");
    }

    #[cfg(feature = "scenario_sleep")]
    {
        let _ = create_task("worker-sleep", crate::user::worker_sleep_e2e);
        uart::write_line("sleep image: U-mode sleep then exit");
    }

    #[cfg(feature = "scenario_preempt")]
    {
        let _ = create_task("worker-yield", crate::user::worker_yield_main);
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
        let _ = create_task("worker-yield", crate::user::worker_yield_main);
        let _ = create_task("worker-sleep", crate::user::worker_sleep_main);
        let _ = create_task("worker-pmp-deny", crate::user::worker_pmp_deny);
        uart::write_line("default image: idle + worker_yield + worker_sleep");
    }

    print_tasks();
}

const _: &[fn()] = &[idle_task, worker_a_task, worker_b_task];
