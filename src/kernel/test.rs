use crate::drivers::uart;
use crate::kernel;

#[derive(Clone, Copy)]
struct RuntimeSelftestScenario {
    name: &'static str,
    run_bootstrap: fn(),
    run_after_scheduler_init: fn(),
}

#[allow(clippy::needless_return)]
fn runtime_selftest_scenario() -> RuntimeSelftestScenario {
    #[cfg(feature = "kernel_fault_guard_test")]
    {
        return RuntimeSelftestScenario {
            name: "kernel_fault_guard",
            run_bootstrap: runtime_bootstrap_kernel_fault_guard,
            run_after_scheduler_init: runtime_after_scheduler_noop,
        };
    }

    #[cfg(all(
        not(feature = "kernel_fault_guard_test"),
        feature = "task_yield_test",
        feature = "timer_preemption_selftest"
    ))]
    {
        return RuntimeSelftestScenario {
            name: "timer_preemption",
            run_bootstrap: runtime_bootstrap_task_yield,
            run_after_scheduler_init: runtime_after_scheduler_noop,
        };
    }

    #[cfg(all(
        not(feature = "kernel_fault_guard_test"),
        feature = "task_yield_test",
        not(feature = "timer_preemption_selftest")
    ))]
    {
        return RuntimeSelftestScenario {
            name: "task_yield",
            run_bootstrap: runtime_bootstrap_task_yield,
            run_after_scheduler_init: runtime_after_scheduler_task_yield,
        };
    }

    #[cfg(all(
        not(feature = "kernel_fault_guard_test"),
        not(feature = "task_yield_test")
    ))]
    RuntimeSelftestScenario {
        name: "basic_tasks",
        run_bootstrap: runtime_bootstrap_basic_tasks,
        run_after_scheduler_init: runtime_after_scheduler_noop,
    }
}

pub fn run_memory_tests() {
    kernel::memory::print_memory_layout();
    kernel::memory::test_page_allocator();
    kernel::heap::test_heap();
}

pub fn print_test_complete() {
    uart::write_line("timer test complete");
    uart::write_line("system halted");
}

pub fn run_runtime_selftest_bootstrap() {
    let scenario = runtime_selftest_scenario();
    uart::write_str("runtime selftest scenario: ");
    uart::write_line(scenario.name);
    (scenario.run_bootstrap)();
}

pub fn run_runtime_selftest_after_scheduler_init() {
    let scenario = runtime_selftest_scenario();
    (scenario.run_after_scheduler_init)();
}

#[cfg(all(
    not(feature = "kernel_fault_guard_test"),
    not(feature = "task_yield_test")
))]
fn runtime_bootstrap_basic_tasks() {
    run_memory_tests();
    crate::kernel::task::test_tasks();
}

#[cfg(feature = "task_yield_test")]
fn runtime_bootstrap_task_yield() {
    run_memory_tests();
    crate::kernel::task::test_tasks_with_yield_worker();
}

#[cfg(feature = "kernel_fault_guard_test")]
fn runtime_bootstrap_kernel_fault_guard() {
    crate::kernel::task::test_kernel_fault_guard();
}

#[cfg(feature = "task_yield_test")]
fn runtime_after_scheduler_task_yield() {
    crate::kernel::task::test_task_yield();
}

const fn runtime_after_scheduler_noop() {}

#[cfg(feature = "selftest")]
pub fn run_selftests() -> ! {
    uart::write_line("");
    uart::write_line("selftest mode:");

    uart::write_line("");
    uart::write_line("[selftest] memory");
    run_memory_tests();

    uart::write_line("");
    uart::write_line("");
    uart::write_line("[selftest] task table");
    #[cfg(feature = "task_yield_test")]
    crate::kernel::task::test_tasks_with_yield_worker();

    #[cfg(not(feature = "task_yield_test"))]
    crate::kernel::task::test_tasks();

    uart::write_line("");
    uart::write_line("selftest complete");
    crate::arch::halt();
}
