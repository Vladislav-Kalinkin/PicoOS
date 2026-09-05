use crate::arch::riscv64::ecall::{u_ecall_0, u_ecall_a0, u_ecall_a0a1, u_ecall_a0a1a2, u_ecall_recv};
use crate::kernel::sys::{
    SYS_EXIT, SYS_GETTID, SYS_JOIN, SYS_LOG, SYS_RECV, SYS_SEND, SYS_SLEEP, SYS_SPAWN, SYS_YIELD,
};

/// U-mode stub: `ecall` only. Worker/trampoline Rust may call this, not UART.
#[unsafe(no_mangle)]
#[unsafe(link_section = ".usertext")]
pub extern "Rust" fn u_sys_yield() {
    let _ = u_ecall_0(SYS_YIELD);
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".usertext")]
pub extern "Rust" fn u_sys_sleep(ticks: u64) {
    let _ = u_ecall_a0(SYS_SLEEP, ticks);
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".usertext")]
pub extern "Rust" fn u_sys_exit() -> ! {
    let _ = u_ecall_0(SYS_EXIT);
    loop {
        core::hint::spin_loop();
    }
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".usertext")]
pub extern "Rust" fn u_sys_log(bytes: &[u8]) {
    let _ = u_ecall_a0a1(SYS_LOG, bytes.as_ptr() as u64, bytes.len() as u64);
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".usertext")]
pub extern "Rust" fn u_sys_spawn(entry: u64, arg: u64) -> u64 {
    u_ecall_a0a1(SYS_SPAWN, entry, arg)
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".usertext")]
pub extern "Rust" fn u_sys_join(tid: u64) -> u64 {
    u_ecall_a0(SYS_JOIN, tid)
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".usertext")]
pub extern "Rust" fn u_sys_gettid() -> u64 {
    u_ecall_0(SYS_GETTID)
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".usertext")]
pub extern "Rust" fn u_sys_send(tid: u64, bytes: &[u8]) -> u64 {
    u_ecall_a0a1a2(SYS_SEND, tid, bytes.as_ptr() as u64, bytes.len() as u64)
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".usertext")]
pub extern "Rust" fn u_sys_recv(buf: &mut [u8]) -> (u64, u64) {
    u_ecall_recv(SYS_RECV, buf.as_mut_ptr() as u64, buf.len() as u64)
}
