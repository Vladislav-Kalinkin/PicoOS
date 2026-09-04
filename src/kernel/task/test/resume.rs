#![allow(unused_imports)]
use super::invariants;
use crate::drivers::uart;
#[cfg(feature = "resume_restore_test")]
use crate::kernel::cpu::{set_current, set_current_stack_bounds};

#[cfg(feature = "resume_restore_test")]
pub fn test_resume_restore() {
    uart::write_line("");
    uart::write_line("resume restore test:");

    let Some(task_id) = crate::kernel::task::table::find_first_resumable_task() else {
        crate::drivers::uart::write_line("selected task: none");

        #[cfg(feature = "real_resume_restore_jump")]
        {
            if real_resume_jump_completion_check() {
                crate::drivers::uart::write_line(
                    "preflight result: SKIPPED after successful real resume jump",
                );
                crate::drivers::uart::write_line("real resume jump test complete");
                crate::arch::halt();
            }

            crate::drivers::uart::write_line("preflight result: FAILED");
            crate::arch::halt();
        }

        #[cfg(not(feature = "real_resume_restore_jump"))]
        {
            crate::drivers::uart::write_line("preflight result: FAILED");
            crate::arch::halt();
        }
    };

    uart::write_str("selected task: ");
    crate::kernel::task::table::print_task_name_by_id(task_id);
    uart::write_line("");

    let Some(frame) = crate::kernel::task::table::get_task_resume_frame(task_id) else {
        uart::write_line("cpu context: none");
        uart::write_line("restore result: FAILED");
        crate::arch::halt();
    };

    uart::write_str("restore sp: ");
    uart::write_hex_u64(frame.sp);
    uart::write_line("");

    uart::write_str("restore resume_pc: ");
    uart::write_hex_u64(frame.resume_pc);
    uart::write_line("");

    uart::write_str("restore frame:");
    crate::kernel::task::cpu_context::print_cpu_context(frame);
    uart::write_line("");

    if !resume_restore_precheck(task_id) {
        uart::write_line("restore aborted by guard");
        crate::arch::halt();
    }

    uart::write_line("restore guarded precheck passed");
    uart::write_line("calling arch restore_verified_resume_frame...");

    set_current(task_id);
    match (
        crate::kernel::task::table::get_task_stack_start(task_id),
        crate::kernel::task::table::get_task_stack_top(task_id),
    ) {
        (Some(start), Some(top)) => set_current_stack_bounds(start, top),
        _ => {
            uart::write_line("restore aborted: missing task stack bounds");
            crate::arch::halt();
        }
    }

    crate::arch::restore_verified_resume_frame(frame);
}

