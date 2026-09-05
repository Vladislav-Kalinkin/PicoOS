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

#[cfg(feature = "scenario_reap")]
fn idle_task() {
    uart::write_line("idle_task: running");
}

#[cfg(feature = "scenario_reap")]
fn worker_a_task() {
    uart::write_line("worker_a_task: running");
}

#[cfg(feature = "scenario_reap")]
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
        let _ = create_task("worker-resume", worker_two_yield);
        uart::write_line("resume image: U-mode two-yield worker");
    }

    #[cfg(feature = "scenario_handoff")]
    {
        let _ = create_task("worker-a", worker_handoff_a);
        let _ = create_task("worker-b", worker_handoff_b);
        uart::write_line("handoff image: two U-mode yield workers");
    }

    #[cfg(feature = "scenario_fault")]
    {
        let _ = create_task("worker-a", worker_clean_exit);
        let _ = create_task("trap-worker", worker_ebreak);
        uart::write_line("fault image: exit worker + U-mode ebreak");
    }

    #[cfg(feature = "scenario_sleep")]
    {
        let _ = create_task("worker-sleep", worker_sleep_e2e);
        uart::write_line("sleep image: U-mode sleep then exit");
    }

    #[cfg(feature = "scenario_preempt")]
    {
        let _ = create_task("worker-yield", worker_yield_main);
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
        let _ = create_task("worker-yield", worker_yield_main);
        let _ = create_task("worker-sleep", worker_sleep_main);
        let _ = create_task("worker-pmp-deny", worker_pmp_deny);
        uart::write_line("default image: idle + worker_yield + worker_sleep");
    }

    print_tasks();
}

#[cfg(not(any(
    feature = "scenario_reap",
    feature = "scenario_kernel_fault",
    feature = "scenario_resume",
    feature = "scenario_handoff",
    feature = "scenario_fault",
    feature = "scenario_sleep"
)))]
fn worker_yield_main() {
    crate::kernel::sys::u_sys_log(b"worker_yield: start\n");
    loop {
        crate::kernel::sys::u_sys_yield();
    }
}

#[cfg(not(any(
    feature = "scenario_reap",
    feature = "scenario_kernel_fault",
    feature = "scenario_resume",
    feature = "scenario_handoff",
    feature = "scenario_fault",
    feature = "scenario_sleep",
    feature = "scenario_preempt"
)))]
fn worker_sleep_main() {
    crate::kernel::sys::u_sys_log(b"worker_sleep: start\n");
    loop {
        crate::kernel::sys::u_sys_sleep(1);
    }
}

#[cfg(not(any(
    feature = "scenario_reap",
    feature = "scenario_kernel_fault",
    feature = "scenario_resume",
    feature = "scenario_handoff",
    feature = "scenario_fault",
    feature = "scenario_sleep",
    feature = "scenario_preempt"
)))]
fn worker_pmp_deny() {
    crate::kernel::sys::u_sys_log(b"pmp deny probe: store to .data\n");
    // SAFETY: U-mode PMP probe; a store to `.data` must trap.
    unsafe {
        core::ptr::write_volatile(crate::kernel::memory::data_start() as *mut u64, 0xDEAD);
    }
    crate::kernel::sys::u_sys_log(b"pmp deny probe: FAILED\n");
    crate::kernel::sys::u_sys_exit();
}

#[cfg(feature = "scenario_resume")]
fn worker_two_yield() {
    crate::kernel::sys::u_sys_log(b"two_yielding_task: step 1\n");
    crate::kernel::sys::u_sys_yield();
    crate::kernel::sys::u_sys_log(b"two_yielding_task: step 2\n");
    crate::kernel::sys::u_sys_yield();
    crate::kernel::sys::u_sys_log(b"two_yielding_task: step 3\n");
    crate::kernel::sys::u_sys_exit();
}

#[cfg(feature = "scenario_handoff")]
fn worker_handoff_a() {
    crate::kernel::sys::u_sys_log(b"handoff_worker_a: step 1\n");
    crate::kernel::sys::u_sys_yield();
    crate::kernel::sys::u_sys_log(b"handoff_worker_a: resumed after first yield\n");
    crate::kernel::sys::u_sys_yield();
    crate::kernel::sys::u_sys_log(b"handoff_worker_a: resumed after second yield\n");
    crate::kernel::sys::u_sys_exit();
}

#[cfg(feature = "scenario_handoff")]
fn worker_handoff_b() {
    crate::kernel::sys::u_sys_log(b"handoff_worker_b: step 1\n");
    crate::kernel::sys::u_sys_yield();
    crate::kernel::sys::u_sys_log(b"handoff_worker_b: resumed after yield\n");
    crate::kernel::sys::u_sys_exit();
}

#[cfg(feature = "scenario_fault")]
fn worker_clean_exit() {
    crate::kernel::sys::u_sys_log(b"worker-a: exit\n");
    crate::kernel::sys::u_sys_exit();
}

#[cfg(feature = "scenario_fault")]
fn worker_ebreak() {
    crate::kernel::sys::u_sys_log(b"trap-worker: ebreak\n");
    // SAFETY: `ebreak` is the intended U-mode fault for this worker.
    unsafe {
        core::arch::asm!("ebreak", options(nomem, nostack, preserves_flags));
    }
    crate::kernel::sys::u_sys_log(b"trap-worker: FAILED\n");
    crate::kernel::sys::u_sys_exit();
}

#[cfg(feature = "scenario_sleep")]
fn worker_sleep_e2e() {
    crate::kernel::sys::u_sys_log(b"sleeping_task_runtime_e2e: step 1\n");
    crate::kernel::sys::u_sys_sleep(2);
    crate::kernel::sys::u_sys_log(b"sleeping_task_runtime_e2e: resumed after timer wake\n");
    crate::kernel::sys::u_sys_exit();
}
