use crate::kernel::irq_cell::IrqCell;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TrapExecutionContext {
    Kernel,
    Task,
}

pub struct Cpu {
    current: Option<usize>,
    kernel_sp_before_task: u64,
    current_stack_start: u64,
    current_stack_top: u64,
}

impl Cpu {
    const fn new() -> Self {
        Self {
            current: None,
            kernel_sp_before_task: 0,
            current_stack_start: 0,
            current_stack_top: 0,
        }
    }
}

static CPU: IrqCell<Cpu> = IrqCell::new(Cpu::new());

pub fn current() -> Option<usize> {
    CPU.with(|cpu| cpu.current)
}

pub fn set_current(id: usize) {
    CPU.with(|cpu| cpu.current = Some(id));
}

pub fn clear_current() {
    CPU.with(|cpu| cpu.current = None);
}

pub fn trap_execution_context() -> TrapExecutionContext {
    if current().is_some() {
        TrapExecutionContext::Task
    } else {
        TrapExecutionContext::Kernel
    }
}

pub fn print_trap_execution_context() {
    match trap_execution_context() {
        TrapExecutionContext::Kernel => {
            crate::drivers::uart::write_line("trap execution context: kernel");
        }
        TrapExecutionContext::Task => {
            crate::drivers::uart::write_line("trap execution context: task");
        }
    }
}

pub fn kernel_sp_before_task() -> u64 {
    CPU.with(|cpu| cpu.kernel_sp_before_task)
}

pub fn set_kernel_sp_before_task(sp: u64) {
    CPU.with(|cpu| cpu.kernel_sp_before_task = sp);
}

pub fn current_stack_start() -> u64 {
    CPU.with(|cpu| cpu.current_stack_start)
}

pub fn current_stack_top() -> u64 {
    CPU.with(|cpu| cpu.current_stack_top)
}

pub fn set_current_stack_bounds(start: u64, top: u64) {
    CPU.with(|cpu| {
        cpu.current_stack_start = start;
        cpu.current_stack_top = top;
    });
}

const _: fn() = print_trap_execution_context;