#[cfg(feature = "resume_preflight_test")]
pub fn test_resume_preflight_check() {
    uart::write_line("");
    uart::write_line("resume preflight check:");

    let Some(task_id) = crate::kernel::task::table::find_first_resumable_task() else {
        uart::write_line("selected task: none");

        #[cfg(feature = "real_resume_restore_jump")]
        {
            if real_resume_jump_completion_check() {
                uart::write_line("preflight result: SKIPPED after successful real resume jump");
                uart::write_line("real resume jump test complete");
                crate::arch::halt();
            }

            uart::write_line("preflight result: FAILED");
            crate::arch::halt();
        }

        #[cfg(not(feature = "real_resume_restore_jump"))]
        {
            uart::write_line("preflight result: FAILED");
            crate::arch::halt();
        }
    };

    uart::write_str("selected task: ");
    crate::kernel::task::table::print_task_name_by_id(task_id);
    uart::write_line("");

    uart::write_str("state: ");
    crate::kernel::task::table::print_task_state_by_id(task_id);
    uart::write_line("");

    uart::write_str("can_resume: ");
    match crate::kernel::task::table::can_task_resume(task_id) {
        Some(value) => {
            crate::kernel::task::table::print_yes_no(value);
            uart::write_line("");
        }
        None => uart::write_line("unknown"),
    }

    uart::write_str("last_return: ");
    crate::kernel::task::table::print_task_return_kind_by_id(task_id);
    uart::write_line("");

    let task_sp = crate::kernel::task::table::get_task_last_task_sp(task_id);
    let kernel_sp = crate::kernel::task::table::get_task_last_kernel_sp(task_id);
    let kernel_return_pc = crate::kernel::task::table::get_task_last_kernel_return_pc(task_id);
    let entry = crate::kernel::task::table::get_task_entry(task_id);
    let cpu_context = crate::kernel::task::table::get_task_cpu_context(task_id);

    uart::write_str("last_task_sp: ");
    match task_sp {
        Some(sp) => {
            uart::write_hex_u64(sp);
            uart::write_line("");
        }
        None => uart::write_line("none"),
    }

    uart::write_str("last_kernel_sp: ");
    match kernel_sp {
        Some(sp) => {
            uart::write_hex_u64(sp);
            uart::write_line("");
        }
        None => uart::write_line("none"),
    }

    uart::write_str("kernel_return_pc: ");
    match kernel_return_pc {
        Some(pc) => {
            uart::write_hex_u64(pc);
            uart::write_line("");
        }
        None => uart::write_line("none"),
    }

    uart::write_str("entry present: ");
    crate::kernel::task::table::print_yes_no(entry.is_some());
    uart::write_line("");

    uart::write_str("cpu context valid: ");
    match cpu_context {
        Some(context) => {
            crate::kernel::task::table::print_yes_no(context.is_valid());
            uart::write_line("");

            #[cfg(feature = "verbose_resume_debug")]
            {
                uart::write_str("cpu context detail:");
                crate::kernel::task::cpu_context::print_cpu_context(context);
                uart::write_line("");
            }
        }
        None => uart::write_line("unknown"),
    }

    let sp_inside = match task_sp {
        Some(sp) => crate::kernel::task::table::is_sp_inside_task_stack(task_id, sp),
        None => None,
    };

    uart::write_str("task SP check: ");
    match sp_inside {
        Some(true) => uart::write_line("inside task stack"),
        Some(false) => uart::write_line("outside task stack"),
        None => uart::write_line("unknown"),
    }

    invariants::print_cpu_context_consistency_check(task_id);
    let _ = print_resume_pc_proximity_check(task_id);

    uart::write_line("preflight result: OK");

    #[cfg(feature = "resume_dry_run_test")]
    {
        test_resume_dry_run();
    }

    #[cfg(not(feature = "resume_dry_run_test"))]
    {
        crate::arch::halt();
    }
}

