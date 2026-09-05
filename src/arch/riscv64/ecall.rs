//! U-mode `ecall` helpers. Linked into `.usertext` so U may fetch them.

#[unsafe(link_section = ".usertext")]
pub fn u_ecall_0(nr: u64) -> u64 {
    let ret: u64;
    // SAFETY: uncompressed U-mode `ecall`; `nr` is a defined syscall in `a7`.
    // The worker stack is live. Kernel same-frame returns write `a0`.
    unsafe {
        core::arch::asm!(
            ".option push",
            ".option norvc",
            "ecall",
            ".option pop",
            lateout("a0") ret,
            inlateout("a7") nr => _,
            options(nomem, nostack)
        );
    }
    ret
}

#[unsafe(link_section = ".usertext")]
pub fn u_ecall_a0(nr: u64, a0: u64) -> u64 {
    let ret: u64;
    // SAFETY: uncompressed U-mode `ecall`; `a0` is the syscall argument and
    // the kernel writes the result into the trap-stack `a0` before `mret`.
    unsafe {
        core::arch::asm!(
            ".option push",
            ".option norvc",
            "ecall",
            ".option pop",
            inlateout("a0") a0 => ret,
            inlateout("a7") nr => _,
            options(nomem, nostack)
        );
    }
    ret
}

#[unsafe(link_section = ".usertext")]
pub fn u_ecall_a0a1(nr: u64, a0: u64, a1: u64) -> u64 {
    let ret: u64;
    // SAFETY: uncompressed U-mode `ecall`; `a0`/`a1` are syscall arguments
    // (entry/arg, or a log buffer pointer/length). The kernel may read that
    // buffer; result returns in `a0`.
    unsafe {
        core::arch::asm!(
            ".option push",
            ".option norvc",
            "ecall",
            ".option pop",
            inlateout("a0") a0 => ret,
            in("a1") a1,
            inlateout("a7") nr => _,
            options(nostack)
        );
    }
    ret
}
