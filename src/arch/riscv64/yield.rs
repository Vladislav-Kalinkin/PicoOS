#[cfg(target_arch = "riscv64")]
core::arch::global_asm!(
    r#"
    .section .text
    .global task_yield_boundary
    .type task_yield_boundary, @function

task_yield_boundary:
    /*
     * Rust call ABI:
     *   a0 = kernel_sp
     *   a1 = return_pc
     *
     * At function entry:
     *   sp = task stack pointer at call boundary
     *   ra = continuation address after call task_yield_boundary
     */

    mv t2, a0
    mv t3, a1

    mv t0, sp
    mv t1, ra

    /*
     * yield_to_kernel_returning_stub ABI:
     *   a0 = task_sp
     *   a1 = resume_pc
     *   a2 = kernel_sp
     *   a3 = return_pc
     */
    mv a0, t0
    mv a1, t1
    mv a2, t2
    mv a3, t3

    j yield_to_kernel_returning_stub
"#
);

#[cfg(any(
    feature = "task_yield_test",
    feature = "task_sleep_runtime_e2e_test",
    feature = "two_yield_task_test",
    feature = "two_task_resume_handoff_test",
    feature = "scheduler_fault_lifecycle_test"
))]
unsafe extern "C" {
    pub fn task_yield_boundary(kernel_sp: u64, return_pc: u64);
}

pub fn capture_task_cpu_context(
    sp: u64,
    return_pc: u64,
) -> crate::kernel::task::cpu_context::TaskCpuContext {
    let ra: u64;
    let mut s = [0u64; 12];

    unsafe {
        core::arch::asm!(
        "mv {ra_out}, ra",
        "mv {s0_out}, s0",
        "mv {s1_out}, s1",
        "mv {s2_out}, s2",
        "mv {s3_out}, s3",
        "mv {s4_out}, s4",
        "mv {s5_out}, s5",
        "mv {s6_out}, s6",
        "mv {s7_out}, s7",
        "mv {s8_out}, s8",
        "mv {s9_out}, s9",
        "mv {s10_out}, s10",
        "mv {s11_out}, s11",
        ra_out = out(reg) ra,
        s0_out = out(reg) s[0],
        s1_out = out(reg) s[1],
        s2_out = out(reg) s[2],
        s3_out = out(reg) s[3],
        s4_out = out(reg) s[4],
        s5_out = out(reg) s[5],
        s6_out = out(reg) s[6],
        s7_out = out(reg) s[7],
        s8_out = out(reg) s[8],
        s9_out = out(reg) s[9],
        s10_out = out(reg) s[10],
        s11_out = out(reg) s[11],
        options(nomem, nostack, preserves_flags),
        );
    }

    crate::kernel::task::cpu_context::TaskCpuContext {
        sp,
        return_pc,
        resume_pc: ra,
        ra,
        s,
    }
}

pub fn yield_to_kernel_raw(
    task_sp: u64,
    resume_pc: u64,
    kernel_sp: u64,
    kernel_return_pc: u64,
) -> ! {
    crate::kernel::task::debug::set_debug_task_resume_context(task_sp, resume_pc);
    if matches!(
        crate::kernel::task::debug::debug_task_return_kind(),
        crate::kernel::task::TaskReturnKind::None
    ) {
        crate::kernel::task::debug::set_debug_task_return_kind(
            crate::kernel::task::TaskReturnKind::Yield,
        );
    }

    crate::kernel::task::debug::print_debug_task_resume_context();

    super::return_to_kernel_stack_checked(kernel_sp, kernel_return_pc);
}

#[cfg(target_arch = "riscv64")]
#[unsafe(no_mangle)]
#[inline(never)]
pub unsafe extern "C" fn yield_to_kernel_returning_stub(
    task_sp: u64,
    resume_pc: u64,
    kernel_sp: u64,
    return_pc: u64,
) -> ! {
    crate::drivers::uart::write_line("yield returning stub:");
    crate::drivers::uart::write_line(" mode: placeholder");

    crate::drivers::uart::write_line(" delegating to raw yield jump");

    yield_to_kernel_raw(task_sp, resume_pc, kernel_sp, return_pc);
}

#[cfg(feature = "verbose_resume_debug")]
fn print_returning_yield_contract() {
    crate::drivers::uart::write_line(" RISC-V returning yield contract:");
    crate::drivers::uart::write_line(" capture task SP before switching stacks");
    crate::drivers::uart::write_line(" capture resume PC as point after yield call");
    crate::drivers::uart::write_line(" save kernel SP and kernel return PC");
    crate::drivers::uart::write_line(" switch from task stack to kernel stack");
    crate::drivers::uart::write_line(" enter kernel task return handler");
    crate::drivers::uart::write_line(
        " after future restore, yield_to_kernel_and_return must return normally",
    );
    crate::drivers::uart::write_line(" Rust code after yield_now must be reachable");
}
