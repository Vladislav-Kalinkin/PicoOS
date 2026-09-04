#![no_std]
#![no_main]
// Test/scenario features compile helpers that the other `kernel_main` path
// does not call. Default (no features) still denies unused code.
#![cfg_attr(
    any(
        feature = "selftest",
        feature = "task_yield_test",
        feature = "resume_candidate_test",
        feature = "resume_preflight_test",
        feature = "resume_dry_run_test",
        feature = "resume_restore_test",
        feature = "real_resume_restore_test",
        feature = "real_resume_restore_jump",
        feature = "two_yield_task_test",
        feature = "scheduler_resume_loop_test",
        feature = "verbose_resume_debug",
        feature = "scheduler_dispatch_test",
        feature = "scheduler_run_test",
        feature = "scheduler_reentry_test",
        feature = "two_task_resume_handoff_test",
        feature = "task_fault_test",
        feature = "kernel_fault_guard_test",
        feature = "scheduler_verbose_dispatch_trace",
        feature = "task_sleep_test",
        feature = "task_sleep_runtime_e2e_test",
        feature = "kernel_log_scoped",
        feature = "log_trap",
        feature = "log_timer",
        feature = "log_fault",
        feature = "log_sleep",
        feature = "timer_preemption_prototype",
        feature = "scheduler_fault_lifecycle_test"
    ),
    allow(dead_code, unused_imports, unreachable_code)
)]

use core::panic::PanicInfo;

mod arch;
mod drivers;
mod kernel;
mod platform;

use crate::drivers::uart;

#[unsafe(no_mangle)]
pub extern "C" fn kernel_main() -> ! {
    uart::write_line("");
    kernel::banner::print_boot_banner();

    uart::write_line("arch: riscv64");
    uart::write_line("target arch: riscv64");
    uart::write_str("platform: ");
    uart::write_line(crate::platform::NAME);
    uart::write_line("status: kernel started");
    kernel::banner::print_capabilities();

    #[cfg(feature = "selftest")]
    {
        kernel::test::run_selftests();
    }

    #[cfg(not(feature = "selftest"))]
    {
        arch::init_exceptions();
        arch::pmp::init();
        arch::print_cpu_info();

        #[cfg(any(feature = "task_yield_test", feature = "kernel_fault_guard_test"))]
        {
            kernel::test::run_runtime_selftest_bootstrap();
            kernel::task::scheduler::init();
            kernel::test::run_runtime_selftest_after_scheduler_init();
            start_timer_and_wait();
        }

        #[cfg(not(any(feature = "task_yield_test", feature = "kernel_fault_guard_test")))]
        {
            kernel::test::run_memory_tests();
            kernel::task::test::spawn_default_image();
            kernel::task::scheduler::switch_to_idle();
            arm_timer();
            match kernel::task::scheduler::run() {
                kernel::task::scheduler::RunResult::NoRunnableTask => {
                    kernel::task::scheduler::idle_loop();
                }
                kernel::task::scheduler::RunResult::Failed => {
                    uart::write_line("default scheduler: FAILED");
                    arch::halt();
                }
            }
        }
    }
}

fn arm_timer() {
    use crate::arch::riscv64::{cpu, timer};

    const RISCV_TIMER_HZ: u64 = 1;

    uart::write_line("");
    uart::write_line("RISC-V timer:");

    uart::write_str("timebase frequency: ");
    uart::write_dec_u64(timer::timebase_frequency());
    uart::write_line(" Hz");

    uart::write_str("mtime before: ");
    uart::write_hex_u64(timer::mtime());
    uart::write_line("");

    uart::write_str("starting periodic timer: ");
    uart::write_dec_u64(RISCV_TIMER_HZ);
    uart::write_line(" Hz");

    kernel::ticks::reset();
    timer::arm_timer_hz(RISCV_TIMER_HZ);

    uart::write_line("enabling machine timer interrupt...");
    cpu::enable_machine_timer_interrupt();

    uart::write_line("enabling machine interrupts...");
    arch::enable_irq();

    uart::write_str("mstatus after enable: ");
    uart::write_hex_u64(cpu::mstatus());
    uart::write_line("");

    uart::write_str("mie after enable: ");
    uart::write_hex_u64(cpu::mie());
    uart::write_line("");
}

#[cfg(any(feature = "task_yield_test", feature = "kernel_fault_guard_test"))]
fn start_timer_and_wait() -> ! {
    arm_timer();
    uart::write_line("waiting for RISC-V ticks...");
    loop {
        arch::wait_for_interrupt();
    }
}

static PANICKING: crate::kernel::irq_cell::IrqCell<bool> =
    crate::kernel::irq_cell::IrqCell::new(false);

struct UartFmtWrite;

impl core::fmt::Write for UartFmtWrite {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        uart::write_str(s);
        Ok(())
    }
}

#[panic_handler]
fn panic(info: &PanicInfo<'_>) -> ! {
    let already = PANICKING.with(|flag| {
        let already = *flag;
        *flag = true;
        already
    });
    if already {
        arch::halt();
    }

    arch::disable_irq();

    uart::write_line("");
    uart::write_line("KERNEL PANIC");

    if let Some(location) = info.location() {
        uart::write_str("file: ");
        uart::write_str(location.file());
        uart::write_str(":");
        uart::write_dec_u64(u64::from(location.line()));
        uart::write_str(":");
        uart::write_dec_u64(u64::from(location.column()));
        uart::write_line("");
    }

    uart::write_str("message: ");
    let _ = core::fmt::Write::write_fmt(&mut UartFmtWrite, format_args!("{}", info.message()));
    uart::write_line("");

    arch::halt();
}
