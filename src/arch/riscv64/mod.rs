use core::arch::asm;

pub mod cpu;
pub mod timer;
pub mod traps;

core::arch::global_asm!(include_str!("boot.S"));
core::arch::global_asm!(include_str!("trap.S"));

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

unsafe extern "C" {
    static trap_vector: u8;
    static __trap_stack_top: u8;
}

#[inline(always)]
fn symbol_addr(symbol: *const u8) -> u64 {
    symbol as u64
}

#[inline(always)]
pub fn halt() -> ! {
    loop {
        unsafe {
            asm!("wfi", options(nomem, nostack, preserves_flags));
        }
    }
}

pub fn init_exceptions() {
    let trap_addr = symbol_addr(core::ptr::addr_of!(trap_vector));
    let trap_stack_top = trap_stack_top();

    cpu::set_mtvec(trap_addr);
    cpu::set_mscratch(trap_stack_top);

    crate::drivers::uart::write_str("mtvec: ");
    crate::drivers::uart::write_hex_u64(cpu::mtvec());
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("trap stack top: ");
    crate::drivers::uart::write_hex_u64(trap_stack_top);
    crate::drivers::uart::write_line("");
}

fn trap_stack_top() -> u64 {
    symbol_addr(core::ptr::addr_of!(__trap_stack_top))
}

pub fn reset_trap_stack_pointer_for_next_trap() {
    cpu::set_mscratch(trap_stack_top());
}

#[cfg(feature = "kernel_fault_guard_test")]
pub fn is_trap_stack_addr(addr: u64) -> bool {
    let top = trap_stack_top();

    addr >= top - 4096 && addr < top
}

pub fn enable_irq() {
    cpu::enable_machine_interrupts();
}

pub fn disable_irq() {
    cpu::disable_machine_interrupts();
}

#[inline(always)]
pub fn wait_for_interrupt() {
    unsafe {
        asm!("wfi", options(nomem, nostack, preserves_flags));
    }
}

pub fn print_cpu_info() {
    crate::drivers::uart::write_line("riscv64 CPU info:");

    crate::drivers::uart::write_str("mhartid: ");
    crate::drivers::uart::write_hex_u64(cpu::mhartid());
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("mstatus: ");
    crate::drivers::uart::write_hex_u64(cpu::mstatus());
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("mtvec: ");
    crate::drivers::uart::write_hex_u64(cpu::mtvec());
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("mepc: ");
    crate::drivers::uart::write_hex_u64(cpu::mepc());
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("mcause: ");
    crate::drivers::uart::write_hex_u64(cpu::mcause());
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("mie: ");
    crate::drivers::uart::write_hex_u64(cpu::mie());
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("mip: ");
    crate::drivers::uart::write_hex_u64(cpu::mip());
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("SP: ");
    crate::drivers::uart::write_hex_u64(cpu::stack_pointer());
    crate::drivers::uart::write_line("");
}

#[allow(dead_code)]
#[cfg(feature = "task_yield_test")]
#[inline(never)]
pub unsafe fn start_task_on_stack(entry: usize, stack_top: u64) -> ! {
    unsafe {
        asm!(
        "mv sp, {stack}",
        "mv a0, {entry}",
        "call {trampoline}",
        stack = in(reg) stack_top,
        entry = in(reg) entry,
        trampoline = sym crate::kernel::task::task_trampoline_raw,
        options(noreturn)
        );
    }
}

#[inline(always)]
pub fn stack_pointer() -> u64 {
    cpu::stack_pointer()
}

#[inline(never)]
pub unsafe fn return_to_kernel_stack(kernel_sp: u64, return_pc: u64) -> ! {
    unsafe {
        asm!(
        "mv sp, {kernel_sp}",
        "jr {return_pc}",
        kernel_sp = in(reg) kernel_sp,
        return_pc = in(reg) return_pc,
        options(noreturn)
        );
    }
}

