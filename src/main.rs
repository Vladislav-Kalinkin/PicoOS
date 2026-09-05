#![no_std]
#![no_main]
// Truncated boots: reap never calls `arch::init` / `scheduler::run`,
// kernel-fault `ebreak`s before the scheduler. The always-on kernel stays
// compiled; unused here is the short path, not leftover dual-mode code.
#![cfg_attr(
    any(feature = "scenario_reap", feature = "scenario_kernel_fault"),
    allow(dead_code, unused_imports)
)]
#![cfg_attr(feature = "scenario_kernel_fault", allow(unreachable_code))]

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

    #[cfg(feature = "scenario_reap")]
    {
        kernel::test::run_selftests();
    }

    #[cfg(not(feature = "scenario_reap"))]
    {
        arch::init_exceptions();
        arch::pmp::init();
        arch::print_cpu_info();

        #[cfg(feature = "scenario_kernel_fault")]
        kernel::test::run_kernel_fault_guard();

        #[cfg(not(feature = "scenario_kernel_fault"))]
        {
            kernel::test::run_memory_tests();
            kernel::task::test::spawn_default_image();
            kernel::task::scheduler::switch_to_idle();
            arm_timer();
            kernel::task::scheduler::run();
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
