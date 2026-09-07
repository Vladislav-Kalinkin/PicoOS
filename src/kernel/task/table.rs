use crate::drivers::uart;
use crate::kernel::hart_local::HartLocal;
use crate::kernel::memory::{self, PhysPage};
use crate::kernel::trap_frame::TrapImage;

pub const TASK_NAME_LEN: usize = 16;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TaskId(pub u64);

impl TaskId {
    pub const fn new(generation: u16, hart: u16, local: u16) -> Self {
        Self(((generation as u64) << 32) | ((hart as u64) << 16) | (local as u64))
    }

    pub const fn hart(self) -> u16 {
        (self.0 >> 16) as u16
    }

    pub const fn local(self) -> u16 {
        self.0 as u16
    }

    pub const fn is_valid(self) -> bool {
        self.0 != 0 && self.0 != u64::MAX
    }
}

pub type TaskEntry = extern "C" fn(u64);

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Empty,
    Ready,
    Running,
    Blocked,
    Finished,
    Faulted,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TaskReturnKind {
    None,
    Exit,
    Yield,
    Sleep,
    Fault,
    Join,
    Send,
    Recv,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BlockReason {
    SleepUntil(u64),
    Join(TaskId),
    Send { to: TaskId, len: u8 },
    Recv { ptr: u64, max: u64 },
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TaskFaultReason {
    Breakpoint,
    InstructionAccessFault,
    LoadAccessFault,
    StoreAccessFault,
    IllegalInstruction,
    Unknown(u64),
}

impl TaskFaultReason {
    pub const fn from_mcause(cause: u64) -> Self {
        match cause {
            1 => Self::InstructionAccessFault,
            2 => Self::IllegalInstruction,
            3 => Self::Breakpoint,
            5 => Self::LoadAccessFault,
            7 => Self::StoreAccessFault,
            other => Self::Unknown(other),
        }
    }
}

#[derive(Clone, Copy)]
pub struct Task {
    pub id: TaskId,
    pub state: TaskState,
    pub name: [u8; TASK_NAME_LEN],
    pub stack_start: u64,
    pub stack_top: u64,
    pub initial_sp: u64,
    pub initial_pc: u64,
    pub trap_image: Option<TrapImage>,
    pub has_started: bool,
    pub can_resume: bool,
    pub last_return_kind: TaskReturnKind,
    pub block: Option<BlockReason>,
    pub spawn_arg: u64,
    pub ipc_pending: [u8; 32],
    pub ipc_len: u8,
    next_free: Option<u16>,
}

impl Task {
    pub const fn empty() -> Self {
        Self {
            id: TaskId(0),
            state: TaskState::Empty,
            name: [0; TASK_NAME_LEN],
            stack_start: 0,
            stack_top: 0,
            initial_sp: 0,
            initial_pc: 0,
            trap_image: None,
            has_started: false,
            can_resume: false,
            last_return_kind: TaskReturnKind::None,
            block: None,
            spawn_arg: 0,
            ipc_pending: [0; 32],
            ipc_len: 0,
            next_free: None,
        }
    }
}

#[derive(Clone, Copy)]
struct HartTable {
    slots: *mut Task,
    cap: usize,
    used: usize,
    generation: u16,
    free_head: Option<u16>,
}

// SAFETY: Access to `slots` is strictly isolated to a single hart via `HartLocal` with interrupts disabled.
unsafe impl Send for HartTable {}

impl HartTable {
    const fn uninit() -> Self {
        Self {
            slots: core::ptr::null_mut(),
            cap: 0,
            used: 0,
            generation: 1,
            free_head: None,
        }
    }
}

static TABLE: HartLocal<HartTable> = HartLocal::new(HartTable::uninit());

pub fn init() {
    let hart = crate::arch::riscv64::cpu::mhartid() as u16;
    let local_free = memory::free_memory_current();
    let computed_cap = if local_free > 0 {
        let pages_in_partition = (local_free / memory::PAGE_SIZE) as usize;
        (pages_in_partition / 8).clamp(4, 512)
    } else {
        4
    };

    let bytes_needed = computed_cap * size_of::<Task>();
    let pages_needed = bytes_needed.div_ceil(memory::PAGE_SIZE as usize);

    let Some(page) = memory::alloc_pages(pages_needed) else {
        uart::write_line("mm: hart pool too small for table");
        crate::arch::halt();
    };

    let slots_ptr = page.addr() as *mut Task;

    TABLE.with(|table| {
        table.slots = slots_ptr;
        table.cap = computed_cap;
        table.used = 0;
        table.generation = 1;
        table.free_head = Some(0);

        for i in 0..computed_cap {
            // SAFETY: `slots_ptr` points to a valid, allocated page buffer.
            unsafe {
                let slot = slots_ptr.add(i);
                slot.write(Task::empty());
                (*slot).next_free = if i + 1 < computed_cap {
                    Some((i + 1) as u16)
                } else {
                    None
                };
            }
        }
    });

    if hart == 0 {
        uart::write_str("task table hart 0: cap ");
        uart::write_dec_u64(computed_cap as u64);
        uart::write_str(" pages ");
        uart::write_dec_u64(pages_needed as u64);
        uart::write_line("");
    }
}

fn with_slot<R>(local_idx: usize, f: impl FnOnce(&Task) -> R) -> Option<R> {
    TABLE.with(|table| {
        if local_idx >= table.cap || table.slots.is_null() {
            return None;
        }
        // SAFETY: access within the `0..table.cap` range of a local hart with IRQs disabled.
        unsafe {
            let slot = &*table.slots.add(local_idx);
            if slot.state == TaskState::Empty {
                None
            } else {
                Some(f(slot))
            }
        }
    })
}

fn with_slot_mut<R>(local_idx: usize, f: impl FnOnce(&mut Task) -> R) -> Option<R> {
    TABLE.with(|table| {
        if local_idx >= table.cap || table.slots.is_null() {
            return None;
        }
        // SAFETY: exclusive access to the slot on the current chart.
        unsafe {
            let slot = &mut *table.slots.add(local_idx);
            if slot.state == TaskState::Empty {
                None
            } else {
                Some(f(slot))
            }
        }
    })
}

pub fn spawn(name: &str, entry: TaskEntry, arg: u64) -> Option<TaskId> {
    spawn_inner(name, entry as *const () as usize as u64, arg)
}

pub(crate) fn spawn_user(entry_pc: u64, arg: u64) -> Option<TaskId> {
    let mut name = [0u8; 16];
    format_user_task_name(&mut name);
    let name_str = core::str::from_utf8(&name).unwrap_or("user");
    spawn_inner(name_str, entry_pc, arg)
}

fn format_user_task_name(dst: &mut [u8; 16]) {
    dst[0] = b'u';
    dst[1] = b'-';
    let hart = crate::arch::riscv64::cpu::mhartid() as u8;
    dst[2] = b'0' + (hart % 10);
    dst[3] = 0;
}

fn spawn_inner(name: &str, entry_pc: u64, arg: u64) -> Option<TaskId> {
    let hart = crate::arch::riscv64::cpu::mhartid() as u16;

    let (local_idx, task_id) = TABLE.with(|table| {
        let local = table.free_head?;
        let idx = local as usize;

        // SAFETY: `idx < table.cap` is guaranteed by the linked list of free slots.
        let slot = unsafe { &mut *table.slots.add(idx) };
        table.free_head = slot.next_free;
        table.used += 1;

        let id = TaskId::new(table.generation, hart, local);
        Some((idx, id))
    })?;

    let Some(stack_start) = memory::allocate_page() else {
        rollback_slot(local_idx);
        return None;
    };

    let stack_top = stack_start + memory::PAGE_SIZE;
    let mut printed_name = [0u8; TASK_NAME_LEN];
    copy_name(&mut printed_name, name);

    TABLE.with(|table| {
        // SAFETY: initialization of an occupied slot.
        unsafe {
            let slot = &mut *table.slots.add(local_idx);
            *slot = Task {
                id: task_id,
                state: TaskState::Ready,
                name: printed_name,
                stack_start,
                stack_top,
                initial_sp: stack_top,
                initial_pc: entry_pc,
                spawn_arg: arg,
                ..Task::empty()
            };
        }
    });

    Some(task_id)
}

fn rollback_slot(local_idx: usize) {
    TABLE.with(|table| {
        // SAFETY: return the slot to the intrusive list upon a stack memory allocation error.
        unsafe {
            let slot = &mut *table.slots.add(local_idx);
            slot.state = TaskState::Empty;
            slot.next_free = table.free_head;
            table.free_head = Some(local_idx as u16);
            table.used = table.used.saturating_sub(1);
        }
    });
}

pub fn destroy(id: TaskId) -> bool {
    let hart = crate::arch::riscv64::cpu::mhartid() as u16;
    if id.hart() != hart {
        return false;
    }

    let local_idx = id.local() as usize;

    let freed_stack = TABLE.with(|table| {
        if local_idx >= table.cap {
            return None;
        }

        // SAFETY: Check ownership and generation before deletion.
        unsafe {
            let slot = &mut *table.slots.add(local_idx);
            if slot.id != id || !matches!(slot.state, TaskState::Finished | TaskState::Faulted) {
                return None;
            }

            let stack = slot.stack_start;
            *slot = Task::empty();
            slot.next_free = table.free_head;
            table.free_head = Some(local_idx as u16);
            table.used = table.used.saturating_sub(1);

            table.generation = table.generation.wrapping_add(1);
            if table.generation == 0 {
                table.generation = 1;
            }

            Some(stack)
        }
    });

    if let Some(stack_addr) = freed_stack {
        if let Some(page) = PhysPage::new(stack_addr) {
            memory::free_pages(page, 1);
        }
        true
    } else {
        false
    }
}

pub fn find_next_dispatchable_after(current: Option<TaskId>) -> Option<TaskId> {
    TABLE.with(|table| {
        if table.cap == 0 || table.slots.is_null() {
            return None;
        }

        let start = current
            .filter(|id| id.hart() == crate::arch::riscv64::cpu::mhartid() as u16)
            .map_or(0, |id| (id.local() as usize + 1) % table.cap);

        for offset in 0..table.cap {
            let idx = (start + offset) % table.cap;
            // SAFETY: scanning the local table within the bounds of `cap`.
            unsafe {
                let slot = &*table.slots.add(idx);
                if is_dispatchable_task_ref(slot) {
                    return Some(slot.id);
                }
            }
        }
        None
    })
}

fn is_dispatchable_task_ref(task: &Task) -> bool {
    if task.state != TaskState::Ready {
        return false;
    }
    if !task.has_started && !task.can_resume {
        return true;
    }
    task.can_resume && is_resume_frame_safe(task)
}

fn is_resume_frame_safe(task: &Task) -> bool {
    let Some(ref image) = task.trap_image else {
        return false;
    };
    image.is_valid()
        && task.stack_start <= image.gpr.sp
        && image.gpr.sp <= task.stack_top
        && memory::is_inside_user_text(image.mepc)
}

pub fn wake_sleeping_tasks(current_tick: u64) -> usize {
    TABLE.with(|table| {
        let mut woke = 0;
        for idx in 0..table.cap {
            // SAFETY: update sleep state in the local table.
            unsafe {
                let slot = &mut *table.slots.add(idx);
                if slot.state != TaskState::Blocked {
                    continue;
                }
                if let Some(BlockReason::SleepUntil(wake_tick)) = slot.block
                    && current_tick >= wake_tick
                {
                    slot.state = TaskState::Ready;
                    slot.block = None;
                    let can_resume = is_resume_frame_safe(slot);
                    slot.can_resume = can_resume;
                    slot.last_return_kind = if can_resume {
                        TaskReturnKind::Sleep
                    } else {
                        TaskReturnKind::None
                    };
                    woke += 1;
                }
            }
        }
        woke
    })
}

pub fn save_preempted_trap_image(id: TaskId, image: &TrapImage) -> bool {
    with_slot_mut(id.local() as usize, |task| {
        if task.id != id {
            return false;
        }
        task.trap_image = Some(*image);
        task.state = TaskState::Ready;
        task.last_return_kind = TaskReturnKind::Yield;
        task.can_resume = true;
        task.block = None;
        true
    })
    .unwrap_or(false)
}

pub fn mark_task_running(id: TaskId) -> bool {
    with_slot_mut(id.local() as usize, |task| {
        if task.id == id && (task.state == TaskState::Ready || task.state == TaskState::Running) {
            task.state = TaskState::Running;
            true
        } else {
            false
        }
    })
    .unwrap_or(false)
}

pub fn mark_task_finished(id: TaskId) -> bool {
    with_slot_mut(id.local() as usize, |task| {
        if task.id != id {
            return false;
        }
        task.state = TaskState::Finished;
        task.last_return_kind = TaskReturnKind::Exit;
        task.can_resume = false;
        task.block = None;
        true
    })
    .unwrap_or(false)
}

pub fn mark_task_faulted(id: TaskId) -> bool {
    with_slot_mut(id.local() as usize, |task| {
        if task.id != id {
            return false;
        }
        task.state = TaskState::Faulted;
        task.last_return_kind = TaskReturnKind::Fault;
        task.can_resume = false;
        task.block = None;
        true
    })
    .unwrap_or(false)
}

pub fn record_task_fault(id: TaskId, mcause: u64) -> Option<TaskFaultReason> {
    if mark_task_faulted(id) {
        Some(TaskFaultReason::from_mcause(mcause))
    } else {
        None
    }
}

pub fn get_task_state(id: TaskId) -> Option<TaskState> {
    with_slot(id.local() as usize, |t| {
        if t.id == id { Some(t.state) } else { None }
    })
    .flatten()
}

pub fn get_task_stack_start(id: TaskId) -> Option<u64> {
    with_slot(id.local() as usize, |t| {
        if t.id == id {
            Some(t.stack_start)
        } else {
            None
        }
    })
    .flatten()
}

pub fn get_task_stack_top(id: TaskId) -> Option<u64> {
    with_slot(id.local() as usize, |t| {
        if t.id == id { Some(t.stack_top) } else { None }
    })
    .flatten()
}

pub fn get_task_initial_sp(id: TaskId) -> Option<u64> {
    with_slot(id.local() as usize, |t| {
        if t.id == id { Some(t.initial_sp) } else { None }
    })
    .flatten()
}

pub fn get_task_initial_pc(id: TaskId) -> Option<u64> {
    with_slot(id.local() as usize, |t| {
        if t.id == id { Some(t.initial_pc) } else { None }
    })
    .flatten()
}

pub fn get_task_spawn_arg(id: TaskId) -> Option<u64> {
    with_slot(id.local() as usize, |t| {
        if t.id == id { Some(t.spawn_arg) } else { None }
    })
    .flatten()
}

pub fn get_task_trap_image(id: TaskId) -> Option<TrapImage> {
    with_slot(id.local() as usize, |t| {
        if t.id == id { t.trap_image } else { None }
    })
    .flatten()
}

pub fn set_task_trap_image(id: TaskId, image: &TrapImage) -> bool {
    with_slot_mut(id.local() as usize, |t| {
        if t.id == id {
            t.trap_image = Some(*image);
            true
        } else {
            false
        }
    })
    .unwrap_or(false)
}

pub fn is_fresh_ready_task(id: TaskId) -> bool {
    with_slot(id.local() as usize, |t| {
        t.id == id && t.state == TaskState::Ready && !t.has_started && !t.can_resume
    })
    .unwrap_or(false)
}

pub fn mark_task_started(id: TaskId) -> bool {
    with_slot_mut(id.local() as usize, |t| {
        if t.id == id && (t.state == TaskState::Ready || t.state == TaskState::Running) {
            t.has_started = true;
            true
        } else {
            false
        }
    })
    .unwrap_or(false)
}

pub fn mark_task_blocked_until(id: TaskId, wake_tick: u64) -> bool {
    with_slot_mut(id.local() as usize, |t| {
        if t.id != id {
            return false;
        }
        t.state = TaskState::Blocked;
        t.last_return_kind = TaskReturnKind::Sleep;
        t.can_resume = false;
        t.block = Some(BlockReason::SleepUntil(wake_tick));
        true
    })
    .unwrap_or(false)
}

pub fn mark_task_blocked_join(id: TaskId, target: TaskId) -> bool {
    with_slot_mut(id.local() as usize, |t| {
        if t.id != id {
            return false;
        }
        t.state = TaskState::Blocked;
        t.last_return_kind = TaskReturnKind::Join;
        t.can_resume = false;
        t.block = Some(BlockReason::Join(target));
        true
    })
    .unwrap_or(false)
}

pub fn mark_task_blocked_send(id: TaskId, to: TaskId, len: u8) -> bool {
    with_slot_mut(id.local() as usize, |t| {
        if t.id != id {
            return false;
        }
        t.state = TaskState::Blocked;
        t.last_return_kind = TaskReturnKind::Send;
        t.can_resume = false;
        t.block = Some(BlockReason::Send { to, len });
        true
    })
    .unwrap_or(false)
}

pub fn mark_task_blocked_recv(id: TaskId, ptr: u64, max: u64) -> bool {
    with_slot_mut(id.local() as usize, |t| {
        if t.id != id {
            return false;
        }
        t.state = TaskState::Blocked;
        t.last_return_kind = TaskReturnKind::Recv;
        t.can_resume = false;
        t.block = Some(BlockReason::Recv { ptr, max });
        true
    })
    .unwrap_or(false)
}

pub fn join_status(id: TaskId) -> Option<u64> {
    match get_task_state(id)? {
        TaskState::Finished => Some(0),
        TaskState::Faulted => Some(1),
        _ => None,
    }
}

pub fn join_wake(target: TaskId) -> bool {
    let Some(status) = join_status(target) else {
        return false;
    };

    let joiner = TABLE.with(|table| {
        for idx in 0..table.cap {
            // SAFETY: search for a task blocked on join(target).
            unsafe {
                let slot = &*table.slots.add(idx);
                if slot.state == TaskState::Blocked && slot.block == Some(BlockReason::Join(target))
                {
                    return Some(slot.id);
                }
            }
        }
        None
    });

    let Some(joiner_id) = joiner else {
        return false;
    };

    let _ = with_slot_mut(joiner_id.local() as usize, |task| {
        if let Some(image) = task.trap_image.as_mut() {
            image.gpr.a0 = status;
        }
        task.state = TaskState::Ready;
        task.block = None;
        task.last_return_kind = TaskReturnKind::Join;
        task.can_resume = is_resume_frame_safe(task);
    });

    destroy(target)
}

pub fn ready_from_block(id: TaskId, a0: u64, a1: u64, kind: TaskReturnKind) -> bool {
    with_slot_mut(id.local() as usize, |task| {
        if task.id != id || task.state != TaskState::Blocked {
            return false;
        }
        if let Some(image) = task.trap_image.as_mut() {
            image.gpr.a0 = a0;
            image.gpr.a1 = a1;
        }
        task.state = TaskState::Ready;
        task.block = None;
        task.ipc_pending = [0; 32];
        task.ipc_len = 0;
        task.can_resume = is_resume_frame_safe(task);
        task.last_return_kind = if task.can_resume {
            kind
        } else {
            TaskReturnKind::None
        };
        true
    })
    .unwrap_or(false)
}

pub fn has_join_waiter(target: TaskId) -> bool {
    TABLE.with(|table| {
        for idx in 0..table.cap {
            // SAFETY: scanning the local table for pending items.
            unsafe {
                let slot = &*table.slots.add(idx);
                if slot.state == TaskState::Blocked && slot.block == Some(BlockReason::Join(target))
                {
                    return true;
                }
            }
        }
        false
    })
}

pub fn recv_buf(id: TaskId) -> Option<(u64, u64)> {
    with_slot(id.local() as usize, |t| {
        if t.id == id
            && t.state == TaskState::Blocked
            && let Some(BlockReason::Recv { ptr, max }) = t.block
        {
            return Some((ptr, max));
        }

        None
    })
    .flatten()
}

pub fn find_send_waiter_to(to: TaskId) -> Option<TaskId> {
    TABLE.with(|table| {
        for idx in 0..table.cap {
            // SAFETY: search for a local sender waiting for the `to` recipient.
            unsafe {
                let slot = &*table.slots.add(idx);
                if slot.state == TaskState::Blocked
                    && let Some(BlockReason::Send { to: dest, .. }) = slot.block
                    && dest == to
                {
                    return Some(slot.id);
                }
            }
        }
        None
    })
}

pub fn set_ipc_pending(id: TaskId, bytes: [u8; 32], len: u8) -> bool {
    with_slot_mut(id.local() as usize, |t| {
        if t.id == id {
            t.ipc_pending = bytes;
            t.ipc_len = len;
            true
        } else {
            false
        }
    })
    .unwrap_or(false)
}

pub fn take_ipc_pending(id: TaskId) -> Option<([u8; 32], u8)> {
    with_slot_mut(id.local() as usize, |t| {
        if t.id == id {
            let out = (t.ipc_pending, t.ipc_len);
            t.ipc_pending = [0; 32];
            t.ipc_len = 0;
            Some(out)
        } else {
            None
        }
    })
    .flatten()
}

pub fn ipc_pending_len(id: TaskId) -> Option<u8> {
    with_slot(id.local() as usize, |t| {
        if t.id == id { Some(t.ipc_len) } else { None }
    })
    .flatten()
}

pub fn has_potential_ipc_sender(recv_id: TaskId) -> bool {
    TABLE.with(|table| {
        for idx in 0..table.cap {
            // SAFETY: check local senders.
            unsafe {
                let slot = &*table.slots.add(idx);
                if slot.id == recv_id || slot.state == TaskState::Empty {
                    continue;
                }
                match slot.state {
                    TaskState::Ready | TaskState::Running => return true,
                    TaskState::Blocked => {
                        if let Some(BlockReason::Send { to, .. }) = slot.block
                            && to == recv_id
                        {
                            return true;
                        }
                    }
                    _ => {}
                }
            }
        }
        false
    })
}

pub fn stack_contains(id: TaskId, ptr: u64, len: u64) -> bool {
    let Some(end) = ptr.checked_add(len) else {
        return false;
    };
    with_slot(id.local() as usize, |task| {
        task.id == id && ptr >= task.stack_start && end <= task.stack_top
    })
    .unwrap_or(false)
}

pub fn has_dispatchable_tasks() -> bool {
    TABLE.with(|table| {
        for idx in 0..table.cap {
            // SAFETY: scanning Hart tasks.
            unsafe {
                if is_dispatchable_task_ref(&*table.slots.add(idx)) {
                    return true;
                }
            }
        }
        false
    })
}

fn copy_name(dst: &mut [u8; TASK_NAME_LEN], name: &str) {
    dst.fill(0);
    let bytes = name.as_bytes();
    let len = core::cmp::min(bytes.len(), TASK_NAME_LEN - 1);
    dst[..len].copy_from_slice(&bytes[..len]);
}

pub fn print_task_name_by_id(id: TaskId) {
    with_slot(id.local() as usize, |task| {
        if task.id == id {
            for &byte in &task.name {
                if byte == 0 {
                    break;
                }
                uart::putc(byte);
            }
        }
    });
}

pub fn print_task_fault_reason(reason: TaskFaultReason) {
    match reason {
        TaskFaultReason::Breakpoint => uart::write_str("breakpoint"),
        TaskFaultReason::InstructionAccessFault => uart::write_str("instruction access fault"),
        TaskFaultReason::LoadAccessFault => uart::write_str("load access fault"),
        TaskFaultReason::StoreAccessFault => uart::write_str("store access fault"),
        TaskFaultReason::IllegalInstruction => uart::write_str("illegal instruction"),
        TaskFaultReason::Unknown(code) => {
            uart::write_str("unknown (code: ");
            uart::write_hex_u64(code);
            uart::write_str(")");
        }
    }
}

pub fn print_tasks() {
    uart::write_line("task list:");
    TABLE.with(|table| {
        for idx in 0..table.cap {
            // SAFETY: Output diagnostic information about slots.
            unsafe {
                let slot = &*table.slots.add(idx);
                if slot.state == TaskState::Empty {
                    continue;
                }
                uart::write_str("id: ");
                uart::write_hex_u64(slot.id.0);
                uart::write_str(" state: Ready name: ");
                for &b in &slot.name {
                    if b == 0 {
                        break;
                    }
                    uart::putc(b);
                }
                uart::write_line("");
            }
        }
    });
}

pub fn print_yes_no(value: bool) {
    if value {
        uart::write_str("yes");
    } else {
        uart::write_str("no");
    }
}

pub fn can_task_resume(id: TaskId) -> Option<bool> {
    with_slot(id.local() as usize, |t| {
        if t.id == id { Some(t.can_resume) } else { None }
    })
    .flatten()
}

pub fn get_task_return_kind(id: TaskId) -> Option<TaskReturnKind> {
    with_slot(id.local() as usize, |t| {
        if t.id == id {
            Some(t.last_return_kind)
        } else {
            None
        }
    })
    .flatten()
}

pub fn find_first_resumable_task() -> Option<TaskId> {
    TABLE.with(|table| {
        for idx in 0..table.cap {
            // SAFETY: search for the first task ready to resume within the capacity of the local table.
            unsafe {
                let slot = &*table.slots.add(idx);
                if slot.state == TaskState::Ready && slot.can_resume && is_resume_frame_safe(slot) {
                    return Some(slot.id);
                }
            }
        }
        None
    })
}

pub fn table_capacity() -> usize {
    TABLE.with(|table| table.cap)
}

pub fn get_task_by_local_index(local: usize) -> Option<Task> {
    TABLE.with(|table| {
        if local >= table.cap || table.slots.is_null() {
            return None;
        }
        // SAFETY: direct slot read by local index within the hash map's capacity.
        unsafe {
            let slot = &*table.slots.add(local);
            if slot.state == TaskState::Empty {
                None
            } else {
                Some(*slot)
            }
        }
    })
}
