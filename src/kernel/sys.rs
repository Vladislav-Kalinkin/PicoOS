use crate::drivers::uart;
use crate::kernel::cpu;
use crate::kernel::memory;
use crate::kernel::trap_frame::{Riscv64TrapFrame, TrapImage};

pub const SYS_YIELD: u64 = 0;
pub const SYS_SLEEP: u64 = 1;
pub const SYS_EXIT: u64 = 2;
pub const SYS_LOG: u64 = 3;

macro_rules! ecall {
    ($a7:expr) => {
        core::arch::asm!(
            ".option push",
            ".option norvc",
            "li a7, {nr}",
            "ecall",
            ".option pop",
            nr = const $a7,
            lateout("a7") _,
            options(nomem, nostack)
        );
    };
}

/// U-mode stub: `ecall` only. Worker/trampoline Rust may call this, not UART.
pub fn u_sys_yield() {
    unsafe {
        ecall!(SYS_YIELD);
    }
}

#[allow(dead_code)]
pub fn u_sys_sleep(ticks: u64) {
    unsafe {
        core::arch::asm!(
            ".option push",
            ".option norvc",
            "li a7, {nr}",
            "ecall",
            ".option pop",
            nr = const SYS_SLEEP,
            in("a0") ticks,
            lateout("a7") _,
            options(nomem, nostack)
        );
    }
}

pub fn u_sys_exit() -> ! {
    unsafe {
        ecall!(SYS_EXIT);
        crate::arch::halt();
    }
}

pub fn u_sys_log(bytes: &[u8]) {
    unsafe {
        core::arch::asm!(
            ".option push",
            ".option norvc",
            "li a7, {nr}",
            "ecall",
            ".option pop",
            nr = const SYS_LOG,
            in("a0") bytes.as_ptr(),
            in("a1") bytes.len(),
            lateout("a7") _,
            options(nostack)
        );
    }
}

pub fn handle_ecall(frame: *const Riscv64TrapFrame) {
    let (a7, a0, a1) = unsafe { ((*frame).a7, (*frame).a0, (*frame).a1) };

    match a7 {
        SYS_YIELD => sys_yield(frame),
        SYS_SLEEP => sys_sleep(frame, a0),
        SYS_EXIT => sys_exit(),
        SYS_LOG => sys_log(frame, a0, a1),
        _ => illegal_syscall(frame),
    }
}

fn sys_yield(frame: *const Riscv64TrapFrame) -> ! {
    let Some(id) = cpu::current() else {
        crate::kernel::log::fail("sys", "yield with no current task");
        crate::arch::halt();
    };

    let mepc = crate::arch::riscv64::cpu::mepc().wrapping_add(4);
    let image = unsafe {
        TrapImage::from_frame(
            &*frame,
            mepc,
            crate::arch::riscv64::cpu::mstatus(),
        )
    };
    let _ = crate::kernel::task::table::save_preempted_trap_image(id, &image);
    crate::kernel::task::scheduler::note_default_image_return(
        crate::kernel::task::table::TaskReturnKind::Yield,
    );

    switch_after_syscall(Some(id));
}

fn sys_sleep(frame: *const Riscv64TrapFrame, ticks: u64) -> ! {
    let Some(id) = cpu::current() else {
        crate::kernel::log::fail("sys", "sleep with no current task");
        crate::arch::halt();
    };

    let mepc = crate::arch::riscv64::cpu::mepc().wrapping_add(4);
    let image = unsafe {
        TrapImage::from_frame(
            &*frame,
            mepc,
            crate::arch::riscv64::cpu::mstatus(),
        )
    };
    let _ = crate::kernel::task::table::set_task_trap_image(id, &image);
    let cpu_context = image.to_yield_context();
    let _ = crate::kernel::task::table::set_task_cpu_context(id, cpu_context);
    let _ = crate::kernel::task::table::set_task_last_return_context(id, image.gpr.sp, 0, mepc);

    let wake_tick = crate::kernel::ticks::get().saturating_add(ticks.max(1));
    if !crate::kernel::task::table::mark_task_blocked_until(id, wake_tick) {
        crate::kernel::log::fail("sys", "sleep mark blocked failed");
        crate::arch::halt();
    }

    crate::kernel::task::scheduler::note_default_image_return(
        crate::kernel::task::table::TaskReturnKind::Sleep,
    );

    switch_after_syscall(Some(id));
}

fn sys_exit() -> ! {
    let Some(id) = cpu::current() else {
        crate::kernel::log::fail("sys", "exit with no current task");
        crate::arch::halt();
    };

    let _ = crate::kernel::task::table::mark_task_finished(id);
    crate::kernel::task::scheduler::note_default_image_return(
        crate::kernel::task::table::TaskReturnKind::Exit,
    );
    cpu::clear_current();
    switch_after_syscall(Some(id));
}

fn sys_log(frame: *const Riscv64TrapFrame, ptr: u64, len: u64) {
    if !user_buffer_ok(ptr, len) {
        illegal_syscall(frame);
    }

    let bytes = unsafe { core::slice::from_raw_parts(ptr as *const u8, len as usize) };
    for &byte in bytes {
        uart::putc(byte);
    }

    let mepc = crate::arch::riscv64::cpu::mepc().wrapping_add(4);
    crate::arch::riscv64::cpu::set_mepc(mepc);
    crate::arch::riscv64::cpu::set_mstatus(
        crate::arch::riscv64::cpu::synthesize_mstatus_for_mret_worker(),
    );
    crate::kernel::cpu::set_in_trap(false);
}

fn illegal_syscall(_frame: *const Riscv64TrapFrame) -> ! {
    crate::kernel::log::fail("sys", "illegal ecall");
    crate::kernel::task::fault::record_and_switch_user_fault(
        crate::arch::riscv64::cpu::mcause(),
        crate::arch::riscv64::cpu::mepc(),
        crate::arch::riscv64::cpu::mtval(),
    );
}

fn user_buffer_ok(ptr: u64, len: u64) -> bool {
    let Some(end) = ptr.checked_add(len) else {
        return false;
    };

    let stack_start = cpu::current_stack_start();
    let stack_top = cpu::current_stack_top();
    if ptr >= stack_start && end <= stack_top && stack_top > stack_start {
        return true;
    }

    ptr >= memory::rodata_start() && end <= memory::rodata_end()
}

pub fn switch_after_syscall(after: Option<usize>) -> ! {
    let next = crate::kernel::task::scheduler::prepare_timer_switch(after);
    if let Some(id) = after
        && crate::kernel::task::table::is_terminal_task(id)
    {
        let _ = crate::kernel::task::table::destroy(id);
    }

    match next {
        Some(id) => {
            let fresh = crate::kernel::task::table::is_fresh_ready_task(id);
            let Some(image) = (if fresh {
                crate::kernel::task::scheduler::build_fresh_trap_image(id)
            } else {
                crate::kernel::task::scheduler::trap_image_for_resume(id)
            }) else {
                crate::arch::idle_exit_from_trap();
            };
            crate::kernel::task::scheduler::arm_worker_for_mret(id, fresh);
            crate::arch::mret_to_trap_image(&image);
        }
        None => crate::arch::idle_exit_from_trap(),
    }
}
