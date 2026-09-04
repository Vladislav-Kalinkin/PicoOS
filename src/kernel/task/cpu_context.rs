use crate::drivers::uart;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TaskCpuContext {
    pub sp: u64,
    pub return_pc: u64,
    pub resume_pc: u64,
    pub ra: u64,
    pub s: [u64; 12],
}

impl TaskCpuContext {
    pub const fn empty() -> Self {
        Self::initial(0, 0)
    }

    pub const fn initial(sp: u64, return_pc: u64) -> Self {
        Self {
            sp,
            return_pc,
            resume_pc: return_pc,
            ra: return_pc,
            s: [0; 12],
        }
    }

    pub const fn is_valid(&self) -> bool {
        self.sp != 0 && self.return_pc != 0 && self.resume_pc != 0
    }
}

pub fn print_cpu_context(context: TaskCpuContext) {
    uart::write_str(" cpu_context: sp: ");
    uart::write_hex_u64(context.sp);

    uart::write_str(" return_pc: ");
    uart::write_hex_u64(context.return_pc);

    uart::write_str(" resume_pc: ");
    uart::write_hex_u64(context.resume_pc);

    uart::write_str(" ra: ");
    uart::write_hex_u64(context.ra);

    uart::write_str(" s0: ");
    uart::write_hex_u64(context.s[0]);

    uart::write_str(" s1: ");
    uart::write_hex_u64(context.s[1]);

    uart::write_str(" s2: ");
    uart::write_hex_u64(context.s[2]);

    uart::write_str(" s11: ");
    uart::write_hex_u64(context.s[11]);
}
