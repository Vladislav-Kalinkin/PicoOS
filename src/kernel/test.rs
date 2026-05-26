use crate::drivers::uart;
use crate::kernel;

pub fn run_memory_tests() {
    kernel::memory::print_memory_layout();
    kernel::memory::test_page_allocator();
    kernel::heap::test_heap();
}

pub fn print_test_complete() {
    uart::write_line("timer test complete");
    uart::write_line("system halted");
}

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