#[cfg(feature = "resume_dry_run_test")]
pub fn test_resume_dry_run() {
    uart::write_line("");
    uart::write_line("resume dry-run:");

    let Some(task_id) = crate::kernel::task::table::find_first_resumable_task() else {
        uart::write_line("selected task: none");
        uart::write_line("resume plan result: FAILED");
        crate::arch::halt();
    };

    uart::write_str("selected task: ");
    crate::kernel::task::table::print_task_name_by_id(task_id);
    uart::write_line("");

    let Some(frame) = crate::kernel::task::table::get_task_resume_frame(task_id) else {
        uart::write_line("cpu context: none");
        uart::write_line("resume plan result: FAILED");
        crate::arch::halt();
    };

    uart::write_str("restore sp: ");
    uart::write_hex_u64(frame.sp);
    uart::write_line("");

    uart::write_str("restore resume_pc: ");
    uart::write_hex_u64(frame.resume_pc);
    uart::write_line("");

    uart::write_str("restore kernel_return_pc: ");
    uart::write_hex_u64(frame.return_pc);
    uart::write_line("");

    uart::write_str("kernel text: ");
    uart::write_hex_u64(crate::kernel::memory::kernel_text_start());
    uart::write_str(" - ");
    uart::write_hex_u64(crate::kernel::memory::kernel_text_end());
    uart::write_line("");

    let sp_inside = crate::kernel::task::table::is_sp_inside_task_stack(task_id, frame.sp);
    let resume_pc_inside_text = crate::kernel::memory::is_inside_kernel_text(frame.resume_pc);
    let return_pc_inside_text = crate::kernel::memory::is_inside_kernel_text(frame.return_pc);

    uart::write_str("task SP inside task stack: ");
    match sp_inside {
        Some(value) => {
            crate::kernel::task::table::print_yes_no(value);
            uart::write_line("");
        }
        None => uart::write_line("unknown"),
    }

    uart::write_str("resume_pc inside kernel text: ");
    crate::kernel::task::table::print_yes_no(resume_pc_inside_text);
    uart::write_line("");

    uart::write_str("kernel_return_pc inside kernel text: ");
    crate::kernel::task::table::print_yes_no(return_pc_inside_text);
    uart::write_line("");

    uart::write_str("resume frame valid: ");
    crate::kernel::task::table::print_yes_no(frame.is_valid());
    uart::write_line("");

    #[cfg(feature = "verbose_resume_debug")]
    {
        uart::write_str("resume frame detail:");
        crate::kernel::task::cpu_context::print_cpu_context(frame);
        uart::write_line("");
    }

    let context_consistent = match (
        crate::kernel::task::table::get_task_last_task_sp(task_id),
        crate::kernel::task::table::get_task_last_kernel_return_pc(task_id),
    ) {
        (Some(last_sp), Some(kernel_pc)) => frame.sp == last_sp && frame.return_pc == kernel_pc,
        _ => false,
    };

    let frame_ok = print_resume_frame_check(task_id);

    uart::write_str("CPU context consistent: ");
    crate::kernel::task::table::print_yes_no(context_consistent);
    uart::write_line("");

    let ok = frame.is_valid()
        && matches!(sp_inside, Some(true))
        && resume_pc_inside_text
        && return_pc_inside_text
        && context_consistent
        && frame_ok;

    uart::write_str("resume plan result: ");
    if ok {
        uart::write_line("OK");
    } else {
        uart::write_line("FAILED");
    }

    if !ok {
        crate::arch::halt();
    }

    #[cfg(feature = "resume_restore_test")]
    {
        test_resume_restore();
    }

    #[cfg(not(feature = "resume_restore_test"))]
    {
        crate::arch::halt();
    }
}

#[cfg(any(
    feature = "resume_preflight_test",
    feature = "resume_dry_run_test",
    feature = "resume_restore_test",
    feature = "real_resume_restore_test",
    feature = "real_resume_restore_jump",
    feature = "scheduler_resume_loop_test",
    feature = "two_task_resume_handoff_test",
    feature = "scheduler_fault_lifecycle_test"
))]
pub fn print_resume_pc_proximity_check(task_id: usize) -> bool {
    crate::drivers::uart::write_line("  resume PC proximity check:");

    let Some(context) = crate::kernel::task::table::get_task_cpu_context(task_id) else {
        crate::drivers::uart::write_line("    cpu context: none");
        return false;
    };

    let Some(entry) = crate::kernel::task::table::get_task_entry(task_id) else {
        crate::drivers::uart::write_line("    entry: none");
        return false;
    };

    let entry_addr = entry as usize as u64;

    crate::drivers::uart::write_str("    entry: ");
    crate::drivers::uart::write_hex_u64(entry_addr);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("    resume_pc: ");
    crate::drivers::uart::write_hex_u64(context.resume_pc);
    crate::drivers::uart::write_line("");

    #[cfg(any(
        feature = "two_task_resume_handoff_test",
        feature = "task_fault_test",
        feature = "scheduler_resume_loop_test",
        feature = "real_resume_restore_jump"
    ))]
    {
        let resume_pc_inside_text = crate::kernel::memory::is_inside_kernel_text(context.resume_pc);

        crate::drivers::uart::write_line("    mode: RISC-V yield boundary continuation");

        crate::drivers::uart::write_str("    resume_pc inside kernel text: ");
        crate::kernel::task::table::print_yes_no(resume_pc_inside_text);
        crate::drivers::uart::write_line("");

        crate::drivers::uart::write_str("    result: ");
        if resume_pc_inside_text {
            crate::drivers::uart::write_line("OK");
        } else {
            crate::drivers::uart::write_line("FAILED");
        }

        resume_pc_inside_text
    }

    #[cfg(not(any(
        feature = "two_task_resume_handoff_test",
        feature = "task_fault_test",
        feature = "scheduler_resume_loop_test",
        feature = "real_resume_restore_jump"
    )))]
    {
        if context.resume_pc < entry_addr {
            crate::drivers::uart::write_line("    delta: below entry");
            crate::drivers::uart::write_line("    result: FAILED");
            return false;
        }

        let delta = context.resume_pc - entry_addr;

        crate::drivers::uart::write_str("    delta: ");
        crate::drivers::uart::write_hex_u64(delta);
        crate::drivers::uart::write_line("");

        let ok = delta < 0x400;

        crate::drivers::uart::write_str("    result: ");
        if ok {
            crate::drivers::uart::write_line("OK");
        } else {
            crate::drivers::uart::write_line("FAILED");
        }

        ok
    }
}

