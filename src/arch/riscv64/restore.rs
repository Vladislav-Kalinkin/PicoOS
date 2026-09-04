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