pub fn return_to_kernel_stack_checked(kernel_sp: u64, return_pc: u64) -> ! {
    if kernel_sp == 0 || !crate::kernel::memory::is_inside_kernel_text(return_pc) {
        crate::drivers::uart::write_line("invalid kernel return context");
        crate::arch::halt();
    }

    reset_trap_stack_pointer_for_next_trap();

    unsafe {
        return_to_kernel_stack(kernel_sp, return_pc);
    }
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

    return_to_kernel_stack_checked(kernel_sp, kernel_return_pc);
}

#[cfg(any(
    feature = "resume_restore_test",
    feature = "scheduler_dispatch_test",
    feature = "timer_preemption_prototype",
    feature = "two_task_resume_handoff_test",
))]
#[allow(dead_code)]
pub fn restore_verified_resume_frame(frame: crate::kernel::task::cpu_context::TaskCpuContext) -> ! {
    crate::drivers::uart::write_line("arch restore verified resume frame:");

    crate::drivers::uart::write_str(" restore sp: ");
    crate::drivers::uart::write_hex_u64(frame.sp);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str(" restore resume_pc: ");
    crate::drivers::uart::write_hex_u64(frame.resume_pc);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str(" restore return_pc: ");
    crate::drivers::uart::write_hex_u64(frame.return_pc);
    crate::drivers::uart::write_line("");

    let frame_valid = frame.is_valid();
    let resume_pc_inside_text = crate::kernel::memory::is_inside_kernel_text(frame.resume_pc);
    let return_pc_inside_text = crate::kernel::memory::is_inside_kernel_text(frame.return_pc);

    crate::drivers::uart::write_line(" arch restore preconditions:");

    crate::drivers::uart::write_str(" frame valid: ");
    crate::kernel::task::table::print_yes_no(frame_valid);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str(" resume_pc inside kernel text: ");
    crate::kernel::task::table::print_yes_no(resume_pc_inside_text);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str(" return_pc inside kernel text: ");
    crate::kernel::task::table::print_yes_no(return_pc_inside_text);
    crate::drivers::uart::write_line("");

    let ok = frame_valid && resume_pc_inside_text && return_pc_inside_text;

    crate::drivers::uart::write_str(" result: ");
    if ok {
        crate::drivers::uart::write_line("OK");
    } else {
        crate::drivers::uart::write_line("FAILED");
        crate::arch::halt();
    }

    print_restore_plan(frame);
    print_restore_contract();

    crate::drivers::uart::write_line(" calling disabled assembly restore stub...");
    unsafe {
        restore_resume_frame_asm_stub(frame);
    }
}

#[cfg(any(
    feature = "resume_restore_test",
    feature = "scheduler_dispatch_test",
    feature = "timer_preemption_prototype",
    feature = "two_task_resume_handoff_test",
))]
fn print_restore_plan(frame: crate::kernel::task::cpu_context::TaskCpuContext) {
    crate::drivers::uart::write_line(" restore plan:");

    crate::drivers::uart::write_str(" sp <- ");
    crate::drivers::uart::write_hex_u64(frame.sp);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str(" pc <- ");
    crate::drivers::uart::write_hex_u64(frame.resume_pc);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str(" ra <- ");
    crate::drivers::uart::write_hex_u64(frame.ra);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str(" return_pc/debug <- ");
    crate::drivers::uart::write_hex_u64(frame.return_pc);
    crate::drivers::uart::write_line("");
}

#[cfg(any(
    feature = "resume_restore_test",
    feature = "scheduler_dispatch_test",
    feature = "timer_preemption_prototype",
    feature = "two_task_resume_handoff_test",
))]
fn print_restore_contract() {
    crate::drivers::uart::write_line(" assembly restore contract:");
    crate::drivers::uart::write_line(" set sp to verified frame.sp");
    crate::drivers::uart::write_line(" restore ra from verified frame.ra");
    crate::drivers::uart::write_line(" jump to verified frame.resume_pc");
    crate::drivers::uart::write_line(" do not return to caller");
    crate::drivers::uart::write_line(" do not touch kernel stack after switching sp");
}

