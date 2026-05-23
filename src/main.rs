#![no_std]
#![no_main]
#![cfg_attr(feature = "selftest", allow(dead_code))]

use core::arch::asm;
use core::panic::PanicInfo;

mod arch;
mod drivers;
mod kernel;
mod platform;

#[cfg(target_arch = "aarch64")]
use crate::drivers::gic;

#[cfg(target_arch = "aarch64")]
use crate::kernel::timer;

use crate::drivers::uart;

#[cfg(target_arch = "aarch64")]
const TIMER_HZ: u64 = 1;

#[no_mangle]
pub extern "C" fn kernel_main() -> ! {
    // LLVM may emit FP/SIMD instructions for ordinary memory operations.
    // Enable FP/SIMD early on ARM64 to avoid traps in kernel code.
    #[cfg(target_arch = "aarch64")]
    {
        crate::arch::enable_fp_simd();
    }
    uart::write_line("");
    kernel::banner::print_boot_banner();

    uart::write_line("arch: multiarch");
    #[cfg(target_arch = "aarch64")]
    {
        uart::write_line("target arch: aarch64");
        uart::write_line("platform: QEMU virt aarch64");
    }
    #[cfg(target_arch = "riscv64")]
    {
        uart::write_line("target arch: riscv64");
        uart::write_line("platform: QEMU virt riscv64");
    }
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

        kernel::test::run_memory_tests();

        #[cfg(feature = "task_yield_test")]
        {
            kernel::task::test_tasks_with_yield_worker();
        }

        #[cfg(not(feature = "task_yield_test"))]
        {
            kernel::task::test_tasks();
        }

        kernel::task::scheduler::init();

        #[cfg(feature = "task_bootstrap_test")]
        {
            kernel::task::test_task_trampoline();
        }

        #[cfg(feature = "task_stack_switch_test")]
        {
            kernel::task::test_task_stack_switch();
        }

        #[cfg(feature = "sequential_task_test")]
        {
            kernel::task::test_sequential_task_runner();
        }

        #[cfg(feature = "task_yield_test")]
        {
            kernel::task::test_task_yield();
        }

        #[cfg(feature = "scheduler_skip_finished_test")]
        {
            kernel::task::test_scheduler_skips_finished_tasks();
        }

        #[cfg(feature = "scheduler_driven_task_test")]
        {
            kernel::task::test_scheduler_driven_task_runner();
        }

        #[cfg(feature = "resume_candidate_test")]
        {
            kernel::task::test_resume_candidate_selection();
        }
        #[cfg(target_arch = "riscv64")]
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

        #[cfg(target_arch = "aarch64")]
        {
            print_timer_info_short();

            gic::init();

            uart::write_line("");
            uart::write_str("starting periodic timer: ");
            uart::write_dec_u64(TIMER_HZ);
            uart::write_line(" Hz");

            kernel::ticks::reset();
            timer::arm_timer_hz(TIMER_HZ);

            uart::write_line("enabling CPU IRQ...");
            arch::enable_irq();

            uart::write_line("waiting for ticks...");
        }
        loop {
            arch::wait_for_interrupt();
        }
    }
}

#[cfg(target_arch = "aarch64")]
#[no_mangle]
pub extern "C" fn handle_irq(frame: *const kernel::trap_frame::Aarch64TrapFrame) {
    arch::disable_irq();

    let iar = gic::acknowledge_irq();
    let irq_id = iar & 0x3FF;

    if irq_id == gic::IRQ_PHYSICAL_TIMER {
        let saved_sp = frame as u64;
        let saved_pc = crate::arch::aarch64::cpu::elr_el1();

        let saved_task = kernel::task::scheduler::save_current_context(saved_sp, saved_pc);

        let tick = kernel::ticks::increment();

        uart::write_str("tick: ");
        uart::write_dec_u64(tick);

        uart::write_str(" saved current: ");
        match saved_task {
            Some(id) => {
                kernel::task::scheduler::print_task_name(id);
                kernel::task::print_task_context_values(saved_sp, saved_pc);
            }
            None => uart::write_str("none"),
        }

        uart::write_str(" next task: ");
        kernel::task::scheduler::schedule_next();
        kernel::task::scheduler::print_current_task_name();

        uart::write_str(" context:");
        match kernel::task::scheduler::current_task_id() {
            Some(id) => kernel::task::print_task_full_context_by_id(id),
            None => uart::write_str(" none"),
        }

        uart::write_line("");

        timer::disable_timer();

        if kernel::ticks::is_test_complete() {
            gic::end_irq(iar);

            kernel::test::print_test_complete();

            arch::halt();
        }

        timer::arm_timer_hz(TIMER_HZ)
    } else {
        uart::write_line("");
        uart::write_line("=== IRQ ===");
        uart::write_line("unknown IRQ received");

        uart::write_str("GICC_IAR: ");
        uart::write_hex_u64(iar as u64);
        uart::write_line("");

        uart::write_str("interrupt id: ");
        uart::write_dec_u64(irq_id as u64);
        uart::write_line("");
    }

    gic::end_irq(iar);
}

#[cfg(target_arch = "aarch64")]
fn print_timer_info_short() {
    uart::write_str("CNTFRQ_EL0: ");
    uart::write_dec_u64(timer::frequency_hz());
    uart::write_line(" Hz");
}

#[cfg(target_arch = "aarch64")]
#[allow(dead_code)]
fn trigger_test_exception() {
    unsafe {
        asm!("brk #0", options(nomem, nostack, preserves_flags));
    }
}

#[cfg(target_arch = "riscv64")]
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
