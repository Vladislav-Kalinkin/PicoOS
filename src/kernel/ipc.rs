use crate::drivers::uart;
use crate::kernel::cpu;
use crate::kernel::sys;
use crate::kernel::task::table::{self, BlockReason, TaskId, TaskReturnKind, TaskState};
use crate::kernel::trap_frame::Riscv64TrapFrame;
use core::sync::atomic::{AtomicBool, Ordering};

pub const IPC_PAYLOAD_MAX: u64 = 32;

static RENDEZVOUS_PRINTED: AtomicBool = AtomicBool::new(false);

pub fn sys_send(frame: &mut Riscv64TrapFrame) {
    let Some(self_id) = cpu::current() else {
        crate::kernel::log::fail("ipc", "send with no current task");
        crate::arch::halt();
    };

    let dest = TaskId(frame.a0);
    let ptr = frame.a1;
    let len = frame.a2;

    if dest == self_id || len == 0 || len > IPC_PAYLOAD_MAX || !dest.is_valid() {
        sys::illegal_syscall();
    }
    if !sys::user_stack_buffer_ok(ptr, len) {
        sys::illegal_syscall();
    }

    let len_u8 = len as u8;
    match table::get_task_state(dest) {
        None | Some(TaskState::Empty | TaskState::Finished | TaskState::Faulted) => {
            sys::same_frame_return_a0(frame, u64::MAX);
        }
        Some(_) => {
            if table::recv_buf(dest).is_some() {
                complete_send(frame, self_id, dest, ptr, len_u8);
            } else {
                block_send(frame, self_id, dest, ptr, len_u8);
            }
        }
    }
}

pub fn sys_recv(frame: &mut Riscv64TrapFrame) {
    let Some(self_id) = cpu::current() else {
        crate::kernel::log::fail("ipc", "recv with no current task");
        crate::arch::halt();
    };

    let ptr = frame.a0;
    let max = frame.a1;
    if !sys::user_stack_buffer_ok(ptr, max) {
        sys::illegal_syscall();
    }

    loop {
        if let Some(sender) = table::find_send_waiter_to(self_id) {
            if try_complete_recv(frame, self_id, sender, ptr, max) {
                return;
            }
            continue;
        }
        if table::has_potential_ipc_sender(self_id) {
            block_recv(frame, self_id, ptr, max);
        }
        sys::same_frame_return_a0_a1(frame, u64::MAX, u64::MAX);
        return;
    }
}

pub fn on_peer_exit(target: TaskId) {
    wake_senders_to(target);
    if table::join_wake(target) {
        crate::kernel::task::scheduler::note_join_reap();
    }
    wake_stranded_recvs();
}

fn complete_send(frame: &mut Riscv64TrapFrame, self_id: TaskId, dest: TaskId, ptr: u64, len: u8) {
    let Some((recv_ptr, max)) = table::recv_buf(dest) else {
        sys::same_frame_return_a0(frame, u64::MAX);
        return;
    };
    let n = u64::from(len);
    if n > max {
        sys::illegal_syscall();
    }
    if !table::stack_contains(dest, recv_ptr, n) {
        sys::illegal_syscall();
    }

    copy_user(ptr, recv_ptr, n as usize);
    note_rendezvous();
    let _ = table::ready_from_block(dest, n, self_id.0, TaskReturnKind::Recv);
    sys::same_frame_return_a0(frame, n);
}

fn block_send(frame: &Riscv64TrapFrame, self_id: TaskId, dest: TaskId, ptr: u64, len: u8) {
    store_pending(self_id, ptr, len);
    let image = sys::trap_image_after_ecall(frame);
    let _ = table::set_task_trap_image(self_id, &image);
    if !table::mark_task_blocked_send(self_id, dest, len) {
        crate::kernel::log::fail("ipc", "send mark blocked failed");
        crate::arch::halt();
    }
    crate::kernel::task::scheduler::switch_after(Some(self_id));
}

