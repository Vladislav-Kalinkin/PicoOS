pub mod entry;
pub mod fault;
pub mod scheduler;
pub mod table;
pub mod test;

pub use table::*;

pub fn print_task_context_values(saved_sp: u64, saved_pc: u64) {
    crate::drivers::uart::write_str(" saved_sp: ");
    crate::drivers::uart::write_hex_u64(saved_sp);
    crate::drivers::uart::write_str(" saved_pc: ");
    crate::drivers::uart::write_hex_u64(saved_pc);
}

pub fn print_task_full_context_by_id(id: table::TaskId) {
    crate::drivers::uart::write_str(" initial_sp: ");
    match table::get_task_initial_sp(id) {
        Some(sp) => crate::drivers::uart::write_hex_u64(sp),
        None => crate::drivers::uart::write_str("none"),
    }
    crate::drivers::uart::write_str(" initial_pc: ");
    match table::get_task_initial_pc(id) {
        Some(pc) => crate::drivers::uart::write_hex_u64(pc),
        None => crate::drivers::uart::write_str("none"),
    }
}
