use core::cell::UnsafeCell;

/// Callee-saved GPRs captured in `task_yield_boundary` before kernel Rust
/// clobbers them. Uniprocessor: written from asm, read on the task-return path.
#[repr(transparent)]
struct YieldSavedS(UnsafeCell<[u64; 12]>);

// SAFETY: one hart; the yield boundary stores these registers before any
// kernel function prologue runs, and the return path reads them before the
// next yield.
unsafe impl Sync for YieldSavedS {}

#[unsafe(no_mangle)]
static YIELD_SAVED_S: YieldSavedS = YieldSavedS(UnsafeCell::new([0; 12]));

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
     *   s0–s11 = task callee-saved (must be stored before a Rust prologue)
     */

    mv t2, a0
    mv t3, a1

    mv t0, sp
    mv t1, ra

    la t4, YIELD_SAVED_S
    sd s0, 0(t4)
    sd s1, 8(t4)
    sd s2, 16(t4)
    sd s3, 24(t4)
    sd s4, 32(t4)
    sd s5, 40(t4)
    sd s6, 48(t4)
    sd s7, 56(t4)
    sd s8, 64(t4)
    sd s9, 72(t4)
    sd s10, 80(t4)
    sd s11, 88(t4)

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

    unsafe {
        core::arch::asm!(
            "mv {ra_out}, ra",
            ra_out = out(reg) ra,
            options(nomem, nostack, preserves_flags),
        );
    }

    // SAFETY: stored by task_yield_boundary on this hart before kernel Rust
    // ran; the return path reads it before the next yield.
    let s = unsafe { *YIELD_SAVED_S.0.get() };

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
    crate::kernel::cpu::set_task_resume_context(task_sp, resume_pc);
    if matches!(
        crate::kernel::cpu::task_return_kind(),
        crate::kernel::task::TaskReturnKind::None
    ) {
        crate::kernel::cpu::set_task_return_kind(crate::kernel::task::TaskReturnKind::Yield);
    }

    crate::kernel::cpu::print_task_resume_context();

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
    crate::drivers::uart::write_line(" mode: s0-s11");

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
