#![no_std]
#![no_main]
#![cfg_attr(feature = "selftest", allow(dead_code))]
#![cfg_attr(
    feature = "kernel_fault_guard_test",
    allow(dead_code, unreachable_code)
)]

use core::arch::asm;
use core::panic::PanicInfo;

mod arch;
mod drivers;
mod kernel;
mod platform;

use crate::drivers::uart;

#[no_mangle]
pub extern "C" fn kernel_main() -> ! {
    uart::write_line("");
    kernel::banner::print_boot_banner();

    uart::write_line("arch: riscv64");
    uart::write_line("target arch: riscv64");
    uart::write_line("platform: QEMU virt riscv64");
    uart::write_line("status: kernel started");
    kernel::banner::print_capabilities();

    #[cfg(feature = "selftest")]
    {
        kernel::test::run_selftests();
    }

    #[cfg(not(feature = "selftest"))]
    {
        arch::init_exceptions();
        arch::print_cpu_info();
        kernel::test::run_runtime_selftest_bootstrap();

        kernel::task::scheduler::init();
        kernel::test::run_runtime_selftest_after_scheduler_init();

        {
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

            uart::write_line("waiting for RISC-V ticks...");
        }

        loop {
            arch::wait_for_interrupt();
        }
    }
}

#[allow(dead_code)]
fn trigger_test_exception() {
    unsafe {
        asm!("ebreak", options(nomem, nostack, preserves_flags));
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    uart::write_line("");
    uart::write_line("KERNEL PANIC");

    arch::halt();
}
