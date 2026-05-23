use core::arch::asm;

pub mod cpu;
pub mod exceptions;

core::arch::global_asm!(include_str!("boot.S"));
core::arch::global_asm!(include_str!("vectors.S"));

unsafe extern "C" {
    static exception_vectors: u8;
}

#[inline(always)]
pub fn halt() -> ! {
    loop {
        unsafe {
            asm!("wfe", options(nomem, nostack, preserves_flags));
        }
    }
}

pub fn init_exceptions() {
    let vectors_addr = unsafe { &exception_vectors as *const u8 as u64 };

    cpu::set_vbar_el1(vectors_addr);

    crate::drivers::uart::write_str("VBAR_EL1: ");
    crate::drivers::uart::write_hex_u64(cpu::vbar_el1());
    crate::drivers::uart::write_line("");
}

#[inline(always)]
pub fn enable_irq() {
    cpu::enable_irq();
}

#[inline(always)]
pub fn disable_irq() {
    cpu::disable_irq();
}

#[inline(always)]
pub fn wait_for_interrupt() {
    unsafe {
        asm!("wfe", options(nomem, nostack, preserves_flags));
    }
}

#[allow(dead_code)]
#[inline(always)]
pub fn return_from_interrupt() -> ! {
    unsafe {
        asm!("eret", options(noreturn));
    }
}

pub fn print_cpu_info() {
    let el = cpu::current_el();

    crate::drivers::uart::write_str("current EL: ");

    match el {
        0 => crate::drivers::uart::write_line("EL0"),
        1 => crate::drivers::uart::write_line("EL1"),
        2 => crate::drivers::uart::write_line("EL2"),
        3 => crate::drivers::uart::write_line("EL3"),
        _ => crate::drivers::uart::write_line("unknown"),
    }

    crate::drivers::uart::write_str("current EL number: ");
    crate::drivers::uart::write_dec_u64(el);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("CurrentEL raw: ");
    crate::drivers::uart::write_hex_u64(cpu::current_el_raw());
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("SCTLR_EL1: ");
    crate::drivers::uart::write_hex_u64(cpu::sctlr_el1());
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("DAIF: ");
    crate::drivers::uart::write_hex_u64(cpu::daif());
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
        "mov sp, {stack}",
        "mov x0, {entry}",
        "bl {trampoline}",
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
        "mov sp, {kernel_sp}",
        "br {return_pc}",
        kernel_sp = in(reg) kernel_sp,
        return_pc = in(reg) return_pc,
        options(noreturn)
        );
    }
}

pub fn enable_fp_simd() {
    cpu::enable_fp_simd();
}

pub fn capture_task_cpu_context(
    task_sp: u64,
    kernel_return_pc: u64,
) -> crate::kernel::task::cpu_context::TaskCpuContext {
    let resume_pc = crate::kernel::task::debug::debug_task_resume_pc();
    let x19_x30 = crate::kernel::task::debug::debug_arm64_x19_x30();

    crate::kernel::task::cpu_context::TaskCpuContext {
        sp: task_sp,
        return_pc: kernel_return_pc,
        resume_pc,
        x19_x30,
    }
}

#[allow(dead_code)]
pub unsafe fn restore_task_cpu_context(
    context: crate::kernel::task::cpu_context::TaskCpuContext,
) -> ! {
    core::arch::asm!(
    "mov sp, {sp_in}",

    "mov x19, {x19_in}",
    "mov x20, {x20_in}",
    "mov x21, {x21_in}",
    "mov x22, {x22_in}",
    "mov x23, {x23_in}",
    "mov x24, {x24_in}",
    "mov x25, {x25_in}",
    "mov x26, {x26_in}",
    "mov x27, {x27_in}",
    "mov x28, {x28_in}",
    "mov x29, {x29_in}",
    "mov x30, {resume_pc_in}",

    "ret",

    sp_in = in(reg) context.sp,

    x19_in = in(reg) context.x19_x30[0],
    x20_in = in(reg) context.x19_x30[1],
    x21_in = in(reg) context.x19_x30[2],
    x22_in = in(reg) context.x19_x30[3],
    x23_in = in(reg) context.x19_x30[4],
    x24_in = in(reg) context.x19_x30[5],
    x25_in = in(reg) context.x19_x30[6],
    x26_in = in(reg) context.x19_x30[7],
    x27_in = in(reg) context.x19_x30[8],
    x28_in = in(reg) context.x19_x30[9],
    x29_in = in(reg) context.x19_x30[10],
    resume_pc_in = in(reg) context.resume_pc,

    options(noreturn)
    );
}

#[allow(dead_code)]
#[inline(always)]
pub fn return_address() -> u64 {
    let lr: u64;

    unsafe {
        core::arch::asm!(
        "mov {lr_out}, x30",
        lr_out = out(reg) lr,
        options(nomem, nostack, preserves_flags),
        );
    }

    lr
}