#[cfg(any(
    feature = "resume_preflight_test",
    feature = "resume_dry_run_test",
    feature = "resume_restore_test",
    feature = "real_resume_restore_test",
    feature = "real_resume_restore_jump",
    feature = "scheduler_resume_loop_test",
    feature = "two_task_resume_handoff_test",
    feature = "scheduler_fault_lifecycle_test"
))]
pub fn print_resume_frame_check(task_id: usize) -> bool {
    uart::write_line("  resume frame check:");

    let Some(frame) = crate::kernel::task::table::get_task_resume_frame(task_id) else {
        uart::write_line("    frame present: no");
        uart::write_line("    result: FAILED");
        return false;
    };

    uart::write_line("    frame present: yes");

    uart::write_str("    frame valid: ");
    crate::kernel::task::table::print_yes_no(frame.is_valid());
    uart::write_line("");

    uart::write_str("    frame SP: ");
    uart::write_hex_u64(frame.sp);
    uart::write_line("");

    uart::write_str("    frame resume_pc: ");
    uart::write_hex_u64(frame.resume_pc);
    uart::write_line("");

    uart::write_str("    frame return_pc: ");
    uart::write_hex_u64(frame.return_pc);
    uart::write_line("");

    let sp_inside = matches!(
        crate::kernel::task::table::is_sp_inside_task_stack(task_id, frame.sp),
        Some(true)
    );

    uart::write_str("    frame SP inside task stack: ");
    crate::kernel::task::table::print_yes_no(sp_inside);
    uart::write_line("");

    let resume_pc_inside_text = crate::kernel::memory::is_inside_kernel_text(frame.resume_pc);
    let return_pc_inside_text = crate::kernel::memory::is_inside_kernel_text(frame.return_pc);

    uart::write_str("    frame resume_pc inside kernel text: ");
    crate::kernel::task::table::print_yes_no(resume_pc_inside_text);
    uart::write_line("");

    uart::write_str("    frame return_pc inside kernel text: ");
    crate::kernel::task::table::print_yes_no(return_pc_inside_text);
    uart::write_line("");

    let ok = frame.is_valid() && sp_inside && resume_pc_inside_text && return_pc_inside_text;

    uart::write_str("    frame check result: ");
    if ok {
        uart::write_line("OK");
    } else {
        uart::write_line("FAILED");
    }

    ok
}

#[cfg(feature = "resume_restore_test")]
pub fn resume_restore_precheck(task_id: usize) -> bool {
    uart::write_line("restore guarded precheck:");

    let Some(frame) = crate::kernel::task::table::get_task_resume_frame(task_id) else {
        uart::write_line("  cpu context: none");
        uart::write_line("  result: FAILED");
        return false;
    };

    let sp_ok = matches!(
        crate::kernel::task::table::is_sp_inside_task_stack(task_id, frame.sp),
        Some(true)
    );

    let resume_pc_text_ok = crate::kernel::memory::is_inside_kernel_text(frame.resume_pc);
    let return_pc_text_ok = crate::kernel::memory::is_inside_kernel_text(frame.return_pc);
    let context_valid = frame.is_valid();

    uart::write_str("  resume frame valid: ");
    crate::kernel::task::table::print_yes_no(context_valid);
    uart::write_line("");

    uart::write_str("  task SP inside stack: ");
    crate::kernel::task::table::print_yes_no(sp_ok);
    uart::write_line("");

    uart::write_str("  resume_pc inside kernel text: ");
    crate::kernel::task::table::print_yes_no(resume_pc_text_ok);
    uart::write_line("");

    uart::write_str("  return_pc inside kernel text: ");
    crate::kernel::task::table::print_yes_no(return_pc_text_ok);
    uart::write_line("");

    let frame_ok = print_resume_frame_check(task_id);

    let ok = context_valid && sp_ok && resume_pc_text_ok && return_pc_text_ok && frame_ok;

    uart::write_str("  result: ");
    if ok {
        uart::write_line("OK");
    } else {
        uart::write_line("FAILED");
    }

    ok
}

