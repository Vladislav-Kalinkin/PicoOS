pub mod mmio;
pub mod uart;

#[cfg(target_arch = "aarch64")]
pub mod gic;
