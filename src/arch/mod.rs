#[cfg(target_arch = "aarch64")]
pub mod aarch64;

#[cfg(target_arch = "riscv64")]
pub mod riscv64;

#[cfg(target_arch = "aarch64")]
pub use aarch64::*;

#[cfg(target_arch = "riscv64")]
pub use riscv64::*;

#[cfg(target_arch = "riscv64")]
pub use riscv64::yield_to_kernel_and_return;

#[cfg(target_arch = "aarch64")]
pub use aarch64::yield_to_kernel_and_return;