#[cfg(any(
    feature = "resume_restore_test",
    feature = "scheduler_dispatch_test",
    feature = "timer_preemption_prototype",
    feature = "two_task_resume_handoff_test",
))]
#[inline(never)]
unsafe fn restore_resume_frame_asm_stub(
    frame: crate::kernel::task::cpu_context::TaskCpuContext,
) -> ! {
    crate::drivers::uart::write_line(" restore_resume_frame_asm_stub:");
    crate::drivers::uart::write_line(" asm boundary reached");
    crate::drivers::uart::write_line(" this function must not return");
    crate::drivers::uart::write_str(" received sp: ");
    crate::drivers::uart::write_hex_u64(frame.sp);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str(" received resume_pc: ");
    crate::drivers::uart::write_hex_u64(frame.resume_pc);
    crate::drivers::uart::write_line("");

    #[cfg(feature = "real_resume_restore_test")]
    {
        crate::drivers::uart::write_line(" real resume restore feature enabled");
        print_riscv_real_resume_success_marker_plan();

        if !print_riscv_real_restore_attempt_guard(frame) {
            crate::drivers::uart::write_line(" real RISC-V restore attempt blocked by guard");
            crate::arch::halt();
        }

        crate::drivers::uart::write_line(" real RISC-V restore attempt guard passed");

        #[cfg(feature = "real_resume_restore_jump")]
        {
            crate::drivers::uart::write_line(" decision: real restore jump enabled");
            unsafe {
                restore_resume_frame_real_jump(frame);
            }
        }

        #[cfg(not(feature = "real_resume_restore_jump"))]
        {
            crate::drivers::uart::write_line(
                " decision: real restore still disabled without real_resume_restore_jump",
            );
            crate::arch::halt();
        }
    }

    #[cfg(not(feature = "real_resume_restore_test"))]
    {
        crate::drivers::uart::write_line(" safe mode: real asm restore disabled");
        crate::arch::halt();
    }
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

#[cfg(feature = "real_resume_restore_test")]
fn print_riscv_real_resume_success_marker_plan() {
    #[cfg(feature = "scheduler_resume_loop_test")]
    {
        crate::drivers::uart::write_line(" RISC-V real resume success markers:");
        crate::drivers::uart::write_line(" expect: scheduler-driven repeated resume loop");
        crate::drivers::uart::write_line(" expect: two_yielding_task: step 2");
        crate::drivers::uart::write_line(" expect: second yield request");
        crate::drivers::uart::write_line(" expect: two_yielding_task: step 3");
        crate::drivers::uart::write_line(" expect: task exit requested");
        crate::drivers::uart::write_line(" expect: scheduler resume loop result: OK");
        crate::drivers::uart::write_line(" if these do not appear, scheduler resume loop failed");
    }

    #[cfg(all(
        feature = "two_yield_task_test",
        not(feature = "scheduler_resume_loop_test")
    ))]
    {
        crate::drivers::uart::write_line(" RISC-V real resume success markers:");
        crate::drivers::uart::write_line(" expect: two_yielding_task: step 2");
        crate::drivers::uart::write_line(" expect: second yield request");
        crate::drivers::uart::write_line(" expect: two_yielding_task: step 3");
        crate::drivers::uart::write_line(" expect: task exit requested");
        crate::drivers::uart::write_line(
            " if these do not appear, repeated resume did not work correctly",
        );
    }

    #[cfg(not(any(
        feature = "two_yield_task_test",
        feature = "scheduler_resume_loop_test"
    )))]
    {
        crate::drivers::uart::write_line(" RISC-V real resume success markers:");
        crate::drivers::uart::write_line(" expect: yield_now: resumed after arch yield");
        crate::drivers::uart::write_line(" expect: yielding_task: step 2");
        crate::drivers::uart::write_line(" expect: task exit requested");
        crate::drivers::uart::write_line(
            " if these do not appear, restore did not resume Rust correctly",
        );
    }
}

