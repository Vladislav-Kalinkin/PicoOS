use crate::drivers::uart;
use crate::kernel::cpu;
use crate::kernel::memory;
use crate::kernel::trap_frame::{Riscv64TrapFrame, TrapImage};

pub const SYS_YIELD: u64 = 0;
pub const SYS_SLEEP: u64 = 1;
pub const SYS_EXIT: u64 = 2;
pub const SYS_LOG: u64 = 3;
pub const SYS_SPAWN: u64 = 4;
pub const SYS_JOIN: u64 = 5;
pub const SYS_SEND: u64 = 6;
pub const SYS_RECV: u64 = 7;
pub const SYS_GETTID: u64 = 8;

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn kernel_fetch_probe_target() {
    crate::arch::halt();
}

pub fn handle_ecall(frame: &mut Riscv64TrapFrame) {
    match frame.a7 {
        SYS_YIELD => sys_yield(frame),
        SYS_SLEEP => sys_sleep(frame, frame.a0),
        SYS_EXIT => sys_exit(),
        SYS_LOG => sys_log(frame),
        SYS_SPAWN => sys_spawn(frame),
        SYS_JOIN => sys_join(frame),
        SYS_SEND => crate::kernel::ipc::sys_send(frame),
        SYS_RECV => crate::kernel::ipc::sys_recv(frame),
        SYS_GETTID => sys_gettid(frame),
        _ => illegal_syscall(),
    }
}

pub(crate) fn trap_image_after_ecall(frame: &Riscv64TrapFrame) -> TrapImage {
    TrapImage::from_frame(
        frame,
        crate::arch::riscv64::cpu::mepc().wrapping_add(4),
        crate::arch::riscv64::cpu::mstatus(),
    )
}

fn advance_ecall_pc() {
    let mepc = crate::arch::riscv64::cpu::mepc().wrapping_add(4);
    crate::arch::riscv64::cpu::set_mepc(mepc);
    crate::arch::riscv64::cpu::set_mstatus(
        crate::arch::riscv64::cpu::synthesize_mstatus_for_mret_worker(),
    );
}

pub(crate) fn same_frame_return_a0(frame: &mut Riscv64TrapFrame, a0: u64) {
    frame.a0 = a0;
    advance_ecall_pc();
}

pub(crate) fn same_frame_return_a0_a1(frame: &mut Riscv64TrapFrame, a0: u64, a1: u64) {
    frame.a0 = a0;
    frame.a1 = a1;
    advance_ecall_pc();
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
    crate::kernel::ipc::on_peer_exit(id);
    cpu::clear_current();
    crate::kernel::task::scheduler::switch_after(Some(id));
}

fn sys_log(frame: &Riscv64TrapFrame) {
    if !user_buffer_ok(frame.a0, frame.a1) {
        illegal_syscall();
    }

    // SAFETY: `user_buffer_ok` accepted this range as the current worker
    // stack or kernel `.rodata`.
    let bytes = unsafe { core::slice::from_raw_parts(frame.a0 as *const u8, frame.a1 as usize) };
    for &byte in bytes {
        uart::putc(byte);
    }

    advance_ecall_pc();
}

fn sys_spawn(frame: &mut Riscv64TrapFrame) {
    let entry_pc = frame.a0;
    let arg = frame.a1;

    if !memory::is_inside_user_text(entry_pc) {
        illegal_syscall();
    }

    let tid =
        crate::kernel::task::table::spawn_user(entry_pc, arg).map_or(u64::MAX, |id| id as u64);
    same_frame_return_a0(frame, tid);
}

fn sys_join(frame: &mut Riscv64TrapFrame) {
    let Some(self_id) = cpu::current() else {
        crate::kernel::log::fail("sys", "join with no current task");
        crate::arch::halt();
    };

    let target = frame.a0 as usize;
    if target == self_id {
        illegal_syscall();
    }

    if let Some(status) = crate::kernel::task::table::join_status(target) {
        let _ = crate::kernel::task::table::destroy(target);
        crate::kernel::task::scheduler::note_join_reap();
        same_frame_return_a0(frame, status);
        return;
    }

    if crate::kernel::task::table::get_task_state(target).is_none() {
        same_frame_return_a0(frame, u64::MAX);
        return;
    }

    if crate::kernel::task::table::has_join_waiter(target) {
        same_frame_return_a0(frame, u64::MAX);
        return;
    }

    let image = trap_image_after_ecall(frame);
    let _ = crate::kernel::task::table::set_task_trap_image(self_id, &image);
    if !crate::kernel::task::table::mark_task_blocked_join(self_id, target) {
        crate::kernel::log::fail("sys", "join mark blocked failed");
        crate::arch::halt();
    }

    crate::kernel::task::scheduler::switch_after(Some(self_id));
}

fn sys_gettid(frame: &mut Riscv64TrapFrame) {
    let Some(id) = cpu::current() else {
        crate::kernel::log::fail("sys", "gettid with no current task");
        crate::arch::halt();
    };

    same_frame_return_a0(frame, id as u64);
}

pub(crate) fn illegal_syscall() -> ! {
    crate::kernel::log::fail("sys", "illegal ecall");
    crate::kernel::task::fault::record_and_switch_user_fault(
        crate::arch::riscv64::cpu::mcause(),
        crate::arch::riscv64::cpu::mepc(),
        crate::arch::riscv64::cpu::mtval(),
    );
}

pub(crate) fn user_stack_buffer_ok(ptr: u64, len: u64) -> bool {
    let Some(end) = ptr.checked_add(len) else {
        return false;
    };

    let stack_start = cpu::current_stack_start();
    let stack_top = cpu::current_stack_top();
    ptr >= stack_start && end <= stack_top && stack_top > stack_start
}

fn user_buffer_ok(ptr: u64, len: u64) -> bool {
    if user_stack_buffer_ok(ptr, len) {
        return true;
    }

    let Some(end) = ptr.checked_add(len) else {
        return false;
    };
    ptr >= memory::rodata_start() && end <= memory::rodata_end()
}