fn try_complete_recv(
    frame: &mut Riscv64TrapFrame,
    self_id: TaskId,
    sender: TaskId,
    ptr: u64,
    max: u64,
) -> bool {
    let Some(n) = table::ipc_pending_len(sender) else {
        return true;
    };
    let n64 = u64::from(n);
    if n64 > max {
        let _ = table::mark_task_faulted(sender);
        on_peer_exit(sender);
        return false;
    }
    if !table::stack_contains(self_id, ptr, n64) {
        sys::illegal_syscall();
    }

    let Some((bytes, _)) = table::take_ipc_pending(sender) else {
        return true;
    };
    copy_kernel_to_user(&bytes, ptr, n as usize);
    note_rendezvous();
    let _ = table::ready_from_block(sender, n64, 0, TaskReturnKind::Send);
    sys::same_frame_return_a0_a1(frame, n64, sender.0);
    true
}

fn block_recv(frame: &Riscv64TrapFrame, self_id: TaskId, ptr: u64, max: u64) -> ! {
    let image = sys::trap_image_after_ecall(frame);
    let _ = table::set_task_trap_image(self_id, &image);
    if !table::mark_task_blocked_recv(self_id, ptr, max) {
        crate::kernel::log::fail("ipc", "recv mark blocked failed");
        crate::arch::halt();
    }
    crate::kernel::task::scheduler::switch_after(Some(self_id));
}

fn wake_senders_to(target: TaskId) {
    let cap = table::table_capacity();
    for local in 0..cap {
        let Some(t) = table::get_task_by_local_index(local) else {
            continue;
        };
        if t.state == TaskState::Blocked
            && let Some(BlockReason::Send { to, .. }) = t.block
            && to == target
        {
            let _ = table::ready_from_block(t.id, u64::MAX, 0, TaskReturnKind::Send);
        }
    }
}

fn wake_stranded_recvs() {
    let cap = table::table_capacity();
    for local in 0..cap {
        let Some(t) = table::get_task_by_local_index(local) else {
            continue;
        };
        if t.state == TaskState::Blocked
            && table::recv_buf(t.id).is_some()
            && !table::has_potential_ipc_sender(t.id)
        {
            let _ = table::ready_from_block(t.id, u64::MAX, u64::MAX, TaskReturnKind::Recv);
        }
    }
}

fn store_pending(id: TaskId, ptr: u64, len: u8) {
    let mut bytes = [0u8; 32];
    copy_user_to_kernel(ptr, &mut bytes, len as usize);
    let _ = table::set_ipc_pending(id, bytes, len);
}

fn copy_user(src: u64, dst: u64, len: usize) {
    // SAFETY: The caller has verified that `src` and `dst` are within the frame stacks.
    unsafe {
        let src_slice = core::slice::from_raw_parts(src as *const u8, len);
        let dst_slice = core::slice::from_raw_parts_mut(dst as *mut u8, len);
        dst_slice.copy_from_slice(src_slice);
    }
}

fn copy_user_to_kernel(src: u64, dst: &mut [u8; 32], len: usize) {
    // SAFETY: The caller has confirmed the validity of the user-stack buffer.
    unsafe {
        let src_slice = core::slice::from_raw_parts(src as *const u8, len);
        dst[..len].copy_from_slice(src_slice);
    }
}

fn copy_kernel_to_user(src: &[u8; 32], dst: u64, len: usize) {
    // SAFETY: The destination address has been verified and resides on the receiving worker's stack.
    unsafe {
        let dst_slice = core::slice::from_raw_parts_mut(dst as *mut u8, len);
        dst_slice.copy_from_slice(&src[..len]);
    }
}

fn note_rendezvous() {
    let already = RENDEZVOUS_PRINTED.swap(true, Ordering::AcqRel);
    if !already {
        uart::write_line("ipc rendezvous: OK");
    }
}
