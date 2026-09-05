use crate::kernel::task::table::TaskEntry;

pub fn task_trampoline(entry: TaskEntry) -> ! {
    entry();
    crate::kernel::sys::u_sys_exit();
}

#[unsafe(no_mangle)]
pub extern "C" fn task_trampoline_raw(entry_addr: usize) -> ! {
    // SAFETY: `a0` holds the `TaskEntry` pointer stored when the trampoline
    // trap image was built.
    let entry: TaskEntry = unsafe { core::mem::transmute(entry_addr) };

    task_trampoline(entry);
}