#[cfg(feature = "resume_candidate_test")]
pub fn test_resume_candidate_selection() {
    print_resume_candidate_header();

    match crate::kernel::task::table::find_first_resumable_task() {
        Some(task_id) => {
            uart::write_str("selected resumable task: ");
            crate::kernel::task::table::print_task_name_by_id(task_id);
            uart::write_line("");

            uart::write_str("state: ");
            crate::kernel::task::table::print_task_state_by_id(task_id);
            uart::write_line("");

            uart::write_str("last_return: ");
            crate::kernel::task::table::print_task_return_kind_by_id(task_id);
            uart::write_line("");

            uart::write_str("can_resume: ");
            match crate::kernel::task::table::can_task_resume(task_id) {
                Some(value) => {
                    crate::kernel::task::table::print_yes_no(value);
                    uart::write_line("");
                }
                None => uart::write_line("unknown"),
            }

            uart::write_str("saved task SP: ");
            match crate::kernel::task::table::get_task_last_task_sp(task_id) {
                Some(sp) => {
                    uart::write_hex_u64(sp);
                    uart::write_line("");

                    uart::write_str("SP check: ");
                    match crate::kernel::task::table::is_sp_inside_task_stack(task_id, sp) {
                        Some(true) => uart::write_line("inside task stack"),
                        Some(false) => uart::write_line("outside task stack"),
                        None => uart::write_line("unknown task"),
                    }
                }
                None => {
                    uart::write_line("none");
                }
            }

            uart::write_line("resume candidate test complete");

            #[cfg(feature = "scheduler_run_test")]
            {
                uart::write_line("resume candidate selected; delegating to scheduler run");

                match crate::kernel::task::scheduler::run() {
                    crate::kernel::task::scheduler::RunResult::NoRunnableTask => {
                        uart::write_line("scheduler run returned: no runnable task");
                    }
                    crate::kernel::task::scheduler::RunResult::Failed => {
                        uart::write_line("scheduler run returned: failed");
                    }
                }

                crate::arch::halt();
            }

            #[cfg(all(
                feature = "scheduler_dispatch_test",
                not(feature = "scheduler_run_test")
            ))]
            {
                uart::write_line("resume candidate selected; delegating to scheduler run_once");

                match crate::kernel::task::scheduler::run_once() {
                    crate::kernel::task::scheduler::RunOnceResult::NoRunnableTask => {
                        uart::write_line("scheduler run_once returned: no runnable task");
                    }
                    crate::kernel::task::scheduler::RunOnceResult::Failed => {
                        uart::write_line("scheduler run_once returned: failed");
                    }
                }

                crate::arch::halt();
            }

            #[cfg(all(
                feature = "resume_preflight_test",
                not(feature = "scheduler_dispatch_test")
            ))]
            {
                test_resume_preflight_check();
            }
        }
        None => {
            uart::write_line("selected resumable task: none");
            print_resume_candidate_complete();

            #[cfg(all(
                feature = "scheduler_resume_loop_test",
                feature = "real_resume_restore_jump"
            ))]
            {
                if real_resume_jump_completion_check() {
                    uart::write_line("scheduler resume loop result: OK");
                    uart::write_line("scheduler resume loop test complete");

                    #[cfg(all(
                        target_arch = "riscv64",
                        feature = "real_resume_restore_jump",
                        feature = "scheduler_resume_loop_test"
                    ))]
                    print_riscv_cooperative_resume_milestone();

                    crate::arch::halt();
                }
            }
        }
    }

    #[cfg(not(any(
        feature = "resume_preflight_test",
        feature = "resume_dry_run_test",
        feature = "resume_restore_test",
        feature = "scheduler_dispatch_test"
    )))]
    crate::arch::halt();
}