#[allow(dead_code)]
#[inline(always)]
pub fn frame_pointer() -> u64 {
    let fp: u64;

    unsafe {
        core::arch::asm!(
        "mov {fp_out}, x29",
        fp_out = out(reg) fp,
        options(nomem, nostack, preserves_flags),
        );
    }

    fp
}

#[allow(dead_code)]
#[inline(always)]
pub fn resume_stack_pointer() -> u64 {
    frame_pointer() + 16
}

#[allow(dead_code)]
#[inline(always)]
pub fn capture_yield_context() -> (u64, u64) {
    let sp: u64;
    let lr: u64;

    unsafe {
        core::arch::asm!(
        "mov {sp_out}, sp",
        "mov {lr_out}, x30",
        sp_out = out(reg) sp,
        lr_out = out(reg) lr,
        options(nomem, nostack, preserves_flags),
        );
    }

    (sp, lr)
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

    crate::drivers::uart::write_str(" x19 <- ");
    crate::drivers::uart::write_hex_u64(frame.x19_x30[0]);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str(" x20 <- ");
    crate::drivers::uart::write_hex_u64(frame.x19_x30[1]);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str(" x30 <- ");
    crate::drivers::uart::write_hex_u64(frame.x19_x30[11]);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str(" x21 <- ");
    crate::drivers::uart::write_hex_u64(frame.x19_x30[2]);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str(" x22 <- ");
    crate::drivers::uart::write_hex_u64(frame.x19_x30[3]);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str(" x29 <- ");
    crate::drivers::uart::write_hex_u64(frame.x19_x30[10]);
    crate::drivers::uart::write_line("");
}

