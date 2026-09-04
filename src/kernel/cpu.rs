use crate::kernel::task::table::TaskReturnKind;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TrapExecutionContext {
    Kernel,
    Task,
}

pub struct Cpu {
    current: Option<usize>,
    in_trap: bool,
    kernel_return_pc: u64,
    kernel_sp_before_task: u64,
    current_stack_start: u64,
    current_stack_top: u64,
    task_run_stage: u64,
    task_return_kind: TaskReturnKind,
    last_task_sp: u64,
    task_resume_pc: u64,
}

impl Cpu {
    const fn new() -> Self {
        Self {
            current: None,
            in_trap: false,
            kernel_return_pc: 0,
            kernel_sp_before_task: 0,
            current_stack_start: 0,
            current_stack_top: 0,
            task_run_stage: 0,
            task_return_kind: TaskReturnKind::None,
            last_task_sp: 0,
            task_resume_pc: 0,
        }
    }
}

static mut CPU: Cpu = Cpu::new();

pub fn current() -> Option<usize> {
    unsafe { CPU.current }
}

#[allow(dead_code)]
pub fn set_current(id: usize) {
    unsafe {
        CPU.current = Some(id);
    }
}

#[allow(dead_code)]
pub fn clear_current() {
    unsafe {
        CPU.current = None;
    }
}

#[allow(dead_code)]
pub fn in_trap() -> bool {
    unsafe { CPU.in_trap }
}

#[allow(dead_code)]
pub fn set_in_trap(value: bool) {
    unsafe {
        CPU.in_trap = value;
    }
}

pub fn trap_execution_context() -> TrapExecutionContext {
    if current().is_some() {
        TrapExecutionContext::Task
    } else {
        TrapExecutionContext::Kernel
    }
}

#[allow(dead_code)]
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

pub fn kernel_return_pc() -> u64 {
    unsafe { CPU.kernel_return_pc }
}

#[allow(dead_code)]
pub fn set_kernel_return_pc(pc: u64) {
    unsafe {
        CPU.kernel_return_pc = pc;
    }
}

pub fn kernel_sp_before_task() -> u64 {
    unsafe { CPU.kernel_sp_before_task }
}

#[allow(dead_code)]
pub fn set_kernel_sp_before_task(sp: u64) {
    unsafe {
        CPU.kernel_sp_before_task = sp;
    }
}

pub fn current_stack_start() -> u64 {
    unsafe { CPU.current_stack_start }
}

pub fn current_stack_top() -> u64 {
    unsafe { CPU.current_stack_top }
}

#[allow(dead_code)]
pub fn set_current_stack_bounds(start: u64, top: u64) {
    unsafe {
        CPU.current_stack_start = start;
        CPU.current_stack_top = top;
    }
}

pub fn last_task_sp() -> u64 {
    unsafe { CPU.last_task_sp }
}

pub fn set_last_task_sp(sp: u64) {
    unsafe {
        CPU.last_task_sp = sp;
    }
}

pub fn task_run_stage() -> u64 {
    unsafe { CPU.task_run_stage }
}

#[allow(dead_code)]
pub fn set_task_run_stage(stage: u64) {
    unsafe {
        CPU.task_run_stage = stage;
    }
}

pub fn task_return_kind() -> TaskReturnKind {
    unsafe { CPU.task_return_kind }
}

pub fn set_task_return_kind(kind: TaskReturnKind) {
    unsafe {
        CPU.task_return_kind = kind;
    }
}

pub fn task_resume_pc() -> u64 {
    unsafe { CPU.task_resume_pc }
}

pub fn set_task_resume_pc(pc: u64) {
    unsafe {
        CPU.task_resume_pc = pc;
    }
}

pub fn set_task_resume_context(task_sp: u64, resume_pc: u64) {
    set_last_task_sp(task_sp);
    set_task_resume_pc(resume_pc);
}

pub fn print_task_resume_context() {
    let task_sp = last_task_sp();
    let resume_pc = task_resume_pc();

    crate::drivers::uart::write_str("yield resume PC: ");
    crate::drivers::uart::write_hex_u64(resume_pc);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("yield current SP: ");
    crate::drivers::uart::write_hex_u64(task_sp);
    crate::drivers::uart::write_line("");
}
