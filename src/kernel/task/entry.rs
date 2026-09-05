pub use crate::user::user_trampoline as task_trampoline_raw;

const _: extern "C" fn(usize) -> ! = task_trampoline_raw;
