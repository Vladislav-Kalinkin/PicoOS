use crate::drivers::uart;
use crate::kernel::cpu;
use crate::kernel::memory;
use crate::kernel::trap_frame::{Riscv64TrapFrame, TrapImage};

pub const SYS_YIELD: u64 = 0;
pub const SYS_SLEEP: u64 = 1;
pub const SYS_EXIT: u64 = 2;
pub const SYS_LOG: u64 = 3;

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn kernel_fetch_probe_target() {
    crate::arch::halt();
}

pub fn handle_ecall(frame: &Riscv64TrapFrame) {
    match frame.a7 {
        SYS_YIELD => sys_yield(frame),
        SYS_SLEEP => sys_sleep(frame, frame.a0),
        SYS_EXIT => sys_exit(),
        SYS_LOG => sys_log(frame.a0, frame.a1),
        _ => illegal_syscall(),
    }
}

fn trap_image_after_ecall(frame: &Riscv64TrapFrame) -> TrapImage {
    TrapImage::from_frame(
        frame,
        crate::arch::riscv64::cpu::mepc().wrapping_add(4),
        crate::arch::riscv64::cpu::mstatus(),
    )
}

fn sys_yield(frame: &Riscv64TrapFrame) -> ! {
    let Some(id) = cpu::current() else {
        crate::kernel::log::fail("sys", "yield with no current task");
        crate::arch::halt();
    };

    let image = trap_image_after_ecall(frame);
    let _ = crate::kernel::task::table::save_preempted_trap_image(id, &image);
    crate::kernel::task::scheduler::note_default_image_return(
        crate::kernel::task::table::TaskReturnKind::Yield,
    );

    crate::kernel::task::scheduler::switch_after(Some(id));
}

fn sys_sleep(frame: &Riscv64TrapFrame, ticks: u64) -> ! {
    let Some(id) = cpu::current() else {
        crate::kernel::log::fail("sys", "sleep with no current task");
        crate::arch::halt();
    };

    let image = trap_image_after_ecall(frame);
    let _ = crate::kernel::task::table::set_task_trap_image(id, &image);

    let wake_tick = crate::kernel::ticks::get().saturating_add(ticks.max(1));
    if !crate::kernel::task::table::mark_task_blocked_until(id, wake_tick) {
        crate::kernel::log::fail("sys", "sleep mark blocked failed");
        crate::arch::halt();
    }

    crate::kernel::task::scheduler::note_default_image_return(
        crate::kernel::task::table::TaskReturnKind::Sleep,
    );

    crate::kernel::task::scheduler::switch_after(Some(id));
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
    crate::kernel::task::scheduler::switch_after(Some(id));
}

fn sys_log(ptr: u64, len: u64) {
    if !user_buffer_ok(ptr, len) {
        illegal_syscall();
    }

    // SAFETY: `user_buffer_ok` accepted this range as the current worker
    // stack or kernel `.rodata`.
    let bytes = unsafe { core::slice::from_raw_parts(ptr as *const u8, len as usize) };
    for &byte in bytes {
        uart::putc(byte);
    }

    let mepc = crate::arch::riscv64::cpu::mepc().wrapping_add(4);
    crate::arch::riscv64::cpu::set_mepc(mepc);
    crate::arch::riscv64::cpu::set_mstatus(
        crate::arch::riscv64::cpu::synthesize_mstatus_for_mret_worker(),
    );
}

fn illegal_syscall() -> ! {
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