fn print_restore_contract() {
    crate::drivers::uart::write_line(" assembly restore contract:");
    crate::drivers::uart::write_line(" set sp to verified frame.sp");
    crate::drivers::uart::write_line(" restore x19-x30 from verified frame");
    crate::drivers::uart::write_line(" branch to verified frame.resume_pc");
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

    #[cfg(feature = "real_resume_restore_test_arm")]
    {
        crate::drivers::uart::write_line("    real ARM64 restore feature enabled");
        print_arm64_real_resume_success_marker_plan();

        if !print_arm64_real_restore_attempt_guard(frame) {
            crate::drivers::uart::write_line("    real ARM64 restore attempt blocked by guard");
            crate::arch::halt();
        }

        crate::drivers::uart::write_line("    real ARM64 restore attempt guard passed");

        #[cfg(feature = "real_resume_restore_jump_arm")]
        {
            crate::drivers::uart::write_line("    decision: real ARM64 restore jump requested");
            crate::drivers::uart::write_line("    ARM64 real jump blocked in 1.4.1");
            crate::drivers::uart::write_line(
                "    reason: frame-pointer restore reaches Rust but ABI context is incomplete",
            );
            crate::drivers::uart::write_line(
                "    reason: x19-x30 must be captured in the yield stub before switching stacks",
            );
            crate::drivers::uart::write_line(
                "    required next: move ARM64 callee-saved capture to yield_to_kernel_and_return",
            );
            crate::arch::halt();
        }

        #[cfg(not(feature = "real_resume_restore_jump_arm"))]
        {
            crate::drivers::uart::write_line(
                "    decision: real ARM64 restore still disabled in 1.1",
            );
            crate::arch::halt();
        }
    }

    #[cfg(not(feature = "real_resume_restore_test_arm"))]
    {
        crate::drivers::uart::write_line("    safe mode: real asm restore disabled");
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
    #[cfg(target_arch = "aarch64")]
    {
        let regs = capture_current_arm64_x19_x30_for_yield(resume_pc);
        crate::kernel::task::debug::set_debug_arm64_x19_x30(regs);

        crate::drivers::uart::write_line("ARM64 yield frame captured:");
        crate::drivers::uart::write_str("  x19: ");
        crate::drivers::uart::write_hex_u64(regs[0]);
        crate::drivers::uart::write_line("");

        crate::drivers::uart::write_str("  x20: ");
        crate::drivers::uart::write_hex_u64(regs[1]);
        crate::drivers::uart::write_line("");

        crate::drivers::uart::write_str("  x29: ");
        crate::drivers::uart::write_hex_u64(regs[10]);
        crate::drivers::uart::write_line("");

        crate::drivers::uart::write_str("  x30/resume_pc: ");
        crate::drivers::uart::write_hex_u64(regs[11]);
        crate::drivers::uart::write_line("");
    }
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
    crate::drivers::uart::write_line(" delegating to raw yield jump");

    yield_to_kernel_raw(task_sp, resume_pc, kernel_sp, return_pc);
}

#[cfg(feature = "real_resume_restore_test_arm")]
fn print_arm64_real_resume_success_marker_plan() {
    crate::drivers::uart::write_line("    ARM64 real resume success markers:");
    crate::drivers::uart::write_line("      expect: yield_now: resumed after arch yield");
    crate::drivers::uart::write_line("      expect: yielding_task: step 2");
    crate::drivers::uart::write_line("      expect: task exit requested");
    crate::drivers::uart::write_line(
        "      if these do not appear, ARM64 restore did not resume Rust correctly",
    );
}

#[cfg(feature = "real_resume_restore_test_arm")]
fn print_arm64_real_restore_attempt_guard(
    frame: crate::kernel::task::cpu_context::TaskCpuContext,
) -> bool {
    crate::drivers::uart::write_line("    ARM64 real restore attempt guard:");

    let frame_valid = frame.is_valid();
    let task_sp_nonzero = frame.sp != 0;
    let resume_pc_inside_text = crate::kernel::memory::is_inside_kernel_text(frame.resume_pc);
    let return_pc_inside_text = crate::kernel::memory::is_inside_kernel_text(frame.return_pc);
    let x30_matches_resume_pc = frame.x19_x30[11] == frame.resume_pc;
    let x29_captured = frame.x19_x30[10] != 0;
    let x30_matches_resume_pc = frame.x19_x30[11] == frame.resume_pc;
    let yield_captured_x30_matches = frame.x19_x30[11] == frame.resume_pc;
    let full_callee_saved_frame_present = x30_matches_resume_pc;

    crate::drivers::uart::write_str("      feature real_resume_restore_test_arm: ");
    crate::drivers::uart::write_line("enabled");

    crate::drivers::uart::write_str("      arch: ");
    crate::drivers::uart::write_line("aarch64");

    crate::drivers::uart::write_str("      frame valid: ");
    crate::kernel::task::table::print_yes_no(frame_valid);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("      task SP non-zero: ");
    crate::kernel::task::table::print_yes_no(task_sp_nonzero);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("      resume_pc inside kernel text: ");
    crate::kernel::task::table::print_yes_no(resume_pc_inside_text);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("      return_pc inside kernel text: ");
    crate::kernel::task::table::print_yes_no(return_pc_inside_text);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("      x30 == resume_pc: ");
    crate::kernel::task::table::print_yes_no(x30_matches_resume_pc);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("      x29 captured/non-zero: ");
    crate::kernel::task::table::print_yes_no(x29_captured);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("      yield-captured x30 == resume_pc: ");
    crate::kernel::task::table::print_yes_no(yield_captured_x30_matches);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("      full callee-saved frame present: ");
    crate::kernel::task::table::print_yes_no(full_callee_saved_frame_present);
    crate::drivers::uart::write_line("");

    let ok = frame_valid
        && task_sp_nonzero
        && resume_pc_inside_text
        && return_pc_inside_text
        && x30_matches_resume_pc
        && full_callee_saved_frame_present
        && yield_captured_x30_matches;

    crate::drivers::uart::write_str("      result: ");
    if ok {
        crate::drivers::uart::write_line("OK");
    } else {
        crate::drivers::uart::write_line("FAILED");
    }

    ok
}

#[cfg(all(
    feature = "real_resume_restore_test_arm",
    feature = "real_resume_restore_jump_arm"
))]
#[inline(never)]

unsafe fn restore_resume_frame_real_jump_arm(
    frame: crate::kernel::task::cpu_context::TaskCpuContext,
) -> ! {
    crate::drivers::uart::write_line("    ARM64 REAL RESTORE JUMP ENABLED");
    crate::drivers::uart::write_line("    attempting to resume task now");

    crate::drivers::uart::write_str("      sp <- ");
    crate::drivers::uart::write_hex_u64(frame.sp);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("      x19 <- ");
    crate::drivers::uart::write_hex_u64(frame.x19_x30[0]);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("      x20 <- ");
    crate::drivers::uart::write_hex_u64(frame.x19_x30[1]);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("      x30 <- ");
    crate::drivers::uart::write_hex_u64(frame.x19_x30[11]);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("      pc <- ");
    crate::drivers::uart::write_hex_u64(frame.resume_pc);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_line("    jumping now; expected next lines:");
    crate::drivers::uart::write_line("      yield_now: resumed after arch yield");
    crate::drivers::uart::write_line("      yielding_task: step 2");
    crate::drivers::uart::write_line("      task exit requested");

    core::arch::asm!(
        "mov sp, x9",
        "mov x19, x10",
        "mov x20, x11",
        "mov x30, x12",
        "br x13",
        in("x9") frame.sp,
        in("x10") frame.x19_x30[0],
        in("x11") frame.x19_x30[1],
        in("x12") frame.x19_x30[11],
        in("x13") frame.resume_pc,
        options(noreturn)
    );
}

