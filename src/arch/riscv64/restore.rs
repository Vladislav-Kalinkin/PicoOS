use crate::kernel::trap_frame::{TRAP_FRAME_SIZE, TrapImage};

unsafe extern "C" {
    fn trap_return() -> !;
}

/// Rewrite a trap-stack frame from `image`, `csrw mepc`/`mstatus` (`MPP=M`,
/// `MIE=0`, `MPIE=1`), then enter `trap.S` `trap_return` so the epilogue `mret`s.
/// Timer path does **not** add 4 to `mepc`.
pub fn mret_to_trap_image(image: &TrapImage) -> ! {
    crate::arch::disable_irq();
    crate::kernel::cpu::set_in_trap(false);

    let trap_top = super::trap_stack_top();
    let frame =
        (trap_top - TRAP_FRAME_SIZE as u64) as *mut crate::kernel::trap_frame::Riscv64TrapFrame;

    unsafe {
        *frame = image.gpr;
    }

    super::cpu::set_mepc(image.mepc);
    super::cpu::set_mstatus(super::cpu::synthesize_mstatus_for_mret_worker());

    unsafe {
        core::arch::asm!(
            "mv sp, {frame}",
            "j {epilogue}",
            frame = in(reg) frame,
            epilogue = sym trap_return,
            options(noreturn)
        );
    }
}

/// Worker → idle: reset `mscratch`, switch to the kernel stack, enable MIE,
/// jump to `idle_loop`. Must not `mret` (that would leave `mscratch` as the
/// worker SP).
pub fn idle_exit_from_trap() -> ! {
    crate::kernel::cpu::set_in_trap(false);
    crate::kernel::task::scheduler::switch_to_idle();

    let trap_top = super::trap_stack_top();
    let kernel_sp = {
        let saved = crate::kernel::cpu::kernel_sp_before_task();
        if saved == 0 {
            crate::kernel::memory::stack_top()
        } else {
            saved
        }
    };
    let mstatus = super::cpu::synthesize_mstatus_for_idle();

    unsafe {
        core::arch::asm!(
            "csrw mscratch, {trap_top}",
            "csrw mstatus, {mstatus}",
            "mv sp, {kernel_sp}",
            "j {idle}",
            trap_top = in(reg) trap_top,
            mstatus = in(reg) mstatus,
            kernel_sp = in(reg) kernel_sp,
            idle = sym crate::kernel::task::scheduler::idle_loop,
            options(noreturn)
        );
    }
}

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

    print_riscv_real_resume_success_marker_plan();

    if !print_riscv_real_restore_attempt_guard(frame) {
        crate::drivers::uart::write_line(" real RISC-V restore attempt blocked by guard");
        crate::arch::halt();
    }

    crate::drivers::uart::write_line(" real RISC-V restore attempt guard passed");
    crate::drivers::uart::write_line(" decision: real restore jump enabled");

    unsafe {
        restore_resume_frame_real_jump(frame);
    }
}

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

fn print_riscv_real_restore_attempt_guard(
    frame: crate::kernel::task::cpu_context::TaskCpuContext,
) -> bool {
    crate::drivers::uart::write_line(" RISC-V real restore attempt guard:");

    let frame_valid = frame.is_valid();
    let resume_pc_inside_text = crate::kernel::memory::is_inside_kernel_text(frame.resume_pc);
    let return_pc_inside_text = crate::kernel::memory::is_inside_kernel_text(frame.return_pc);
    let task_sp_nonzero = frame.sp != 0;
    let ra_matches_resume_pc = frame.ra == frame.resume_pc;

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

    let ctx = core::ptr::addr_of!(frame);
    unsafe {
        core::arch::asm!(
            "ld ra, {ra_off}({ctx})",
            "ld s0, {s0_off}({ctx})",
            "ld s1, {s1_off}({ctx})",
            "ld s2, {s2_off}({ctx})",
            "ld s3, {s3_off}({ctx})",
            "ld s4, {s4_off}({ctx})",
            "ld s5, {s5_off}({ctx})",
            "ld s6, {s6_off}({ctx})",
            "ld s7, {s7_off}({ctx})",
            "ld s8, {s8_off}({ctx})",
            "ld s9, {s9_off}({ctx})",
            "ld s10, {s10_off}({ctx})",
            "ld s11, {s11_off}({ctx})",
            "ld t0, {pc_off}({ctx})",
            "ld sp, {sp_off}({ctx})",
            "jr t0",
            ctx = in(reg) ctx,
            sp_off = const core::mem::offset_of!(
                crate::kernel::task::cpu_context::TaskCpuContext,
                sp
            ),
            ra_off = const core::mem::offset_of!(
                crate::kernel::task::cpu_context::TaskCpuContext,
                ra
            ),
            pc_off = const core::mem::offset_of!(
                crate::kernel::task::cpu_context::TaskCpuContext,
                resume_pc
            ),
            s0_off = const core::mem::offset_of!(
                crate::kernel::task::cpu_context::TaskCpuContext,
                s
            ),
            s1_off = const core::mem::offset_of!(
                crate::kernel::task::cpu_context::TaskCpuContext,
                s
            ) + 8,
            s2_off = const core::mem::offset_of!(
                crate::kernel::task::cpu_context::TaskCpuContext,
                s
            ) + 16,
            s3_off = const core::mem::offset_of!(
                crate::kernel::task::cpu_context::TaskCpuContext,
                s
            ) + 24,
            s4_off = const core::mem::offset_of!(
                crate::kernel::task::cpu_context::TaskCpuContext,
                s
            ) + 32,
            s5_off = const core::mem::offset_of!(
                crate::kernel::task::cpu_context::TaskCpuContext,
                s
            ) + 40,
            s6_off = const core::mem::offset_of!(
                crate::kernel::task::cpu_context::TaskCpuContext,
                s
            ) + 48,
            s7_off = const core::mem::offset_of!(
                crate::kernel::task::cpu_context::TaskCpuContext,
                s
            ) + 56,
            s8_off = const core::mem::offset_of!(
                crate::kernel::task::cpu_context::TaskCpuContext,
                s
            ) + 64,
            s9_off = const core::mem::offset_of!(
                crate::kernel::task::cpu_context::TaskCpuContext,
                s
            ) + 72,
            s10_off = const core::mem::offset_of!(
                crate::kernel::task::cpu_context::TaskCpuContext,
                s
            ) + 80,
            s11_off = const core::mem::offset_of!(
                crate::kernel::task::cpu_context::TaskCpuContext,
                s
            ) + 88,
            options(noreturn)
        );
    }
}
