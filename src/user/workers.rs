use crate::user::{
    u_sys_exit, u_sys_gettid, u_sys_join, u_sys_log, u_sys_recv, u_sys_send, u_sys_sleep,
    u_sys_spawn, u_sys_yield,
};

unsafe extern "C" {
    static __data_start: u8;
}

#[used]
static KERNEL_FETCH_PROBE_ADDR: extern "C" fn() = crate::kernel::sys::kernel_fetch_probe_target;

#[unsafe(no_mangle)]
#[unsafe(link_section = ".usertext")]
pub extern "C" fn worker_yield_main(_arg: u64) {
    u_sys_log(b"worker_yield: start\n");
    loop {
        u_sys_yield();
    }
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".usertext")]
pub extern "C" fn worker_sleep_main(_arg: u64) {
    u_sys_log(b"worker_sleep: start\n");
    loop {
        u_sys_sleep(1);
    }
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".usertext")]
pub extern "C" fn worker_pmp_deny(_arg: u64) {
    u_sys_log(b"pmp deny probe: store to .data\n");
    // SAFETY: U-mode PMP probe; an `sb` to `.data` must store-access-fault.
    // Inline so the store is fetched from `.usertext`, not `write_volatile` in
    // kernel `.text`.
    unsafe {
        core::arch::asm!(
            "sb {val}, 0({ptr})",
            ptr = in(reg) core::ptr::addr_of!(__data_start),
            val = in(reg) 0xDEu8,
            options(nostack)
        );
    }
    u_sys_log(b"pmp deny probe: FAILED\n");
    u_sys_exit();
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".usertext")]
pub extern "C" fn worker_two_yield(_arg: u64) {
    u_sys_log(b"two_yielding_task: step 1\n");
    u_sys_yield();
    u_sys_log(b"two_yielding_task: step 2\n");
    u_sys_yield();
    u_sys_log(b"two_yielding_task: step 3\n");
    u_sys_exit();
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".usertext")]
pub extern "C" fn worker_handoff_a(_arg: u64) {
    u_sys_log(b"handoff_worker_a: step 1\n");
    u_sys_yield();
    u_sys_log(b"handoff_worker_a: resumed after first yield\n");
    u_sys_yield();
    u_sys_log(b"handoff_worker_a: resumed after second yield\n");
    u_sys_exit();
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".usertext")]
pub extern "C" fn worker_handoff_b(_arg: u64) {
    u_sys_log(b"handoff_worker_b: step 1\n");
    u_sys_yield();
    u_sys_log(b"handoff_worker_b: resumed after yield\n");
    u_sys_exit();
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".usertext")]
pub extern "C" fn worker_clean_exit(_arg: u64) {
    u_sys_log(b"worker-a: exit\n");
    u_sys_exit();
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".usertext")]
pub extern "C" fn worker_ebreak(_arg: u64) {
    u_sys_log(b"trap-worker: ebreak\n");
    // SAFETY: `ebreak` is the intended U-mode fault for this worker.
    unsafe {
        core::arch::asm!("ebreak", options(nomem, nostack, preserves_flags));
    }
    u_sys_log(b"trap-worker: FAILED\n");
    u_sys_exit();
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".usertext")]
pub extern "C" fn worker_sleep_e2e(_arg: u64) {
    u_sys_log(b"sleeping_task_runtime_e2e: step 1\n");
    u_sys_sleep(2);
    u_sys_log(b"sleeping_task_runtime_e2e: resumed after timer wake\n");
    u_sys_exit();
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".usertext")]
pub extern "C" fn worker_kernel_fetch(_arg: u64) {
    u_sys_log(b"user text probe: jalr kernel .text\n");
    // SAFETY: `.rodata` holds the kernel `.text` probe address; U may read it.
    let target = unsafe {
        core::ptr::read_volatile(core::ptr::addr_of!(KERNEL_FETCH_PROBE_ADDR).cast::<u64>())
    };
    // SAFETY: `jalr` of a kernel `.text` address must instruction-access-fault.
    unsafe {
        core::arch::asm!("jalr {t}", t = in(reg) target, options(nostack));
    }
    u_sys_log(b"user text probe: FAILED\n");
    u_sys_exit();
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".usertext")]
pub extern "C" fn child_exit(_arg: u64) {
    u_sys_exit();
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".usertext")]
pub extern "C" fn worker_spawn_main(_arg: u64) {
    u_sys_log(b"worker_spawn: start\n");
    let _ = u_sys_gettid();
    let entry = child_exit as *const () as usize as u64;
    let tid = u_sys_spawn(entry, 0);
    if tid == u64::MAX {
        u_sys_log(b"default spawn join: FAILED\n");
        u_sys_exit();
    }
    let status = u_sys_join(tid);
    if status != 0 {
        u_sys_log(b"default spawn join: FAILED\n");
        u_sys_exit();
    }
    u_sys_exit();
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".usertext")]
pub extern "C" fn worker_ipc_recv(_arg: u64) {
    let mut buf = [0u8; 32];
    let (n, _sender) = u_sys_recv(&mut buf);
    if n != 32 {
        u_sys_log(b"ipc rendezvous: FAILED\n");
    }
    u_sys_exit();
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".usertext")]
pub extern "C" fn worker_ipc_send(arg: u64) {
    let buf = [0xA5u8; 32];
    let n = u_sys_send(arg, &buf);
    if n != 32 {
        u_sys_log(b"ipc rendezvous: FAILED\n");
    }
    u_sys_exit();
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".usertext")]
pub extern "C" fn worker_ipc_parent(_arg: u64) {
    let recv_entry = worker_ipc_recv as *const () as usize as u64;
    let send_entry = worker_ipc_send as *const () as usize as u64;
    let tid_b = u_sys_spawn(recv_entry, 0);
    if tid_b == u64::MAX {
        u_sys_log(b"ipc rendezvous: FAILED\n");
        u_sys_exit();
    }
    let tid_a = u_sys_spawn(send_entry, tid_b);
    if tid_a == u64::MAX {
        u_sys_log(b"ipc rendezvous: FAILED\n");
        u_sys_exit();
    }
    if u_sys_join(tid_a) != 0 || u_sys_join(tid_b) != 0 {
        u_sys_log(b"ipc rendezvous: FAILED\n");
    }
    u_sys_exit();
}

const _: &[extern "C" fn(u64)] = &[
    worker_yield_main,
    worker_sleep_main,
    worker_pmp_deny,
    worker_two_yield,
    worker_handoff_a,
    worker_handoff_b,
    worker_clean_exit,
    worker_ebreak,
    worker_sleep_e2e,
    worker_kernel_fetch,
    child_exit,
    worker_spawn_main,
    worker_ipc_recv,
    worker_ipc_send,
    worker_ipc_parent,
];