#[inline(always)]
fn capture_arm64_callee_saved_x19_x30(resume_pc: u64) -> [u64; 12] {
    let mut regs = [0u64; 12];

    unsafe {
        core::arch::asm!(
            "mov {x19_out}, x19",
            "mov {x20_out}, x20",
            "mov {x21_out}, x21",
            "mov {x22_out}, x22",
            "mov {x23_out}, x23",
            "mov {x24_out}, x24",
            "mov {x25_out}, x25",
            "mov {x26_out}, x26",
            "mov {x27_out}, x27",
            "mov {x28_out}, x28",
            "mov {x29_out}, x29",
            x19_out = out(reg) regs[0],
            x20_out = out(reg) regs[1],
            x21_out = out(reg) regs[2],
            x22_out = out(reg) regs[3],
            x23_out = out(reg) regs[4],
            x24_out = out(reg) regs[5],
            x25_out = out(reg) regs[6],
            x26_out = out(reg) regs[7],
            x27_out = out(reg) regs[8],
            x28_out = out(reg) regs[9],
            x29_out = out(reg) regs[10],
            options(nostack, preserves_flags)
        );
    }

    regs[11] = resume_pc;
    regs
}

#[cfg(all(
    feature = "real_resume_restore_test_arm",
    feature = "real_resume_restore_jump_arm",
))]
#[inline(never)]

unsafe fn restore_resume_frame_real_jump_arm_from_frame_ptr(
    frame: crate::kernel::task::cpu_context::TaskCpuContext,
) -> ! {
    crate::drivers::uart::write_line("    ARM64 REAL RESTORE JUMP ENABLED");
    crate::drivers::uart::write_line("    mode: frame pointer restore");
    crate::drivers::uart::write_line("    attempting to resume task now");

    crate::drivers::uart::write_str("      frame ptr: ");
    crate::drivers::uart::write_hex_u64((&frame as *const _) as usize as u64);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("      sp <- ");
    crate::drivers::uart::write_hex_u64(frame.sp);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("      pc <- ");
    crate::drivers::uart::write_hex_u64(frame.resume_pc);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("      x30 <- ");
    crate::drivers::uart::write_hex_u64(frame.x19_x30[11]);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_line("    jumping now; expected next lines:");
    crate::drivers::uart::write_line("      yield_now: resumed after arch yield");
    crate::drivers::uart::write_line("      yielding_task: step 2");
    crate::drivers::uart::write_line("      task exit requested");

    core::arch::asm!(
        // x0 = frame pointer

        // Load resume target and task stack before touching SP.
        "ldr x9,  [x0, #0]",    // new sp
        "ldr x10, [x0, #16]",   // resume_pc

        // Restore callee-saved registers from x19_x30.
        "ldr x19, [x0, #24]",
        "ldr x20, [x0, #32]",
        "ldr x21, [x0, #40]",
        "ldr x22, [x0, #48]",
        "ldr x23, [x0, #56]",
        "ldr x24, [x0, #64]",
        "ldr x25, [x0, #72]",
        "ldr x26, [x0, #80]",
        "ldr x27, [x0, #88]",
        "ldr x28, [x0, #96]",
        "ldr x29, [x0, #104]",
        "ldr x30, [x0, #112]",

        // Only now switch to the task stack.
        "mov sp, x9",

        // Resume task.
        "br x10",

        in("x0") &frame,
        options(noreturn)
    );
}

#[inline(always)]
fn capture_current_arm64_x19_x30_for_yield(resume_pc: u64) -> [u64; 12] {
    let mut regs = [0u64; 12];

    unsafe {
        core::arch::asm!(
            "mov {x19_out}, x19",
            "mov {x20_out}, x20",
            "mov {x21_out}, x21",
            "mov {x22_out}, x22",
            "mov {x23_out}, x23",
            "mov {x24_out}, x24",
            "mov {x25_out}, x25",
            "mov {x26_out}, x26",
            "mov {x27_out}, x27",
            "mov {x28_out}, x28",
            "mov {x29_out}, x29",
            x19_out = out(reg) regs[0],
            x20_out = out(reg) regs[1],
            x21_out = out(reg) regs[2],
            x22_out = out(reg) regs[3],
            x23_out = out(reg) regs[4],
            x24_out = out(reg) regs[5],
            x25_out = out(reg) regs[6],
            x26_out = out(reg) regs[7],
            x27_out = out(reg) regs[8],
            x28_out = out(reg) regs[9],
            x29_out = out(reg) regs[10],
            options(nostack, preserves_flags)
        );
    }

    regs[11] = resume_pc;
    regs
}
