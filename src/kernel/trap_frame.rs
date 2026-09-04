/// In-memory GPR image saved by `trap.S`. CSRs `mepc`/`mstatus` are not here;
/// they live on [`TrapImage`].
///
/// Layout must match `src/arch/riscv64/trap.S` (248 bytes).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Riscv64TrapFrame {
    pub sp: u64,

    pub ra: u64,

    pub t0: u64,
    pub t1: u64,
    pub t2: u64,
    pub t3: u64,
    pub t4: u64,
    pub t5: u64,
    pub t6: u64,

    pub a0: u64,
    pub a1: u64,
    pub a2: u64,
    pub a3: u64,
    pub a4: u64,
    pub a5: u64,
    pub a6: u64,
    pub a7: u64,

    pub s0: u64,
    pub s1: u64,
    pub s2: u64,
    pub s3: u64,
    pub s4: u64,
    pub s5: u64,
    pub s6: u64,
    pub s7: u64,
    pub s8: u64,
    pub s9: u64,
    pub s10: u64,
    pub s11: u64,

    pub gp: u64,
    pub tp: u64,
}

pub const TRAP_FRAME_SIZE: usize = 248;

const _: () = assert!(core::mem::size_of::<Riscv64TrapFrame>() == TRAP_FRAME_SIZE);

impl Riscv64TrapFrame {
    pub const fn empty() -> Self {
        Self {
            sp: 0,
            ra: 0,
            t0: 0,
            t1: 0,
            t2: 0,
            t3: 0,
            t4: 0,
            t5: 0,
            t6: 0,
            a0: 0,
            a1: 0,
            a2: 0,
            a3: 0,
            a4: 0,
            a5: 0,
            a6: 0,
            a7: 0,
            s0: 0,
            s1: 0,
            s2: 0,
            s3: 0,
            s4: 0,
            s5: 0,
            s6: 0,
            s7: 0,
            s8: 0,
            s9: 0,
            s10: 0,
            s11: 0,
            gp: 0,
            tp: 0,
        }
    }

    pub const fn s_regs(self) -> [u64; 12] {
        [
            self.s0, self.s1, self.s2, self.s3, self.s4, self.s5, self.s6, self.s7, self.s8,
            self.s9, self.s10, self.s11,
        ]
    }
}

/// Full resume image: GPRs plus the CSRs `trap.S` does not store.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TrapImage {
    pub gpr: Riscv64TrapFrame,
    pub mepc: u64,
    pub mstatus: u64,
}

impl TrapImage {
    pub const fn empty() -> Self {
        Self {
            gpr: Riscv64TrapFrame::empty(),
            mepc: 0,
            mstatus: 0,
        }
    }

    pub fn from_frame(frame: &Riscv64TrapFrame, mepc: u64, mstatus: u64) -> Self {
        Self {
            gpr: *frame,
            mepc,
            mstatus,
        }
    }

    /// Build an mret image from a cooperative yield save (callee-saved only).
    pub fn from_yield_context(ctx: crate::kernel::task::cpu_context::TaskCpuContext) -> Self {
        let mut gpr = Riscv64TrapFrame::empty();
        gpr.sp = ctx.sp;
        gpr.ra = ctx.ra;
        gpr.s0 = ctx.s[0];
        gpr.s1 = ctx.s[1];
        gpr.s2 = ctx.s[2];
        gpr.s3 = ctx.s[3];
        gpr.s4 = ctx.s[4];
        gpr.s5 = ctx.s[5];
        gpr.s6 = ctx.s[6];
        gpr.s7 = ctx.s[7];
        gpr.s8 = ctx.s[8];
        gpr.s9 = ctx.s[9];
        gpr.s10 = ctx.s[10];
        gpr.s11 = ctx.s[11];
        Self {
            gpr,
            mepc: ctx.resume_pc,
            mstatus: 0,
        }
    }

    pub fn to_yield_context(self) -> crate::kernel::task::cpu_context::TaskCpuContext {
        crate::kernel::task::cpu_context::TaskCpuContext {
            sp: self.gpr.sp,
            return_pc: self.mepc,
            resume_pc: self.mepc,
            ra: self.gpr.ra,
            s: self.gpr.s_regs(),
        }
    }
}
