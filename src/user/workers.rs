use crate::user::{u_sys_exit, u_sys_log, u_sys_sleep, u_sys_yield};

unsafe extern "C" {
    static __data_start: u8;
}

#[used]
static KERNEL_FETCH_PROBE_ADDR: extern "C" fn() = crate::kernel::sys::kernel_fetch_probe_target;

#[unsafe(no_mangle)]
#[unsafe(link_section = ".usertext")]
pub extern "Rust" fn worker_yield_main() {
    u_sys_log(b"worker_yield: start\n");
    loop {
        u_sys_yield();
    }
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".usertext")]
pub extern "Rust" fn worker_sleep_main() {
    u_sys_log(b"worker_sleep: start\n");
    loop {
        u_sys_sleep(1);
    }
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".usertext")]
pub extern "Rust" fn worker_pmp_deny() {
    u_sys_log(b"pmp deny probe: store to .data\n");
    // SAFETY: U-mode PMP probe; a store to `.data` must trap.
    unsafe {
        core::ptr::write_volatile(core::ptr::addr_of!(__data_start).cast_mut(), 0xDEu8);
    }
    u_sys_log(b"pmp deny probe: FAILED\n");
    u_sys_exit();
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".usertext")]
pub extern "Rust" fn worker_two_yield() {
    u_sys_log(b"two_yielding_task: step 1\n");
    u_sys_yield();
    u_sys_log(b"two_yielding_task: step 2\n");
    u_sys_yield();
    u_sys_log(b"two_yielding_task: step 3\n");
    u_sys_exit();
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".usertext")]
pub extern "Rust" fn worker_handoff_a() {
    u_sys_log(b"handoff_worker_a: step 1\n");
    u_sys_yield();
    u_sys_log(b"handoff_worker_a: resumed after first yield\n");
    u_sys_yield();
    u_sys_log(b"handoff_worker_a: resumed after second yield\n");
    u_sys_exit();
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".usertext")]
pub extern "Rust" fn worker_handoff_b() {
    u_sys_log(b"handoff_worker_b: step 1\n");
    u_sys_yield();
    u_sys_log(b"handoff_worker_b: resumed after yield\n");
    u_sys_exit();
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".usertext")]
pub extern "Rust" fn worker_clean_exit() {
    u_sys_log(b"worker-a: exit\n");
    u_sys_exit();
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".usertext")]
pub extern "Rust" fn worker_ebreak() {
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
pub extern "Rust" fn worker_sleep_e2e() {
    u_sys_log(b"sleeping_task_runtime_e2e: step 1\n");
    u_sys_sleep(2);
    u_sys_log(b"sleeping_task_runtime_e2e: resumed after timer wake\n");
    u_sys_exit();
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".usertext")]
pub extern "Rust" fn worker_kernel_fetch() {
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

const _: &[fn()] = &[
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
];
