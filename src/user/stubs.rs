use crate::kernel::sys::{SYS_EXIT, SYS_LOG, SYS_SLEEP, SYS_YIELD};

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
#[unsafe(no_mangle)]
#[unsafe(link_section = ".usertext")]
pub extern "Rust" fn u_sys_yield() {
    // SAFETY: U-mode `ecall` with a defined syscall number; worker stack is live.
    unsafe {
        ecall!(SYS_YIELD);
    }
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".usertext")]
pub extern "Rust" fn u_sys_sleep(ticks: u64) {
    // SAFETY: U-mode `ecall` with `a0` = sleep ticks; worker stack is live.
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

#[unsafe(no_mangle)]
#[unsafe(link_section = ".usertext")]
pub extern "Rust" fn u_sys_exit() -> ! {
    // SAFETY: U-mode `ecall`; spinning is unreachable if the kernel honors SYS_EXIT.
    unsafe {
        ecall!(SYS_EXIT);
    }
    loop {
        core::hint::spin_loop();
    }
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".usertext")]
pub extern "Rust" fn u_sys_log(bytes: &[u8]) {
    // SAFETY: U-mode `ecall`; `a0`/`a1` point at this worker's slice.
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