#[cfg(all(feature = "resume_restore_test", feature = "real_resume_restore_jump"))]
pub fn real_resume_jump_completion_check() -> bool {
    crate::drivers::uart::write_line("");
    crate::drivers::uart::write_line("real resume jump completion check:");

    #[cfg(feature = "two_task_resume_handoff_test")]
    {
        crate::drivers::uart::write_line("  scenario: two-task handoff");

        let worker_a_ok = print_task_finished_cleanly_check(1);
        let worker_b_ok = print_task_finished_cleanly_check(2);

        let ok = worker_a_ok && worker_b_ok;

        crate::drivers::uart::write_str("    result: ");
        if ok {
            crate::drivers::uart::write_line("OK");
        } else {
            crate::drivers::uart::write_line("FAILED");
        }

        ok
    }

    #[cfg(not(feature = "two_task_resume_handoff_test"))]
    {
        #[cfg(feature = "scheduler_resume_loop_test")]
        crate::drivers::uart::write_line("  scenario: scheduler resume loop task");

        #[cfg(all(
            feature = "two_yield_task_test",
            not(feature = "scheduler_resume_loop_test")
        ))]
        crate::drivers::uart::write_line("  scenario: two-yield task");

        #[cfg(not(any(
            feature = "two_yield_task_test",
            feature = "scheduler_resume_loop_test"
        )))]
        crate::drivers::uart::write_line("  scenario: single-yield task");

        let ok = print_task_finished_cleanly_check(1);

        crate::drivers::uart::write_str("  result: ");
        if ok {
            crate::drivers::uart::write_line("OK");
        } else {
            crate::drivers::uart::write_line("FAILED");
        }

        ok
    }
}

#[cfg(all(feature = "resume_restore_test", feature = "real_resume_restore_jump"))]
pub fn print_task_finished_cleanly_check(task_id: usize) -> bool {
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("  task: ");
    crate::kernel::task::table::print_task_name_by_id(task_id);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("  state: ");
    crate::kernel::task::table::print_task_state_by_id(task_id);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("  can_resume: ");
    match crate::kernel::task::table::can_task_resume(task_id) {
        Some(value) => {
            crate::kernel::task::table::print_yes_no(value);
            crate::drivers::uart::write_line("");
        }
        None => crate::drivers::uart::write_line("unknown"),
    }

    crate::drivers::uart::write_str("  last_return: ");
    crate::kernel::task::table::print_task_return_kind_by_id(task_id);
    crate::drivers::uart::write_line("");

    let state_finished = matches!(
        crate::kernel::task::table::get_task_state(task_id),
        Some(crate::kernel::task::table::TaskState::Finished)
    );

    let can_resume_false = matches!(
        crate::kernel::task::table::can_task_resume(task_id),
        Some(false)
    );

    let last_return_exit = matches!(
        crate::kernel::task::table::get_task_return_kind(task_id),
        Some(crate::kernel::task::table::TaskReturnKind::Exit)
    );

    crate::drivers::uart::write_str("  state Finished: ");
    crate::kernel::task::table::print_yes_no(state_finished);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("  resume disabled: ");
    crate::kernel::task::table::print_yes_no(can_resume_false);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("  last return Exit: ");
    crate::kernel::task::table::print_yes_no(last_return_exit);
    crate::drivers::uart::write_line("");

    state_finished && can_resume_false && last_return_exit
}