#[cfg(feature = "real_resume_restore_test")]
fn print_riscv_real_restore_attempt_guard(
    frame: crate::kernel::task::cpu_context::TaskCpuContext,
) -> bool {
    crate::drivers::uart::write_line(" RISC-V real restore attempt guard:");

    let frame_valid = frame.is_valid();
    let resume_pc_inside_text = crate::kernel::memory::is_inside_kernel_text(frame.resume_pc);
    let return_pc_inside_text = crate::kernel::memory::is_inside_kernel_text(frame.return_pc);
    let task_sp_nonzero = frame.sp != 0;
    let ra_matches_resume_pc = frame.ra == frame.resume_pc;

    crate::drivers::uart::write_str(" feature real_resume_restore_test: ");
    crate::drivers::uart::write_line("enabled");

    crate::drivers::uart::write_str(" arch: ");
    crate::drivers::uart::write_line("riscv64");

    crate::drivers::uart::write_str(" frame valid: ");
    crate::kernel::task::table::print_yes_no(frame_valid);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str(" task SP non-zero: ");
    crate::kernel::task::table::print_yes_no(task_sp_nonzero);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str(" resume_pc inside kernel text: ");
    crate::kernel::task::table::print_yes_no(resume_pc_inside_text);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str(" return_pc inside kernel text: ");
    crate::kernel::task::table::print_yes_no(return_pc_inside_text);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str(" ra == resume_pc: ");
    crate::kernel::task::table::print_yes_no(ra_matches_resume_pc);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str(" success markers documented: ");
    crate::kernel::task::table::print_yes_no(true);
    crate::drivers::uart::write_line("");

    let ok = frame_valid
        && task_sp_nonzero
        && resume_pc_inside_text
        && return_pc_inside_text
        && ra_matches_resume_pc;

    crate::drivers::uart::write_str(" result: ");
    if ok {
        crate::drivers::uart::write_line("OK");
    } else {
        crate::drivers::uart::write_line("FAILED");
    }

    ok
}

#[cfg(all(
    feature = "real_resume_restore_test",
    feature = "real_resume_restore_jump"
))]
#[inline(never)]
unsafe fn restore_resume_frame_real_jump(
    frame: crate::kernel::task::cpu_context::TaskCpuContext,
) -> ! {
    crate::drivers::uart::write_line(" RISC-V REAL RESTORE JUMP ENABLED");
    crate::drivers::uart::write_line(" attempting to resume task now");

    crate::drivers::uart::write_str(" sp <- ");
    crate::drivers::uart::write_hex_u64(frame.sp);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str(" ra <- ");
    crate::drivers::uart::write_hex_u64(frame.ra);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str(" pc <- ");
    crate::drivers::uart::write_hex_u64(frame.resume_pc);
    crate::drivers::uart::write_line("");

    #[cfg(feature = "two_task_resume_handoff_test")]
    {
        crate::drivers::uart::write_line(" jumping now; expected next lines:");
        crate::drivers::uart::write_line(" yield_now: resumed after RISC-V boundary");
        crate::drivers::uart::write_line(" handoff worker resumes after yield");
        crate::drivers::uart::write_line(" worker either yields again or exits");
    }

    #[cfg(not(feature = "two_task_resume_handoff_test"))]
    {
        crate::drivers::uart::write_line(" jumping now; expected next lines:");
        crate::drivers::uart::write_line(" yield_now: resumed after arch yield");
        crate::drivers::uart::write_line(" yielding_task: step 2");
        crate::drivers::uart::write_line(" task exit requested");
    }

    unsafe {
        core::arch::asm!(
            "mv sp, {new_sp}",
            "mv ra, {new_ra}",
            "jr {resume_pc}",
            new_sp = in(reg) frame.sp,
            new_ra = in(reg) frame.ra,
            resume_pc = in(reg) frame.resume_pc,
            options(noreturn)
        );
    }
}
