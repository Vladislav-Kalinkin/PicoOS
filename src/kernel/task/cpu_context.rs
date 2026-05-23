use crate::drivers::uart;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TaskCpuContext {
    pub sp: u64,        // offset 0
    pub return_pc: u64, // offset 8
    pub resume_pc: u64, // offset 16

    #[cfg(target_arch = "riscv64")]
    pub ra: u64,
    #[cfg(target_arch = "riscv64")]
    pub s: [u64; 12],

    #[cfg(target_arch = "aarch64")]
    pub x19_x30: [u64; 12],
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

            #[cfg(target_arch = "riscv64")]
            ra: return_pc,

            #[cfg(target_arch = "riscv64")]
            s: [0; 12],

            #[cfg(target_arch = "aarch64")]
            x19_x30: [
                0,         // x19
                0,         // x20
                0,         // x21
                0,         // x22
                0,         // x23
                0,         // x24
                0,         // x25
                0,         // x26
                0,         // x27
                0,         // x28
                0,         // x29 / FP
                return_pc, // x30 / LR
            ],
        }
    }

    #[allow(dead_code)]
    pub fn is_valid(&self) -> bool {
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

    #[cfg(target_arch = "riscv64")]
    {
        uart::write_str(" ra: ");
        uart::write_hex_u64(context.ra);

        uart::write_str(" s0: ");
        uart::write_hex_u64(context.s[0]);

        uart::write_str(" s1: ");
        uart::write_hex_u64(context.s[1]);
    }

    #[cfg(target_arch = "aarch64")]
    {
        uart::write_str(" x19: ");
        uart::write_hex_u64(context.x19_x30[0]);

        uart::write_str(" x20: ");
        uart::write_hex_u64(context.x19_x30[1]);

        uart::write_str(" x21: ");
        uart::write_hex_u64(context.x19_x30[2]);

        uart::write_str(" x22: ");
        uart::write_hex_u64(context.x19_x30[3]);

        uart::write_str(" x29: ");
        uart::write_hex_u64(context.x19_x30[10]);

        uart::write_str(" x30: ");
        uart::write_hex_u64(context.x19_x30[11]);
    }
}