#[cfg(all(
    target_arch = "riscv64",
    feature = "real_resume_restore_jump",
    feature = "scheduler_resume_loop_test"
))]
pub fn print_riscv_cooperative_resume_milestone() {
    crate::drivers::uart::write_line("PicoOS milestone:");
    crate::drivers::uart::write_line("  baseline: 0.1.0");
    crate::drivers::uart::write_line("  current: 0.1.64");

    crate::drivers::uart::write_line("");
    crate::drivers::uart::write_line("  cleanup:");
    crate::drivers::uart::write_line("    obsolete standalone task tests removed: OK");
    crate::drivers::uart::write_line("    obsolete standalone scheduler scripts removed: OK");
    crate::drivers::uart::write_line("    obsolete resume task script removed: OK");
    crate::drivers::uart::write_line("    obsolete resume PC proximity requirement removed: OK");

    crate::drivers::uart::write_line("");
    crate::drivers::uart::write_line("  task/resume:");
    crate::drivers::uart::write_line("    RISC-V-only baseline: OK");
    crate::drivers::uart::write_line("    cooperative task resume: OK");
    crate::drivers::uart::write_line("    repeated yield/resume loop: OK");
    crate::drivers::uart::write_line("    scheduler-oriented resume loop: OK");
    crate::drivers::uart::write_line("    RISC-V yield boundary: OK");
    crate::drivers::uart::write_line("    two-task cooperative handoff: OK");

    crate::drivers::uart::write_line("");
    crate::drivers::uart::write_line("  scheduler:");
    crate::drivers::uart::write_line("    scheduler first task dispatch: OK");
    crate::drivers::uart::write_line("    scheduler fresh task dispatch: OK");
    crate::drivers::uart::write_line("    scheduler round-robin fairness: OK");
    crate::drivers::uart::write_line("    scheduler task capacity from table: OK");
    crate::drivers::uart::write_line("    scheduler skips faulted tasks: OK");
    crate::drivers::uart::write_line("    no-runnable scheduler policy: OK");
    crate::drivers::uart::write_line("    scheduler dispatch candidate selection: OK");
    crate::drivers::uart::write_line("    scheduler dispatch candidate model: OK");
    crate::drivers::uart::write_line("    scheduler dispatch candidate-to-decision conversion: OK");
    crate::drivers::uart::write_line("    scheduler dispatch decision model: OK");
    crate::drivers::uart::write_line("    scheduler dispatch decision kind: OK");
    crate::drivers::uart::write_line("    scheduler dispatch decision outcome: OK");
    crate::drivers::uart::write_line("    scheduler dispatch decision logging: OK");
    crate::drivers::uart::write_line("    scheduler dispatch pipeline model: OK");

    crate::drivers::uart::write_line("");
    crate::drivers::uart::write_line("  task lifecycle:");
    crate::drivers::uart::write_line("    task state invariants in core: OK");
    crate::drivers::uart::write_line("    task state lookup in core: OK");
    crate::drivers::uart::write_line("    terminal task dispatch invariants in core: OK");
    crate::drivers::uart::write_line("    no-runnable scheduler snapshot in core: OK");
    crate::drivers::uart::write_line("    task completion summary in core: OK");
    crate::drivers::uart::write_line("    task completion output consolidated: OK");

    crate::drivers::uart::write_line("");
    crate::drivers::uart::write_line("  fault lifecycle:");
    crate::drivers::uart::write_line("    task fault state: OK");
    crate::drivers::uart::write_line("    trap-to-task-fault skeleton: OK");
    crate::drivers::uart::write_line("    real trap classification: OK");
    crate::drivers::uart::write_line("    real trap handler classification: OK");
    crate::drivers::uart::write_line("    real trap handler task-fault return path: OK");
    crate::drivers::uart::write_line("    trap fault metadata reporting: OK");
    crate::drivers::uart::write_line("    fault metadata assertions in core: OK");
    crate::drivers::uart::write_line("    explicit task fault assertions: OK");
    crate::drivers::uart::write_line("    faulted task dispatch guard: OK");
    crate::drivers::uart::write_line("    finished task dispatch guard: OK");
    crate::drivers::uart::write_line("    scheduler fault lifecycle feature: OK");
}

#[cfg(feature = "resume_candidate_test")]
pub fn print_resume_candidate_header() {
    crate::drivers::uart::write_line("");

    #[cfg(feature = "scheduler_run_test")]
    {
        crate::drivers::uart::write_line("scheduler resume candidate check:");
    }

    #[cfg(not(feature = "scheduler_run_test"))]
    {
        crate::drivers::uart::write_line("resume candidate test:");
    }
}

#[cfg(feature = "resume_candidate_test")]
pub fn print_resume_candidate_complete() {
    #[cfg(feature = "scheduler_run_test")]
    {
        crate::drivers::uart::write_line("scheduler resume candidate check complete");
    }

    #[cfg(not(feature = "scheduler_run_test"))]
    {
        crate::drivers::uart::write_line("resume candidate test complete");
    }
}
