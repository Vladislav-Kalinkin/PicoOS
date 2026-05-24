pub mod riscv64;

#[cfg(any(feature = "resume_restore_test", feature = "scheduler_dispatch_test"))]
pub use riscv64::restore_verified_resume_frame;

pub use riscv64::*;

/// Public arch-level wrapper for the RISC-V yield boundary.
///
/// The actual symbol is provided by riscv64 global_asm! and declared through
/// an unsafe extern block in riscv64/mod.rs. We keep this wrapper so task code
/// can call crate::arch::task_yield_boundary(...) without depending directly
/// on the riscv64 module layout.
#[cfg(target_arch = "riscv64")]
pub unsafe fn task_yield_boundary(kernel_sp: u64, return_pc: u64) {
    unsafe {
        riscv64::task_yield_boundary(kernel_sp, return_pc);
    }
}
