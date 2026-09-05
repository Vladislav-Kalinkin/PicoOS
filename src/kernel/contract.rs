use crate::drivers::uart;
use crate::kernel::irq_cell::IrqCell;

#[used]
#[unsafe(no_mangle)]
#[unsafe(link_section = ".boot_contract")]
static BOOT_CONTRACT: u8 = 0;

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum BootContract {
    Default = 0,
    Resume = 1,
    Handoff = 2,
    Sleep = 3,
    Fault = 4,
    Preempt = 5,
    Reap = 6,
    KernelFault = 7,
    Ipc = 8,
    UserText = 9,
}

impl BootContract {
    pub const fn from_byte(byte: u8) -> Self {
        match byte {
            1 => Self::Resume,
            2 => Self::Handoff,
            3 => Self::Sleep,
            4 => Self::Fault,
            5 => Self::Preempt,
            6 => Self::Reap,
            7 => Self::KernelFault,
            8 => Self::Ipc,
            9 => Self::UserText,
            _ => Self::Default,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Resume => "resume",
            Self::Handoff => "handoff",
            Self::Sleep => "sleep",
            Self::Fault => "fault",
            Self::Preempt => "preempt",
            Self::Reap => "reap",
            Self::KernelFault => "kernel_fault",
            Self::Ipc => "ipc",
            Self::UserText => "user_text",
        }
    }

    pub const fn is_known_byte(byte: u8) -> bool {
        byte <= 9
    }
}

static PLAN: IrqCell<BootContract> = IrqCell::new(BootContract::Default);

pub fn plan() -> BootContract {
    PLAN.with(|plan| *plan)
}

pub fn set_plan(contract: BootContract) {
    PLAN.with(|plan| *plan = contract);
}

fn boot_contract_byte() -> u8 {
    // SAFETY: M-mode; `.boot_contract` is 8 allocated PROGBITS bytes in the
    // kernel image below `__free_memory_start`; QEMU `-kernel` loaded them;
    // scripts may have patched byte 0 before exec. Not a `static mut` write.
    unsafe { core::ptr::read_volatile(core::ptr::addr_of!(BOOT_CONTRACT)) }
}

pub fn apply_boot_contract() {
    let byte = boot_contract_byte();
    if !BootContract::is_known_byte(byte) {
        uart::write_line("boot contract: unknown");
        set_plan(BootContract::Default);
        return;
    }

    let contract = BootContract::from_byte(byte);
    set_plan(contract);
    uart::write_str("boot contract: ");
    uart::write_line(contract.name());
}
