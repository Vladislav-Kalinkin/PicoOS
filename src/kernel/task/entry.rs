pub use crate::arch::riscv64::user_trampoline as task_trampoline_raw;

const _: unsafe extern "C" fn() = task_trampoline_raw;
