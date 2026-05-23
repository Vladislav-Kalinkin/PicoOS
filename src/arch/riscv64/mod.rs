use core::arch::asm;

pub mod cpu;
pub mod timer;
pub mod traps;

core::arch::global_asm!(include_str!("boot.S"));
core::arch::global_asm!(include_str!("trap.S"));

unsafe extern "C" {
    static trap_vector: u8;
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
    let trap_addr = unsafe { &trap_vector as *const u8 as u64 };

    cpu::set_mtvec(trap_addr);

    crate::drivers::uart::write_str("mtvec: ");
    crate::drivers::uart::write_hex_u64(cpu::mtvec());
    crate::drivers::uart::write_line("");
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

#[allow(dead_code)]
pub fn return_from_interrupt() -> ! {
    halt();
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

#[allow(dead_code)]
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

#[allow(dead_code)]
pub unsafe fn restore_task_cpu_context(
    context: crate::kernel::task::cpu_context::TaskCpuContext,
) -> ! {
    core::arch::asm!(
    "mv sp, {sp_in}",

    "mv ra, {resume_pc_in}",

    "mv s0, {s0_in}",
    "mv s1, {s1_in}",
    "mv s2, {s2_in}",
    "mv s3, {s3_in}",
    "mv s4, {s4_in}",
    "mv s5, {s5_in}",
    "mv s6, {s6_in}",
    "mv s7, {s7_in}",
    "mv s8, {s8_in}",
    "mv s9, {s9_in}",
    "mv s10, {s10_in}",
    "mv s11, {s11_in}",

    "ret",

    sp_in = in(reg) context.sp,
    resume_pc_in = in(reg) context.resume_pc,

    s0_in = in(reg) context.s[0],
    s1_in = in(reg) context.s[1],
    s2_in = in(reg) context.s[2],
    s3_in = in(reg) context.s[3],
    s4_in = in(reg) context.s[4],
    s5_in = in(reg) context.s[5],
    s6_in = in(reg) context.s[6],
    s7_in = in(reg) context.s[7],
    s8_in = in(reg) context.s[8],
    s9_in = in(reg) context.s[9],
    s10_in = in(reg) context.s[10],
    s11_in = in(reg) context.s[11],

    options(noreturn)
    );
}

#[allow(dead_code)]
#[inline(always)]
pub fn return_address() -> u64 {
    let ra: u64;

    unsafe {
        core::arch::asm!(
        "mv {ra_out}, ra",
        ra_out = out(reg) ra,
        options(nomem, nostack, preserves_flags),
        );
    }

    ra
}

#[allow(dead_code)]
#[inline(always)]
pub fn resume_stack_pointer() -> u64 {
    stack_pointer()
}

#[allow(dead_code)]
#[inline(always)]
pub fn capture_yield_context() -> (u64, u64) {
    let sp: u64;
    let ra: u64;

    unsafe {
        core::arch::asm!(
        "mv {sp_out}, sp",
        "mv {ra_out}, ra",
        sp_out = out(reg) sp,
        ra_out = out(reg) ra,
        options(nomem, nostack, preserves_flags),
        );
    }

    (sp, ra)
}

#[allow(dead_code)]
pub unsafe fn yield_to_kernel_raw(
    task_sp: u64,
    resume_pc: u64,
    kernel_sp: u64,
    kernel_return_pc: u64,
) -> ! {
    crate::kernel::task::debug::set_debug_task_resume_context(task_sp, resume_pc);
    crate::kernel::task::debug::set_debug_task_return_kind(
        crate::kernel::task::TaskReturnKind::Yield,
    );

    crate::kernel::task::debug::print_debug_task_resume_context();

    return_to_kernel_stack(kernel_sp, kernel_return_pc);
}

#[allow(dead_code)]
pub unsafe fn restore_verified_resume_frame(
    frame: crate::kernel::task::cpu_context::TaskCpuContext,
) -> ! {
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
    restore_resume_frame_asm_stub(frame);
}

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

fn print_restore_contract() {
    crate::drivers::uart::write_line(" assembly restore contract:");
    crate::drivers::uart::write_line(" set sp to verified frame.sp");
    crate::drivers::uart::write_line(" restore ra from verified frame.ra");
    crate::drivers::uart::write_line(" jump to verified frame.resume_pc");
    crate::drivers::uart::write_line(" do not return to caller");
    crate::drivers::uart::write_line(" do not touch kernel stack after switching sp");
}

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

#[inline(never)]
pub unsafe fn yield_to_kernel_and_return(
    task_sp: u64,
    resume_pc: u64,
    kernel_sp: u64,
    return_pc: u64,
) {
    yield_to_kernel_returning_stub(task_sp, resume_pc, kernel_sp, return_pc);
}

#[inline(never)]
unsafe fn yield_to_kernel_returning_stub(
    task_sp: u64,
    resume_pc: u64,
    kernel_sp: u64,
    return_pc: u64,
) {
    crate::drivers::uart::write_line("yield returning stub:");
    crate::drivers::uart::write_line(" mode: placeholder");
    #[cfg(feature = "verbose_resume_debug")]
    print_returning_yield_contract();

    #[cfg(feature = "verbose_resume_debug")]
    {
        crate::drivers::uart::write_line("  RISC-V returning yield ABI inputs:");

        crate::drivers::uart::write_str("    task_sp: ");
        crate::drivers::uart::write_hex_u64(task_sp);
        crate::drivers::uart::write_line("");

        crate::drivers::uart::write_str("    resume_pc: ");
        crate::drivers::uart::write_hex_u64(resume_pc);
        crate::drivers::uart::write_line("");

        crate::drivers::uart::write_str("    kernel_sp: ");
        crate::drivers::uart::write_hex_u64(kernel_sp);
        crate::drivers::uart::write_line("");

        crate::drivers::uart::write_str("    return_pc: ");
        crate::drivers::uart::write_hex_u64(return_pc);
        crate::drivers::uart::write_line("");
    }

    crate::drivers::uart::write_str(" task_sp: ");
    crate::drivers::uart::write_hex_u64(task_sp);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str(" resume_pc: ");
    crate::drivers::uart::write_hex_u64(resume_pc);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str(" kernel_sp: ");
    crate::drivers::uart::write_hex_u64(kernel_sp);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str(" return_pc: ");
    crate::drivers::uart::write_hex_u64(return_pc);
    crate::drivers::uart::write_line("");

    if !validate_returning_yield_abi_inputs(task_sp, resume_pc, kernel_sp, return_pc) {
        crate::drivers::uart::write_line(" returning yield ABI validation failed");
        crate::arch::halt();
    }

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

fn validate_returning_yield_abi_inputs(
    task_sp: u64,
    resume_pc: u64,
    kernel_sp: u64,
    return_pc: u64,
) -> bool {
    crate::drivers::uart::write_line(" RISC-V returning yield ABI validation:");

    let task_sp_nonzero = task_sp != 0;
    let kernel_sp_nonzero = kernel_sp != 0;
    let resume_pc_inside_text = crate::kernel::memory::is_inside_kernel_text(resume_pc);
    let return_pc_inside_text = crate::kernel::memory::is_inside_kernel_text(return_pc);

    crate::drivers::uart::write_str(" task_sp non-zero: ");
    crate::kernel::task::table::print_yes_no(task_sp_nonzero);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str(" kernel_sp non-zero: ");
    crate::kernel::task::table::print_yes_no(kernel_sp_nonzero);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str(" resume_pc inside kernel text: ");
    crate::kernel::task::table::print_yes_no(resume_pc_inside_text);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str(" return_pc inside kernel text: ");
    crate::kernel::task::table::print_yes_no(return_pc_inside_text);
    crate::drivers::uart::write_line("");

    let ok = task_sp_nonzero && kernel_sp_nonzero && resume_pc_inside_text && return_pc_inside_text;

    crate::drivers::uart::write_str(" result: ");
    if ok {
        crate::drivers::uart::write_line("OK");
    } else {
        crate::drivers::uart::write_line("FAILED");
    }

    ok
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

    crate::drivers::uart::write_line(" jumping now; expected next lines:");
    crate::drivers::uart::write_line(" yield_now: resumed after arch yield");
    crate::drivers::uart::write_line(" yielding_task: step 2");
    crate::drivers::uart::write_line(" task exit requested");

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
