use crate::drivers::uart;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct InitialTaskContext {
    pub entry: u64,
    pub stack_top: u64,
}

pub const INITIAL_TASK_CONTEXT_SIZE: u64 = core::mem::size_of::<InitialTaskContext>() as u64;

pub fn prepare_initial_stack(stack_top: u64, entry: u64) -> Option<u64> {
    let context_sp = align_down(stack_top.checked_sub(INITIAL_TASK_CONTEXT_SIZE)?, 16);
    context_sp
        .checked_add(INITIAL_TASK_CONTEXT_SIZE)
        .filter(|end| *end <= stack_top)?;

    let context = context_sp as *mut InitialTaskContext;

    unsafe {
        (*context).entry = entry;
        (*context).stack_top = stack_top;
    }

    Some(context_sp)
}

pub fn print_initial_context(sp: u64) {
    let context = sp as *const InitialTaskContext;

    unsafe {
        uart::write_str(" prepared_entry: ");
        uart::write_hex_u64((*context).entry);

        uart::write_str(" prepared_stack_top: ");
        uart::write_hex_u64((*context).stack_top);
    }
}

const fn align_down(value: u64, align: u64) -> u64 {
    value & !(align - 1)
}
