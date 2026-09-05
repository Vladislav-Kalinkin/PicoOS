use crate::drivers::uart;
use crate::kernel;

pub fn run_memory_tests() {
    kernel::memory::print_memory_layout();
    kernel::memory::test_page_allocator();
    test_page_alloc_free_four();
    test_reap_leak_check();
    kernel::memory::print_mm_stats();
}

fn test_page_alloc_free_four() {
    uart::write_line("");
    uart::write_line("page alloc/free 4:");

    let used_before = kernel::memory::stats().used;
    let Some(base) = kernel::memory::alloc_pages(4) else {
        uart::write_line("page alloc 4: FAILED");
        return;
    };

    uart::write_str("page alloc 4: ");
    uart::write_hex_u64(base.addr());
    uart::write_line("");

    if kernel::memory::stats().used != used_before + 4 {
        uart::write_line("page alloc 4: FAILED used");
        kernel::memory::free_pages(base, 4);
        return;
    }

    kernel::memory::free_pages(base, 4);

    if kernel::memory::stats().used == used_before {
        uart::write_line("page alloc/free 4: OK");
    } else {
        uart::write_line("page alloc/free 4: FAILED used after free");
    }
}

extern "C" fn reap_probe_task(_arg: u64) {
    crate::arch::halt();
}

fn test_reap_leak_check() {
    use crate::kernel::task::table::{destroy, mark_task_finished, spawn};

    uart::write_line("");
    uart::write_line("mm reap:");

    crate::kernel::task::table::init();

    let used_before = kernel::memory::stats().used;

    let Some(id) = spawn("reap-a", reap_probe_task, 0) else {
        uart::write_line("mm leak check: FAILED create");
        return;
    };

    if !mark_task_finished(id) {
        uart::write_line("mm leak check: FAILED finish");
        return;
    }

    if !destroy(id) {
        uart::write_line("mm leak check: FAILED destroy");
        return;
    }

    if kernel::memory::stats().used != used_before {
        uart::write_line("mm leak check: FAILED used after reap");
        return;
    }

    let Some(id2) = spawn("reap-b", reap_probe_task, 0) else {
        uart::write_line("mm leak check: FAILED recreate");
        return;
    };

    let reused = id2 == id;
    uart::write_str("reaped id reused: ");
    kernel::task::table::print_yes_no(reused);
    uart::write_line("");

    if !mark_task_finished(id2) || !destroy(id2) {
        uart::write_line("mm leak check: FAILED second reap");
        return;
    }

    if reused && kernel::memory::stats().used == used_before {
        uart::write_line("mm leak check: OK");
    } else {
        uart::write_line("mm leak check: FAILED");
    }
}

pub fn run_selftests() -> ! {
    uart::write_line("");
    uart::write_line("selftest mode:");

    uart::write_line("");
    uart::write_line("[selftest] memory");
    run_memory_tests();

    uart::write_line("");
    uart::write_line("");
    uart::write_line("[selftest] task table");
    crate::kernel::task::test::test_tasks();

    uart::write_line("");
    uart::write_line("selftest complete");
    crate::arch::halt();
}

pub fn run_kernel_fault_guard() -> ! {
    uart::write_line("");
    uart::write_line("kernel fault guard test:");
    uart::write_line("triggering real trap from kernel context");
    crate::kernel::cpu::clear_current();

    // SAFETY: `ebreak` is the intended M-mode kernel-fault trigger.
    unsafe {
        core::arch::asm!("ebreak", options(nomem, nostack, preserves_flags));
    }

    uart::write_line("kernel fault guard result: FAILED");
    uart::write_line("kernel continued after kernel fault trap");
    crate::arch::halt();
}
