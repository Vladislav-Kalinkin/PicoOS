#[cfg(target_arch = "aarch64")]
mod qemu_virt_aarch64;

#[cfg(target_arch = "riscv64")]
mod qemu_virt_riscv64;

#[cfg(target_arch = "aarch64")]
pub use qemu_virt_aarch64::*;

#[cfg(target_arch = "riscv64")]
pub use qemu_virt_riscv64::*;
