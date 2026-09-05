mod stubs;
mod workers;

pub use stubs::{u_sys_exit, u_sys_log, u_sys_sleep, u_sys_yield};
pub use workers::*;

use crate::kernel::task::table::TaskEntry;

#[unsafe(no_mangle)]
#[unsafe(link_section = ".usertext")]
pub extern "C" fn user_trampoline(entry_addr: usize) -> ! {
    // SAFETY: `a0` is the `TaskEntry` address stored by `build_fresh_trap_image`.
    let entry: TaskEntry = unsafe { core::mem::transmute(entry_addr) };
    entry();
    u_sys_exit();
}
