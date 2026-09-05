use crate::kernel::trap_frame::{TRAP_FRAME_SIZE, TrapImage};

unsafe extern "C" {
    fn trap_return() -> !;
}

/// Rewrite a trap-stack frame from `image`, `csrw mepc`/`mstatus` (`MPP=U`,
/// `MIE=0`, `MPIE=1`), then enter `trap.S` `trap_return` so the epilogue `mret`s.
/// Timer path does **not** add 4 to `mepc`.
pub fn mret_to_trap_image(image: &TrapImage) -> ! {
    crate::arch::disable_irq();

    let trap_top = super::trap_stack_top();
    let frame =
        (trap_top - TRAP_FRAME_SIZE as u64) as *mut crate::kernel::trap_frame::Riscv64TrapFrame;

    // SAFETY: `frame` is the trap-stack slot (`trap_top - TRAP_FRAME_SIZE`)
    // that `trap.S` `trap_return` loads GPRs from.
    unsafe {
        *frame = image.gpr;
    }

    super::cpu::set_mepc(image.mepc);
    super::cpu::set_mstatus(super::cpu::synthesize_mstatus_for_mret_worker());

    // SAFETY: `sp` points at the rewritten frame; `trap_return` is the asm
    // epilogue and does not return to Rust.
    unsafe {
        core::arch::asm!(
            "mv sp, {frame}",
            "j {epilogue}",
            frame = in(reg) frame,
            epilogue = sym trap_return,
            options(noreturn)
        );
    }
}

/// Worker → idle: reset `mscratch`, switch to the kernel stack, enable MIE,
/// jump to `idle_loop`. Must not `mret` (that would leave `mscratch` as the
/// worker SP).
pub fn idle_exit_from_trap() -> ! {
    crate::kernel::task::scheduler::switch_to_idle();

    let trap_top = super::trap_stack_top();
    let kernel_sp = {
        let saved = crate::kernel::cpu::kernel_sp_before_task();
        if saved == 0 {
            crate::kernel::memory::stack_top()
        } else {
            saved
        }
    };
    let mstatus = super::cpu::synthesize_mstatus_for_idle();

    // SAFETY: idle exit is M-mode only; `mscratch` is the trap stack, `sp` is
    // the kernel stack, and `idle_loop` never returns.
    unsafe {
        core::arch::asm!(
            "csrw mscratch, {trap_top}",
            "csrw mstatus, {mstatus}",
            "mv sp, {kernel_sp}",
            "j {idle}",
            trap_top = in(reg) trap_top,
            mstatus = in(reg) mstatus,
            kernel_sp = in(reg) kernel_sp,
            idle = sym crate::kernel::task::scheduler::idle_loop,
            options(noreturn)
        );
    }
}
