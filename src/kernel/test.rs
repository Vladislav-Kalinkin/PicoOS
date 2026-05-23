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
    crate::kernel::task::test_tasks();
    crate::kernel::task::scheduler::init();

    uart::write_line("");
    uart::write_line("[selftest] scheduler skip finished");
    crate::kernel::task::run_scheduler_skip_finished_check();

    uart::write_line("");
    uart::write_line("selftest complete");
    crate::arch::halt();
}
