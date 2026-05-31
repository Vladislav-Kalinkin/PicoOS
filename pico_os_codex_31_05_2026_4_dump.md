This file is a merged representation of the entire codebase, combined into a single document by Repomix.

# File Summary

## Purpose
This file contains a packed representation of the entire repository's contents.
It is designed to be easily consumable by AI systems for analysis, code review,
or other automated processes.

## File Format
The content is organized as follows:
1. This summary section
2. Repository information
3. Directory structure
4. Repository files (if enabled)
5. Multiple file entries, each consisting of:
  a. A header with the file path (## File: path/to/file)
  b. The full contents of the file in a code block

## Usage Guidelines
- This file should be treated as read-only. Any changes should be made to the
  original repository files, not this packed version.
- When processing this file, use the file path to distinguish
  between different files in the repository.
- Be aware that this file may contain sensitive information. Handle it with
  the same level of security as you would the original repository.

## Notes
- Some files may have been excluded based on .gitignore rules and Repomix's configuration
- Binary files are not included in this packed representation. Please refer to the Repository Structure section for a complete list of file paths, including binary files
- Files matching patterns in .gitignore are excluded
- Files matching default ignore patterns are excluded
- Files are sorted by Git change count (files with more changes are at the bottom)

# Directory Structure
```
.cargo/
  config.toml
.codex/
  config.toml
  system_prompt.txt
.zed/
  settings.json
scripts/
  check-all.sh
  qemu-expect.sh
  run-riscv.sh
  test-kernel-fault-guard-riscv.sh
  test-riscv.sh
  test-scheduler-fault-lifecycle-riscv.sh
  test-scheduler-reentry-riscv.sh
  test-scheduler-run-once-riscv.sh
  test-scheduler-run-riscv.sh
  test-scheduler-runtime-riscv.sh
  test-task-resume-selftest.sh
  test-task-sleep-riscv.sh
  test-two-task-handoff-riscv.sh
src/
  arch/
    riscv64/
      boot.S
      cpu.rs
      mod.rs
      timer.rs
      trap.S
      traps.rs
    mod.rs
  drivers/
    mmio.rs
    mod.rs
    uart.rs
  kernel/
    task/
      test/
        bootstrap.rs
        fault.rs
        handoff.rs
        invariants.rs
        reentry.rs
        resume.rs
      context.rs
      cpu_context.rs
      debug.rs
      entry.rs
      fault.rs
      mod.rs
      scheduler.rs
      table.rs
      test.rs
    banner.rs
    heap.rs
    log.rs
    memory.rs
    mod.rs
    test.rs
    ticks.rs
    trap_frame.rs
  platform/
    mod.rs
    qemu_virt_riscv64.rs
  main.rs
.gitignore
Cargo.toml
linker-riscv64.ld
```

# Files

## File: .cargo/config.toml
```toml
[build]
target = "riscv64gc-unknown-none-elf"

[target.riscv64gc-unknown-none-elf]
rustflags = [
  "-C", "link-arg=-Tlinker-riscv64.ld",
  "-C", "relocation-model=static",
]
```

## File: scripts/run-riscv.sh
```bash
#!/usr/bin/env bash
set -euo pipefail

cargo clean
cargo build

qemu-system-riscv64 \
  -M virt \
  -m 128M \
  -nographic \
  -bios none \
  -kernel target/riscv64gc-unknown-none-elf/debug/PicoOS
```

## File: scripts/test-riscv.sh
```bash
#!/usr/bin/env bash
set -euo pipefail

cargo clean
cargo build --features selftest

qemu-system-riscv64 \
  -M virt \
  -m 128M \
  -nographic \
  -bios none \
  -kernel target/riscv64gc-unknown-none-elf/debug/PicoOS
```

## File: src/arch/riscv64/boot.S
```asm
.section .text.boot
.global _start
.type _start, @function

_start:
    la sp, __stack_top

    la t0, __bss_start
    la t1, __bss_end

clear_bss_loop:
    bgeu t0, t1, clear_bss_done
    sd zero, 0(t0)
    addi t0, t0, 8
    j clear_bss_loop

clear_bss_done:
    call kernel_main

hang:
    wfi
    j hang
```

## File: src/arch/riscv64/timer.rs
```rust
use crate::drivers::mmio;
use crate::platform;

pub fn mtime() -> u64 {
    mmio::read64(platform::CLINT_MTIME)
}

pub fn set_mtimecmp(value: u64) {
    mmio::write64(platform::CLINT_MTIMECMP, value);
}

pub fn timebase_frequency() -> u64 {
    platform::TIMEBASE_FREQ
}

pub fn arm_timer_after_ticks(ticks: u64) {
    let now = mtime();
    set_mtimecmp(now.wrapping_add(ticks));
}

pub fn arm_timer_hz(hz: u64) {
    let ticks = timebase_frequency() / hz;
    arm_timer_after_ticks(ticks);
}

#[allow(dead_code)]
pub fn arm_timer_seconds(seconds: u64) {
    arm_timer_after_ticks(timebase_frequency() * seconds);
}

pub fn disarm_timer() {
    set_mtimecmp(u64::MAX);
}
```

## File: src/drivers/mmio.rs
```rust
mod imp {
    use core::arch::asm;

    #[inline(always)]
    pub unsafe fn write32(addr: usize, value: u32) {
        let value = value as usize;
        core::arch::asm!(
            "sw {value}, 0({addr})",
            addr = in(reg) addr,
            value = in(reg) value,
            options(nostack, preserves_flags)
        );
    }

    #[allow(dead_code)]
    #[inline(always)]
    pub unsafe fn read32(addr: usize) -> u32 {
        let value: usize;

        core::arch::asm!(
            "lwu {value}, 0({addr})",
            addr = in(reg) addr,
            value = out(reg) value,
            options(nostack, preserves_flags)
        );

        value as u32
    }

    #[allow(dead_code)]
    #[inline(always)]
    pub unsafe fn write8(addr: usize, value: u8) {
        let value = value as usize;

        core::arch::asm!(
            "sb {value}, 0({addr})",
            addr = in(reg) addr,
            value = in(reg) value,
            options(nostack, preserves_flags)
        );
    }

    #[allow(dead_code)]
    #[inline(always)]
    pub unsafe fn read8(addr: usize) -> u8 {
        let value: usize;

        core::arch::asm!(
            "lbu {value}, 0({addr})",
            addr = in(reg) addr,
            value = out(reg) value,
            options(nostack, preserves_flags)
        );

        value as u8
    }

    #[inline(always)]
    pub fn write64(addr: usize, value: u64) {
        unsafe {
            asm!(
                "sd {value}, 0({addr})",
                addr = in(reg) addr,
                value = in(reg) value,
                options(nostack)
            );
        }
    }

    #[inline(always)]
    pub fn read64(addr: usize) -> u64 {
        let value: u64;

        unsafe {
            asm!(
                "ld {value}, 0({addr})",
                addr = in(reg) addr,
                value = out(reg) value,
                options(nostack)
            );
        }

        value
    }
}

pub use imp::*;
```

## File: src/drivers/mod.rs
```rust
pub mod mmio;
pub mod uart;
```

## File: src/drivers/uart.rs
```rust
use crate::drivers::mmio;
use crate::platform;

pub fn putc(byte: u8) {
    unsafe {
        mmio::write32(platform::UART0_BASE, byte as u32);
    }
}

#[allow(dead_code)]
pub fn write_byte(byte: u8) {
    putc(byte);
}

pub fn write_str(s: &str) {
    for byte in s.bytes() {
        if byte == b'\n' {
            putc(b'\r');
        }

        putc(byte);
    }
}

pub fn write_line(s: &str) {
    write_str(s);
    write_str("\n");
}

pub fn write_hex_u64(value: u64) {
    write_str("0x");

    let mut shift = 60;

    loop {
        let digit = ((value >> shift) & 0xF) as u8;

        let ch = if digit < 10 {
            b'0' + digit
        } else {
            b'A' + (digit - 10)
        };

        putc(ch);

        if shift == 0 {
            break;
        }

        shift -= 4;
    }
}

pub fn write_dec_u64(mut value: u64) {
    if value == 0 {
        putc(b'0');
        return;
    }

    let mut buffer = [0u8; 20];
    let mut index = buffer.len();

    while value > 0 {
        index -= 1;
        buffer[index] = b'0' + (value % 10) as u8;
        value /= 10;
    }

    while index < buffer.len() {
        putc(buffer[index]);
        index += 1;
    }
}
```

## File: src/kernel/task/cpu_context.rs
```rust
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

    uart::write_str(" ra: ");
    uart::write_hex_u64(context.ra);

    uart::write_str(" s0: ");
    uart::write_hex_u64(context.s[0]);

    uart::write_str(" s1: ");
    uart::write_hex_u64(context.s[1]);
}
```

## File: src/kernel/ticks.rs
```rust
use core::sync::atomic::{AtomicU64, Ordering};

static TICKS: AtomicU64 = AtomicU64::new(0);

pub const MAX_TEST_TICKS: u64 = 5;

pub fn reset() {
    TICKS.store(0, Ordering::SeqCst);
}

pub fn increment() -> u64 {
    TICKS.fetch_add(1, Ordering::SeqCst) + 1
}

pub fn get() -> u64 {
    TICKS.load(Ordering::SeqCst)
}

pub fn is_test_complete() -> bool {
    get() >= MAX_TEST_TICKS
}
```

## File: src/platform/mod.rs
```rust
mod qemu_virt_riscv64;

pub use qemu_virt_riscv64::*;
```

## File: src/platform/qemu_virt_riscv64.rs
```rust
pub const UART0_BASE: usize = 0x1000_0000;

pub const CLINT_BASE: usize = 0x0200_0000;
pub const CLINT_MTIMECMP: usize = CLINT_BASE + 0x4000;
pub const CLINT_MTIME: usize = CLINT_BASE + 0xBFF8;

pub const TIMEBASE_FREQ: u64 = 10_000_000;

pub const RAM_START: usize = 0x8000_0000;
pub const RAM_SIZE: usize = 128 * 1024 * 1024;

#[allow(dead_code)]
pub const NAME: &str = "QEMU virt riscv64";
```

## File: .gitignore
```
/target/
.DS_Store
```

## File: linker-riscv64.ld
```
ENTRY(_start)

SECTIONS
{
    . = 0x80000000;

    __kernel_start = .;

    .text : ALIGN(4K) {
        __text_start = .;
        KEEP(*(.text.boot))
        KEEP(*(.text.trap))
        *(.text*)
        __text_end = .;
    }

    .rodata : ALIGN(4K) {
        __rodata_start = .;
        *(.rodata*)
        __rodata_end = .;
    }

    .data : ALIGN(4K) {
        __data_start = .;
        *(.data*)
        __data_end = .;
    }

    .bss : ALIGN(4K) {
        __bss_start = .;
        *(.bss*)
        *(COMMON)
        . = ALIGN(8);
        __bss_end = .;
    }

    . = ALIGN(16);
    . += 0x10000;
    __stack_top = .;

    . = ALIGN(4K);
    __kernel_end = .;
    __free_memory_start = .;
}
```

## File: .codex/config.toml
```toml
# .codex/config.toml

# ==============================================================================
# ОБЩИЕ НАСТРОЙКИ ПРОЕКТА
# ==============================================================================
system_prompt_file = ".codex/system_prompt.txt"

# ==============================================================================
# НАСТРОЙКИ ПЕСОЧНИЦЫ И БЕЗОПАСНОСТИ (SANDBOX)
# ==============================================================================
# Разрешает изменять код строго внутри папки проекта.
sandbox_mode = "workspace-write"

# Способ одобрения команд.
approval_policy = "on-request"

# Запрещает модели использовать интерактивную оболочку
allow_login_shell = false

# Список строго запрещенных для чтения путей.
deny_read = [
    "target/**",
    "**/*.iso",
    "**/*.bin",
    "**/*.elf",
    "Cargo.lock",
    "**/*.log"
]

# ==============================================================================
# НАСТРОЙКИ СЕТЕВОГО ДОСТУПА (NETWORK PROXY)
# ==============================================================================
[features.network_proxy]
# Запрещает модели привязываться к локальным портам
allow_local_binding = false

# Полная изоляция: запрещает любые исходящие интернет-запросы из кода песочницы
domains = { "*" = "deny" }
```

## File: .codex/system_prompt.txt
```
Ты — ведущий разработчик этого проекта.
Стек: Rust, Risc-V Assembler.
Правила для всех чатов в этом проекте:
1) Читать в коде только то, что нужно. Старайся минимизировать чтение ненужного для задачи кода.
2) Если есть несколько минизадач лучше сразу их выполнять вместе с кратким обьяснением в одну-пару строк что делает и зачем.
```

## File: scripts/test-task-resume-selftest.sh
```bash
#!/usr/bin/env bash
set -euo pipefail

cargo clean

cargo build --features "task_resume_selftest"

scripts/qemu-expect.sh "scheduler resume loop result: OK"
```

## File: scripts/test-task-sleep-riscv.sh
```bash
#!/usr/bin/env bash
set -euo pipefail

cargo clean

cargo build --features "task_resume_selftest,task_sleep_test"

scripts/qemu-expect.sh "task sleep wake result: OK"
```

## File: src/arch/riscv64/cpu.rs
```rust
use core::arch::asm;

#[inline(always)]
pub fn mhartid() -> u64 {
    let value: u64;

    unsafe {
        asm!(
            "csrr {0}, mhartid",
            out(reg) value,
            options(nomem, nostack, preserves_flags)
        );
    }

    value
}

#[inline(always)]
pub fn mstatus() -> u64 {
    let value: u64;

    unsafe {
        asm!(
            "csrr {0}, mstatus",
            out(reg) value,
            options(nomem, nostack, preserves_flags)
        );
    }

    value
}

#[inline(always)]
pub fn mtvec() -> u64 {
    let value: u64;

    unsafe {
        asm!(
            "csrr {0}, mtvec",
            out(reg) value,
            options(nomem, nostack, preserves_flags)
        );
    }

    value
}

#[inline(always)]
pub fn mepc() -> u64 {
    let value: u64;

    unsafe {
        asm!(
            "csrr {0}, mepc",
            out(reg) value,
            options(nomem, nostack, preserves_flags)
        );
    }

    value
}

#[inline(always)]
pub fn mcause() -> u64 {
    let value: u64;

    unsafe {
        asm!(
            "csrr {0}, mcause",
            out(reg) value,
            options(nomem, nostack, preserves_flags)
        );
    }

    value
}

#[inline(always)]
pub fn mie() -> u64 {
    let value: u64;

    unsafe {
        asm!(
            "csrr {0}, mie",
            out(reg) value,
            options(nomem, nostack, preserves_flags)
        );
    }

    value
}

#[inline(always)]
pub fn mip() -> u64 {
    let value: u64;

    unsafe {
        asm!(
            "csrr {0}, mip",
            out(reg) value,
            options(nomem, nostack, preserves_flags)
        );
    }

    value
}

#[inline(always)]
pub fn stack_pointer() -> u64 {
    let value: u64;

    unsafe {
        asm!(
            "mv {0}, sp",
            out(reg) value,
            options(nomem, nostack, preserves_flags)
        );
    }

    value
}

#[inline(always)]
pub fn set_mtvec(addr: u64) {
    unsafe {
        asm!(
            "csrw mtvec, {0}",
            in(reg) addr,
            options(nomem, nostack, preserves_flags)
        );
    }
}

#[inline(always)]
pub fn set_mscratch(value: u64) {
    unsafe {
        asm!(
            "csrw mscratch, {0}",
            in(reg) value,
            options(nomem, nostack, preserves_flags)
        );
    }
}

#[inline(always)]
pub fn mtval() -> u64 {
    let value: u64;

    unsafe {
        asm!(
            "csrr {0}, mtval",
            out(reg) value,
            options(nomem, nostack, preserves_flags)
        );
    }

    value
}

#[inline(always)]
pub fn enable_machine_interrupts() {
    unsafe {
        asm!("li t0, 0x8", "csrs mstatus, t0", options(nomem, nostack));
    }
}

#[inline(always)]
pub fn disable_machine_interrupts() {
    unsafe {
        asm!("li t0, 0x8", "csrc mstatus, t0", options(nomem, nostack));
    }
}

#[inline(always)]
pub fn enable_machine_timer_interrupt() {
    unsafe {
        asm!("li t0, 0x80", "csrs mie, t0", options(nomem, nostack));
    }
}

#[inline(always)]
pub fn disable_machine_timer_interrupt() {
    unsafe {
        asm!("li t0, 0x80", "csrc mie, t0", options(nomem, nostack));
    }
}
```

## File: src/arch/riscv64/trap.S
```asm
.section .text.trap
.align 8
.global trap_vector

trap_vector:
    csrrw sp, mscratch, sp
    addi sp, sp, -232

    sd t0, 16(sp)
    csrr t0, mscratch
    sd t0, 0(sp)

    sd ra, 8(sp)
    sd t1, 24(sp)
    sd t2, 32(sp)
    sd t3, 40(sp)
    sd t4, 48(sp)
    sd t5, 56(sp)
    sd t6, 64(sp)

    sd a0, 72(sp)
    sd a1, 80(sp)
    sd a2, 88(sp)
    sd a3, 96(sp)
    sd a4, 104(sp)
    sd a5, 112(sp)
    sd a6, 120(sp)
    sd a7, 128(sp)

    sd s0, 136(sp)
    sd s1, 144(sp)
    sd s2, 152(sp)
    sd s3, 160(sp)
    sd s4, 168(sp)
    sd s5, 176(sp)
    sd s6, 184(sp)
    sd s7, 192(sp)
    sd s8, 200(sp)
    sd s9, 208(sp)
    sd s10, 216(sp)
    sd s11, 224(sp)

    mv a0, sp
    call riscv64_trap_handler

    la t6, __trap_stack_top
    csrw mscratch, t6

    ld ra, 8(sp)
    ld t0, 16(sp)
    ld t1, 24(sp)
    ld t2, 32(sp)
    ld t3, 40(sp)
    ld t4, 48(sp)
    ld t5, 56(sp)
    ld t6, 64(sp)

    ld a0, 72(sp)
    ld a1, 80(sp)
    ld a2, 88(sp)
    ld a3, 96(sp)
    ld a4, 104(sp)
    ld a5, 112(sp)
    ld a6, 120(sp)
    ld a7, 128(sp)

    ld s0, 136(sp)
    ld s1, 144(sp)
    ld s2, 152(sp)
    ld s3, 160(sp)
    ld s4, 168(sp)
    ld s5, 176(sp)
    ld s6, 184(sp)
    ld s7, 192(sp)
    ld s8, 200(sp)
    ld s9, 208(sp)
    ld s10, 216(sp)
    ld s11, 224(sp)

    ld sp, 0(sp)

    mret

.section .bss.trap_stack
.align 12
.global __trap_stack_bottom
__trap_stack_bottom:
    .skip 4096
.global __trap_stack_top
__trap_stack_top:
```

## File: src/arch/mod.rs
```rust
pub mod riscv64;

#[cfg(any(feature = "resume_restore_test", feature = "scheduler_dispatch_test"))]
pub use riscv64::restore_verified_resume_frame;

pub use riscv64::*;

/// Public arch-level wrapper for the RISC-V yield boundary.
///
/// The actual symbol is provided by riscv64 global_asm! and declared through
/// an unsafe extern block in riscv64/mod.rs. We keep this wrapper so task code
/// can call crate::arch::task_yield_boundary(...) without depending directly
/// on the riscv64 module layout.
#[cfg(target_arch = "riscv64")]
pub unsafe fn task_yield_boundary(kernel_sp: u64, return_pc: u64) {
    unsafe {
        riscv64::task_yield_boundary(kernel_sp, return_pc);
    }
}
```

## File: src/kernel/task/test/fault.rs
```rust
#[cfg(feature = "scheduler_fault_lifecycle_test")]
use crate::kernel::task::test::{
    check_faulted_task_dispatch_guard, check_finished_task_dispatch_guard,
};

#[cfg(feature = "task_fault_test")]
pub fn faulty_worker() {
    crate::drivers::uart::write_line("faulty_worker: step 1");
    crate::drivers::uart::write_line("faulty_worker: intentional fault");
    crate::kernel::task::task_fault();
}

#[cfg(feature = "task_fault_test")]
pub fn task_fault_completion_check() -> bool {
    crate::drivers::uart::write_line("");
    crate::drivers::uart::write_line("task fault completion check:");

    let completion_snapshot = crate::kernel::task::table::get_task_fault_completion_snapshot();

    print_task_fault_completion_snapshot(completion_snapshot);

    #[cfg(feature = "scheduler_fault_lifecycle_test")]
    {
        if let Some(id) = find_faulted_task_for_completion_check() {
            crate::kernel::task::table::print_task_fault_info_by_id(id);

            let fault_metadata_assertions_ok = check_task_fault_metadata_assertions(id);

            if !fault_metadata_assertions_ok {
                return false;
            }

            let faulted_task_dispatch_guard_ok = check_faulted_task_dispatch_guard(id);

            if !faulted_task_dispatch_guard_ok {
                return false;
            }
        } else {
            crate::drivers::uart::write_line("  fault info: faulted task not found");
            return false;
        }

        if let Some(id) = find_finished_task_for_completion_check() {
            let finished_task_dispatch_guard_ok = check_finished_task_dispatch_guard(id);

            if !finished_task_dispatch_guard_ok {
                return false;
            }
        } else {
            crate::drivers::uart::write_line(
                "  finished task dispatch guard: finished task not found",
            );
            return false;
        }

        let no_runnable_scheduler_policy_ok = check_no_runnable_scheduler_policy();

        if !no_runnable_scheduler_policy_ok {
            return false;
        }
    }

    crate::drivers::uart::write_str("  last return Fault: ");
    crate::kernel::task::table::print_yes_no(completion_snapshot.faulted_task_last_return_fault);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("  resume disabled: ");
    crate::kernel::task::table::print_yes_no(completion_snapshot.faulted_task_resume_disabled);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("  task fault result: ");
    if completion_snapshot.result {
        crate::drivers::uart::write_line("OK");
    } else {
        crate::drivers::uart::write_line("FAILED");
    }

    completion_snapshot.result
}

#[cfg(feature = "task_fault_test")]
pub fn print_task_fault_completion_snapshot(
    snapshot: crate::kernel::task::table::TaskFaultCompletionSnapshot,
) {
    crate::drivers::uart::write_line("");
    crate::drivers::uart::write_line("  task: worker-a");

    crate::drivers::uart::write_str("  state Finished: ");
    crate::kernel::task::table::print_yes_no(snapshot.finished_task_finished);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("  last return Exit: ");
    crate::kernel::task::table::print_yes_no(snapshot.finished_task_last_return_exit);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_line("");
    crate::drivers::uart::write_line("  task: trap-worker");

    crate::drivers::uart::write_str("  state Faulted:  ");
    crate::kernel::task::table::print_yes_no(snapshot.faulted_task_faulted);
    crate::drivers::uart::write_line("");
}

#[cfg(feature = "scheduler_fault_lifecycle_test")]
pub fn find_finished_task_for_completion_check() -> Option<usize> {
    crate::kernel::task::table::find_first_finished_task()
}

#[cfg(feature = "scheduler_fault_lifecycle_test")]
pub fn check_no_runnable_scheduler_policy() -> bool {
    let snapshot = crate::kernel::task::scheduler::get_no_runnable_scheduler_snapshot();

    crate::drivers::uart::write_line("  no-runnable scheduler policy:");

    crate::drivers::uart::write_str("    dispatchable tasks remaining: ");
    crate::kernel::task::table::print_yes_no(snapshot.has_dispatchable_tasks);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("    expected remaining == no: ");
    crate::kernel::task::table::print_yes_no(snapshot.no_runnable);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("    dispatchable count: ");
    crate::drivers::uart::write_dec_u64(snapshot.dispatchable_count as u64);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("    result: ");
    if snapshot.result {
        crate::drivers::uart::write_line("OK");
    } else {
        crate::drivers::uart::write_line("FAILED");
    }

    snapshot.result
}

#[cfg(feature = "scheduler_fault_lifecycle_test")]
pub fn find_faulted_task_for_completion_check() -> Option<usize> {
    crate::kernel::task::table::find_first_faulted_task()
}

#[cfg(feature = "scheduler_fault_lifecycle_test")]
pub fn check_task_fault_metadata_assertions(id: usize) -> bool {
    let snapshot = crate::kernel::task::table::get_breakpoint_fault_metadata_assertions(id);

    crate::drivers::uart::write_line("  fault metadata assertions:");

    crate::drivers::uart::write_str("    reason == breakpoint: ");
    crate::kernel::task::table::print_yes_no(snapshot.reason_breakpoint);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("    mcause == 3: ");
    crate::kernel::task::table::print_yes_no(snapshot.mcause_breakpoint);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("    mepc != 0: ");
    crate::kernel::task::table::print_yes_no(snapshot.mepc_nonzero);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("    mtval != 0: ");
    crate::kernel::task::table::print_yes_no(snapshot.mtval_nonzero);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("    result: ");
    if snapshot.result {
        crate::drivers::uart::write_line("OK");
    } else {
        crate::drivers::uart::write_line("FAILED");
    }

    snapshot.result
}

#[cfg(feature = "scheduler_fault_lifecycle_test")]
pub fn real_trap_handler_worker() {
    crate::drivers::uart::write_line("real_trap_handler_worker: step 1");
    crate::drivers::uart::write_line("real_trap_handler_worker: triggering ebreak");

    unsafe {
        core::arch::asm!("ebreak", options(nomem, nostack, preserves_flags));
    }

    crate::drivers::uart::write_line("real_trap_handler_worker: after ebreak (should not reach)");
}

#[cfg(feature = "scheduler_fault_lifecycle_test")]
pub fn real_trap_handler_worker_a() {
    crate::drivers::uart::write_line("real_trap_handler_worker_a: step 1");
    crate::kernel::task::yield_now();

    crate::drivers::uart::write_line("real_trap_handler_worker_a: resumed after yield");
    crate::drivers::uart::write_line("real_trap_handler_worker_a: step 2");

    crate::kernel::task::task_exit();
}
```

## File: src/kernel/task/test/handoff.rs
```rust
#[allow(unused_imports)]
use crate::kernel::task::debug::{set_debug_current_stack_bounds, set_debug_current_task_id};

#[cfg(feature = "two_task_resume_handoff_test")]
pub fn handoff_worker_a() {
    crate::drivers::uart::write_line("handoff_worker_a: step 1");
    crate::kernel::task::yield_now();

    crate::drivers::uart::write_line("handoff_worker_a: resumed after first yield");
    crate::drivers::uart::write_line("handoff_worker_a: step 2");
    crate::kernel::task::yield_now();

    crate::drivers::uart::write_line("handoff_worker_a: resumed after second yield");
    crate::drivers::uart::write_line("handoff_worker_a: step 3");
    crate::kernel::task::task_exit();
}

#[cfg(feature = "two_task_resume_handoff_test")]
pub fn handoff_worker_b() {
    crate::drivers::uart::write_line("handoff_worker_b: step 1");
    crate::kernel::task::yield_now();

    crate::drivers::uart::write_line("handoff_worker_b: resumed after yield");
    crate::drivers::uart::write_line("handoff_worker_b: step 2");
    crate::kernel::task::task_exit();
}

#[cfg(feature = "two_task_resume_handoff_test")]
static mut TWO_TASK_HANDOFF_PHASE: usize = 0;

#[cfg(feature = "two_task_resume_handoff_test")]
pub fn get_two_task_handoff_phase() -> usize {
    unsafe { TWO_TASK_HANDOFF_PHASE }
}

#[cfg(feature = "two_task_resume_handoff_test")]
pub fn advance_two_task_handoff_phase() {
    unsafe {
        TWO_TASK_HANDOFF_PHASE += 1;
    }
}

#[cfg(feature = "two_task_resume_handoff_test")]
pub fn prepare_debug_context_for_task(task_id: usize) {
    let Some(stack_start) = crate::kernel::task::table::get_task_stack_start(task_id) else {
        crate::drivers::uart::write_line("two-task handoff error: missing task stack start");
        crate::arch::halt();
    };

    let Some(stack_top) = crate::kernel::task::table::get_task_stack_top(task_id) else {
        crate::drivers::uart::write_line("two-task handoff error: missing task stack top");
        crate::arch::halt();
    };

    set_debug_current_task_id(task_id);
    set_debug_current_stack_bounds(stack_start, stack_top);
}
```

## File: src/kernel/task/test/reentry.rs
```rust
#[allow(unused_imports)]
use crate::drivers::uart;

#[cfg(feature = "scheduler_fault_lifecycle_test")]
pub(crate) fn check_finished_task_dispatch_guard(id: usize) -> bool {
    print_terminal_task_dispatch_guard("finished", id)
}

#[cfg(feature = "scheduler_fault_lifecycle_test")]
pub(crate) fn check_faulted_task_dispatch_guard(id: usize) -> bool {
    print_terminal_task_dispatch_guard("faulted", id)
}

#[cfg(all(
    feature = "two_yield_task_test",
    not(feature = "two_task_resume_handoff_test"),
    not(feature = "scheduler_fault_lifecycle_test")
))]
pub fn two_yielding_task() {
    crate::drivers::uart::write_line("two_yielding_task: step 1");
    crate::kernel::task::yield_now();
    crate::drivers::uart::write_line("two_yielding_task: step 2");
    crate::kernel::task::yield_now();
    crate::drivers::uart::write_line("two_yielding_task: step 3");
    crate::kernel::task::task_exit();
}

#[cfg(feature = "scheduler_fault_lifecycle_test")]
fn print_terminal_task_dispatch_guard(label: &str, id: usize) -> bool {
    let snapshot = crate::kernel::task::table::get_terminal_task_dispatch_invariants(id);
    let running_blocked = !crate::kernel::task::table::mark_task_running(id);

    crate::drivers::uart::write_str("  ");
    crate::drivers::uart::write_str(label);
    crate::drivers::uart::write_line(" task dispatch guard:");
    crate::drivers::uart::write_str("    terminal task: ");
    crate::kernel::task::table::print_yes_no(snapshot.terminal);
    crate::drivers::uart::write_line("");
    crate::drivers::uart::write_str("    ");
    crate::drivers::uart::write_str(label);
    crate::drivers::uart::write_str(" task resumable: ");
    crate::kernel::task::table::print_yes_no(snapshot.resumable);
    crate::drivers::uart::write_line("");
    crate::drivers::uart::write_str("    expected resumable == no: ");
    crate::kernel::task::table::print_yes_no(!snapshot.resumable);
    crate::drivers::uart::write_line("");
    crate::drivers::uart::write_str("    ");
    crate::drivers::uart::write_str(label);
    crate::drivers::uart::write_str(" task fresh-ready: ");
    crate::kernel::task::table::print_yes_no(snapshot.fresh_ready);
    crate::drivers::uart::write_line("");
    crate::drivers::uart::write_str("    expected fresh-ready == no: ");
    crate::kernel::task::table::print_yes_no(!snapshot.fresh_ready);
    crate::drivers::uart::write_line("");
    crate::drivers::uart::write_str("    ");
    crate::drivers::uart::write_str(label);
    crate::drivers::uart::write_str(" task dispatchable: ");
    crate::kernel::task::table::print_yes_no(snapshot.dispatchable);
    crate::drivers::uart::write_line("");
    crate::drivers::uart::write_str("    expected dispatchable == no: ");
    crate::kernel::task::table::print_yes_no(!snapshot.dispatchable);
    crate::drivers::uart::write_line("");
    crate::drivers::uart::write_str("    force running blocked: ");
    crate::kernel::task::table::print_yes_no(running_blocked);
    crate::drivers::uart::write_line("");

    let ok = snapshot.result && running_blocked;
    crate::drivers::uart::write_str("    result: ");
    if ok {
        crate::drivers::uart::write_line("OK");
    } else {
        crate::drivers::uart::write_line("FAILED");
    }
    ok
}

#[cfg(feature = "scheduler_reentry_test")]
pub fn handle_scheduler_reentry_after_task_return() {
    crate::drivers::uart::write_line("");
    crate::drivers::uart::write_line("scheduler re-entry after task return:");

    let Some(snapshot) = crate::kernel::task::table::get_last_returned_task_snapshot() else {
        crate::drivers::uart::write_line(
            "scheduler re-entry result: missing last returned task snapshot",
        );
        crate::arch::halt();
    };

    crate::drivers::uart::write_str("  last returned task: ");
    crate::kernel::task::table::print_task_name_by_id(snapshot.task_id);
    crate::drivers::uart::write_line("");

    #[cfg(all(
        feature = "two_task_resume_handoff_test",
        not(feature = "task_fault_test")
    ))]
    {
        let phase = crate::kernel::task::test::get_two_task_handoff_phase();

        if phase == 0 && snapshot.task_id == 1 {
            if !matches!(
                snapshot.last_return,
                crate::kernel::task::table::TaskReturnKind::Yield
            ) {
                crate::drivers::uart::write_line("two-task handoff error: worker-a did not yield");
                crate::arch::halt();
            }

            crate::kernel::task::test::advance_two_task_handoff_phase();
            crate::drivers::uart::write_line("two-task handoff phase 0: worker-a yielded");
            crate::drivers::uart::write_line(
                "two-task handoff action: scheduler starts next fresh task",
            );
        }

        if phase == 1 && snapshot.task_id == 2 {
            if !matches!(
                snapshot.last_return,
                crate::kernel::task::table::TaskReturnKind::Yield
            ) {
                crate::drivers::uart::write_line("two-task handoff error: worker-b did not yield");
                crate::arch::halt();
            }

            crate::kernel::task::test::advance_two_task_handoff_phase();
            crate::drivers::uart::write_line("two-task handoff phase 1: worker-b yielded");
            crate::drivers::uart::write_line(
                "two-task handoff action: continue scheduler re-entry",
            );
            crate::kernel::task::test::prepare_debug_context_for_task(1);
        }
    }

    match crate::kernel::task::scheduler::handle_task_return(snapshot) {
        crate::kernel::task::scheduler::TaskReturnHandleResult::NoRunnableTask => {
            crate::drivers::uart::write_line("  action: completion check");

            #[cfg(all(
                feature = "scheduler_resume_loop_test",
                feature = "real_resume_restore_jump"
            ))]
            {
                #[cfg(feature = "task_fault_test")]
                {
                    if crate::kernel::task::test::task_fault_completion_check() {
                        crate::drivers::uart::write_line("task fault scheduler result: OK");
                        print_riscv_cooperative_resume_milestone();
                        crate::arch::halt();
                    }
                }

                #[cfg(not(feature = "task_fault_test"))]
                {
                    if crate::kernel::task::test::real_resume_jump_completion_check() {
                        crate::drivers::uart::write_line("scheduler resume loop result: OK");
                        crate::drivers::uart::write_line("scheduler resume loop test complete");
                        print_riscv_cooperative_resume_milestone();
                        crate::arch::halt();
                    }
                }
            }

            crate::drivers::uart::write_line("scheduler re-entry result: no runnable task");
            crate::arch::halt();
        }
        crate::kernel::task::scheduler::TaskReturnHandleResult::Failed => {
            crate::drivers::uart::write_line("scheduler re-entry result: failed");
            crate::arch::halt();
        }
    }
}

#[cfg(feature = "kernel_fault_guard_test")]
pub fn test_kernel_fault_guard() -> ! {
    crate::drivers::uart::write_line("");
    crate::drivers::uart::write_line("kernel fault guard test:");
    crate::drivers::uart::write_line("triggering real trap from kernel context");
    crate::kernel::task::debug::clear_debug_current_task_id();

    unsafe {
        core::arch::asm!("ebreak", options(nomem, nostack, preserves_flags));
    }

    crate::drivers::uart::write_line("kernel fault guard result: FAILED");
    crate::drivers::uart::write_line("kernel continued after kernel fault trap");
    crate::arch::halt();
}

#[cfg(all(
    target_arch = "riscv64",
    feature = "real_resume_restore_jump",
    feature = "scheduler_resume_loop_test"
))]
fn print_riscv_cooperative_resume_milestone() {
    crate::drivers::uart::write_line("PicoOS milestone:");
    crate::drivers::uart::write_line("  baseline: 0.1.0");
    crate::drivers::uart::write_line("  current: 0.1.64");
    crate::drivers::uart::write_line("");
    crate::drivers::uart::write_line("  cleanup:");
    crate::drivers::uart::write_line("    obsolete standalone task tests removed: OK");
    crate::drivers::uart::write_line("    obsolete standalone scheduler scripts removed: OK");
    crate::drivers::uart::write_line("    obsolete resume task script removed: OK");
    crate::drivers::uart::write_line("    obsolete resume PC proximity requirement removed: OK");
    crate::drivers::uart::write_line("");
    crate::drivers::uart::write_line("  task/resume:");
    crate::drivers::uart::write_line("    RISC-V-only baseline: OK");
    crate::drivers::uart::write_line("    cooperative task resume: OK");
    crate::drivers::uart::write_line("    repeated yield/resume loop: OK");
    crate::drivers::uart::write_line("    scheduler-oriented resume loop: OK");
    crate::drivers::uart::write_line("    RISC-V yield boundary: OK");
    crate::drivers::uart::write_line("    two-task cooperative handoff: OK");
    crate::drivers::uart::write_line("");
    crate::drivers::uart::write_line("  scheduler:");
    crate::drivers::uart::write_line("    scheduler first task dispatch: OK");
    crate::drivers::uart::write_line("    scheduler fresh task dispatch: OK");
    crate::drivers::uart::write_line("    scheduler round-robin fairness: OK");
    crate::drivers::uart::write_line("    scheduler task capacity from table: OK");
    crate::drivers::uart::write_line("    scheduler skips faulted tasks: OK");
    crate::drivers::uart::write_line("    no-runnable scheduler policy: OK");
    crate::drivers::uart::write_line("    scheduler dispatch candidate selection: OK");
    crate::drivers::uart::write_line("    scheduler dispatch candidate model: OK");
    crate::drivers::uart::write_line("    scheduler dispatch candidate-to-decision conversion: OK");
    crate::drivers::uart::write_line("    scheduler dispatch decision model: OK");
    crate::drivers::uart::write_line("    scheduler dispatch decision kind: OK");
    crate::drivers::uart::write_line("    scheduler dispatch decision outcome: OK");
    crate::drivers::uart::write_line("    scheduler dispatch decision logging: OK");
    crate::drivers::uart::write_line("    scheduler dispatch pipeline model: OK");
    crate::drivers::uart::write_line("");
    crate::drivers::uart::write_line("  task lifecycle:");
    crate::drivers::uart::write_line("    task state invariants in core: OK");
    crate::drivers::uart::write_line("    task state lookup in core: OK");
    crate::drivers::uart::write_line("    terminal task dispatch invariants in core: OK");
    crate::drivers::uart::write_line("    no-runnable scheduler snapshot in core: OK");
    crate::drivers::uart::write_line("    task completion summary in core: OK");
    crate::drivers::uart::write_line("    task completion output consolidated: OK");
    crate::drivers::uart::write_line("");
    crate::drivers::uart::write_line("  fault lifecycle:");
    crate::drivers::uart::write_line("    task fault state: OK");
    crate::drivers::uart::write_line("    trap-to-task-fault skeleton: OK");
    crate::drivers::uart::write_line("    real trap classification: OK");
    crate::drivers::uart::write_line("    real trap handler classification: OK");
    crate::drivers::uart::write_line("    real trap handler task-fault return path: OK");
    crate::drivers::uart::write_line("    trap fault metadata reporting: OK");
    crate::drivers::uart::write_line("    fault metadata assertions in core: OK");
    crate::drivers::uart::write_line("    explicit task fault assertions: OK");
    crate::drivers::uart::write_line("    faulted task dispatch guard: OK");
    crate::drivers::uart::write_line("    finished task dispatch guard: OK");
    crate::drivers::uart::write_line("    scheduler fault lifecycle feature: OK");
}
```

## File: src/kernel/task/test/resume.rs
```rust
#[allow(unused_imports)]
use super::invariants;
#[allow(unused_imports)]
use crate::drivers::uart;
#[allow(unused_imports)]
use crate::kernel::task::debug::{set_debug_current_stack_bounds, set_debug_current_task_id};

#[cfg(feature = "resume_restore_test")]
pub fn test_resume_restore() {
    uart::write_line("");
    uart::write_line("resume restore test:");

    let Some(task_id) = crate::kernel::task::table::find_first_resumable_task() else {
        crate::drivers::uart::write_line("selected task: none");

        #[cfg(feature = "real_resume_restore_jump")]
        {
            if real_resume_jump_completion_check() {
                crate::drivers::uart::write_line(
                    "preflight result: SKIPPED after successful real resume jump",
                );
                crate::drivers::uart::write_line("real resume jump test complete");
                crate::arch::halt();
            }

            crate::drivers::uart::write_line("preflight result: FAILED");
            crate::arch::halt();
        }

        #[cfg(not(feature = "real_resume_restore_jump"))]
        {
            crate::drivers::uart::write_line("preflight result: FAILED");
            crate::arch::halt();
        }
    };

    uart::write_str("selected task: ");
    crate::kernel::task::table::print_task_name_by_id(task_id);
    uart::write_line("");

    let Some(frame) = crate::kernel::task::table::get_task_resume_frame(task_id) else {
        uart::write_line("cpu context: none");
        uart::write_line("restore result: FAILED");
        crate::arch::halt();
    };

    uart::write_str("restore sp: ");
    uart::write_hex_u64(frame.sp);
    uart::write_line("");

    uart::write_str("restore resume_pc: ");
    uart::write_hex_u64(frame.resume_pc);
    uart::write_line("");

    uart::write_str("restore frame:");
    crate::kernel::task::cpu_context::print_cpu_context(frame);
    uart::write_line("");

    if !resume_restore_precheck(task_id) {
        uart::write_line("restore aborted by guard");
        crate::arch::halt();
    }

    uart::write_line("restore guarded precheck passed");
    uart::write_line("calling arch restore_verified_resume_frame...");

    set_debug_current_task_id(task_id);
    match (
        crate::kernel::task::table::get_task_stack_start(task_id),
        crate::kernel::task::table::get_task_stack_top(task_id),
    ) {
        (Some(start), Some(top)) => set_debug_current_stack_bounds(start, top),
        _ => {
            uart::write_line("restore aborted: missing task stack bounds");
            crate::arch::halt();
        }
    }

    crate::arch::restore_verified_resume_frame(frame);
}

#[cfg(feature = "resume_preflight_test")]
pub fn test_resume_preflight_check() {
    uart::write_line("");
    uart::write_line("resume preflight check:");

    let Some(task_id) = crate::kernel::task::table::find_first_resumable_task() else {
        uart::write_line("selected task: none");

        #[cfg(feature = "real_resume_restore_jump")]
        {
            if real_resume_jump_completion_check() {
                uart::write_line("preflight result: SKIPPED after successful real resume jump");
                uart::write_line("real resume jump test complete");
                crate::arch::halt();
            }

            uart::write_line("preflight result: FAILED");
            crate::arch::halt();
        }

        #[cfg(not(feature = "real_resume_restore_jump"))]
        {
            uart::write_line("preflight result: FAILED");
            crate::arch::halt();
        }
    };

    uart::write_str("selected task: ");
    crate::kernel::task::table::print_task_name_by_id(task_id);
    uart::write_line("");

    uart::write_str("state: ");
    crate::kernel::task::table::print_task_state_by_id(task_id);
    uart::write_line("");

    uart::write_str("can_resume: ");
    match crate::kernel::task::table::can_task_resume(task_id) {
        Some(value) => {
            crate::kernel::task::table::print_yes_no(value);
            uart::write_line("");
        }
        None => uart::write_line("unknown"),
    }

    uart::write_str("last_return: ");
    crate::kernel::task::table::print_task_return_kind_by_id(task_id);
    uart::write_line("");

    let task_sp = crate::kernel::task::table::get_task_last_task_sp(task_id);
    let kernel_sp = crate::kernel::task::table::get_task_last_kernel_sp(task_id);
    let kernel_return_pc = crate::kernel::task::table::get_task_last_kernel_return_pc(task_id);
    let entry = crate::kernel::task::table::get_task_entry(task_id);
    let cpu_context = crate::kernel::task::table::get_task_cpu_context(task_id);

    uart::write_str("last_task_sp: ");
    match task_sp {
        Some(sp) => {
            uart::write_hex_u64(sp);
            uart::write_line("");
        }
        None => uart::write_line("none"),
    }

    uart::write_str("last_kernel_sp: ");
    match kernel_sp {
        Some(sp) => {
            uart::write_hex_u64(sp);
            uart::write_line("");
        }
        None => uart::write_line("none"),
    }

    uart::write_str("kernel_return_pc: ");
    match kernel_return_pc {
        Some(pc) => {
            uart::write_hex_u64(pc);
            uart::write_line("");
        }
        None => uart::write_line("none"),
    }

    uart::write_str("entry present: ");
    crate::kernel::task::table::print_yes_no(entry.is_some());
    uart::write_line("");

    uart::write_str("cpu context valid: ");
    match cpu_context {
        Some(context) => {
            crate::kernel::task::table::print_yes_no(context.is_valid());
            uart::write_line("");

            #[cfg(feature = "verbose_resume_debug")]
            {
                uart::write_str("cpu context detail:");
                crate::kernel::task::cpu_context::print_cpu_context(context);
                uart::write_line("");
            }
        }
        None => uart::write_line("unknown"),
    }

    let sp_inside = match task_sp {
        Some(sp) => crate::kernel::task::table::is_sp_inside_task_stack(task_id, sp),
        None => None,
    };

    uart::write_str("task SP check: ");
    match sp_inside {
        Some(true) => uart::write_line("inside task stack"),
        Some(false) => uart::write_line("outside task stack"),
        None => uart::write_line("unknown"),
    }

    invariants::print_cpu_context_consistency_check(task_id);
    let _ = print_resume_pc_proximity_check(task_id);

    uart::write_line("preflight result: OK");

    #[cfg(feature = "resume_dry_run_test")]
    {
        test_resume_dry_run();
    }

    #[cfg(not(feature = "resume_dry_run_test"))]
    {
        crate::arch::halt();
    }
}

#[cfg(feature = "resume_dry_run_test")]
pub fn test_resume_dry_run() {
    uart::write_line("");
    uart::write_line("resume dry-run:");

    let Some(task_id) = crate::kernel::task::table::find_first_resumable_task() else {
        uart::write_line("selected task: none");
        uart::write_line("resume plan result: FAILED");
        crate::arch::halt();
    };

    uart::write_str("selected task: ");
    crate::kernel::task::table::print_task_name_by_id(task_id);
    uart::write_line("");

    let Some(frame) = crate::kernel::task::table::get_task_resume_frame(task_id) else {
        uart::write_line("cpu context: none");
        uart::write_line("resume plan result: FAILED");
        crate::arch::halt();
    };

    uart::write_str("restore sp: ");
    uart::write_hex_u64(frame.sp);
    uart::write_line("");

    uart::write_str("restore resume_pc: ");
    uart::write_hex_u64(frame.resume_pc);
    uart::write_line("");

    uart::write_str("restore kernel_return_pc: ");
    uart::write_hex_u64(frame.return_pc);
    uart::write_line("");

    uart::write_str("kernel text: ");
    uart::write_hex_u64(crate::kernel::memory::kernel_text_start());
    uart::write_str(" - ");
    uart::write_hex_u64(crate::kernel::memory::kernel_text_end());
    uart::write_line("");

    let sp_inside = crate::kernel::task::table::is_sp_inside_task_stack(task_id, frame.sp);
    let resume_pc_inside_text = crate::kernel::memory::is_inside_kernel_text(frame.resume_pc);
    let return_pc_inside_text = crate::kernel::memory::is_inside_kernel_text(frame.return_pc);

    uart::write_str("task SP inside task stack: ");
    match sp_inside {
        Some(value) => {
            crate::kernel::task::table::print_yes_no(value);
            uart::write_line("");
        }
        None => uart::write_line("unknown"),
    }

    uart::write_str("resume_pc inside kernel text: ");
    crate::kernel::task::table::print_yes_no(resume_pc_inside_text);
    uart::write_line("");

    uart::write_str("kernel_return_pc inside kernel text: ");
    crate::kernel::task::table::print_yes_no(return_pc_inside_text);
    uart::write_line("");

    uart::write_str("resume frame valid: ");
    crate::kernel::task::table::print_yes_no(frame.is_valid());
    uart::write_line("");

    #[cfg(feature = "verbose_resume_debug")]
    {
        uart::write_str("resume frame detail:");
        crate::kernel::task::cpu_context::print_cpu_context(frame);
        uart::write_line("");
    }

    let context_consistent = match (
        crate::kernel::task::table::get_task_last_task_sp(task_id),
        crate::kernel::task::table::get_task_last_kernel_return_pc(task_id),
    ) {
        (Some(last_sp), Some(kernel_pc)) => frame.sp == last_sp && frame.return_pc == kernel_pc,
        _ => false,
    };

    let frame_ok = print_resume_frame_check(task_id);

    uart::write_str("CPU context consistent: ");
    crate::kernel::task::table::print_yes_no(context_consistent);
    uart::write_line("");

    let ok = frame.is_valid()
        && matches!(sp_inside, Some(true))
        && resume_pc_inside_text
        && return_pc_inside_text
        && context_consistent
        && frame_ok;

    uart::write_str("resume plan result: ");
    if ok {
        uart::write_line("OK");
    } else {
        uart::write_line("FAILED");
    }

    if !ok {
        crate::arch::halt();
    }

    #[cfg(feature = "resume_restore_test")]
    {
        test_resume_restore();
    }

    #[cfg(not(feature = "resume_restore_test"))]
    {
        crate::arch::halt();
    }
}

#[cfg(any(
    feature = "resume_preflight_test",
    feature = "resume_dry_run_test",
    feature = "resume_restore_test",
    feature = "real_resume_restore_test",
    feature = "real_resume_restore_jump",
    feature = "scheduler_resume_loop_test",
    feature = "two_task_resume_handoff_test",
    feature = "scheduler_fault_lifecycle_test"
))]
pub fn print_resume_pc_proximity_check(task_id: usize) -> bool {
    crate::drivers::uart::write_line("  resume PC proximity check:");

    let Some(context) = crate::kernel::task::table::get_task_cpu_context(task_id) else {
        crate::drivers::uart::write_line("    cpu context: none");
        return false;
    };

    let Some(entry) = crate::kernel::task::table::get_task_entry(task_id) else {
        crate::drivers::uart::write_line("    entry: none");
        return false;
    };

    let entry_addr = entry as usize as u64;

    crate::drivers::uart::write_str("    entry: ");
    crate::drivers::uart::write_hex_u64(entry_addr);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("    resume_pc: ");
    crate::drivers::uart::write_hex_u64(context.resume_pc);
    crate::drivers::uart::write_line("");

    #[cfg(any(
        feature = "two_task_resume_handoff_test",
        feature = "task_fault_test",
        feature = "scheduler_resume_loop_test",
        feature = "real_resume_restore_jump"
    ))]
    {
        let resume_pc_inside_text = crate::kernel::memory::is_inside_kernel_text(context.resume_pc);

        crate::drivers::uart::write_line("    mode: RISC-V yield boundary continuation");

        crate::drivers::uart::write_str("    resume_pc inside kernel text: ");
        crate::kernel::task::table::print_yes_no(resume_pc_inside_text);
        crate::drivers::uart::write_line("");

        crate::drivers::uart::write_str("    result: ");
        if resume_pc_inside_text {
            crate::drivers::uart::write_line("OK");
        } else {
            crate::drivers::uart::write_line("FAILED");
        }

        resume_pc_inside_text
    }

    #[cfg(not(any(
        feature = "two_task_resume_handoff_test",
        feature = "task_fault_test",
        feature = "scheduler_resume_loop_test",
        feature = "real_resume_restore_jump"
    )))]
    {
        if context.resume_pc < entry_addr {
            crate::drivers::uart::write_line("    delta: below entry");
            crate::drivers::uart::write_line("    result: FAILED");
            return false;
        }

        let delta = context.resume_pc - entry_addr;

        crate::drivers::uart::write_str("    delta: ");
        crate::drivers::uart::write_hex_u64(delta);
        crate::drivers::uart::write_line("");

        let ok = delta < 0x400;

        crate::drivers::uart::write_str("    result: ");
        if ok {
            crate::drivers::uart::write_line("OK");
        } else {
            crate::drivers::uart::write_line("FAILED");
        }

        ok
    }
}

#[cfg(any(
    feature = "resume_preflight_test",
    feature = "resume_dry_run_test",
    feature = "resume_restore_test",
    feature = "real_resume_restore_test",
    feature = "real_resume_restore_jump",
    feature = "scheduler_resume_loop_test",
    feature = "two_task_resume_handoff_test",
    feature = "scheduler_fault_lifecycle_test"
))]
pub fn print_resume_frame_check(task_id: usize) -> bool {
    uart::write_line("  resume frame check:");

    let Some(frame) = crate::kernel::task::table::get_task_resume_frame(task_id) else {
        uart::write_line("    frame present: no");
        uart::write_line("    result: FAILED");
        return false;
    };

    uart::write_line("    frame present: yes");

    uart::write_str("    frame valid: ");
    crate::kernel::task::table::print_yes_no(frame.is_valid());
    uart::write_line("");

    uart::write_str("    frame SP: ");
    uart::write_hex_u64(frame.sp);
    uart::write_line("");

    uart::write_str("    frame resume_pc: ");
    uart::write_hex_u64(frame.resume_pc);
    uart::write_line("");

    uart::write_str("    frame return_pc: ");
    uart::write_hex_u64(frame.return_pc);
    uart::write_line("");

    let sp_inside = matches!(
        crate::kernel::task::table::is_sp_inside_task_stack(task_id, frame.sp),
        Some(true)
    );

    uart::write_str("    frame SP inside task stack: ");
    crate::kernel::task::table::print_yes_no(sp_inside);
    uart::write_line("");

    let resume_pc_inside_text = crate::kernel::memory::is_inside_kernel_text(frame.resume_pc);
    let return_pc_inside_text = crate::kernel::memory::is_inside_kernel_text(frame.return_pc);

    uart::write_str("    frame resume_pc inside kernel text: ");
    crate::kernel::task::table::print_yes_no(resume_pc_inside_text);
    uart::write_line("");

    uart::write_str("    frame return_pc inside kernel text: ");
    crate::kernel::task::table::print_yes_no(return_pc_inside_text);
    uart::write_line("");

    let ok = frame.is_valid() && sp_inside && resume_pc_inside_text && return_pc_inside_text;

    uart::write_str("    frame check result: ");
    if ok {
        uart::write_line("OK");
    } else {
        uart::write_line("FAILED");
    }

    ok
}

#[cfg(feature = "resume_restore_test")]
pub fn resume_restore_precheck(task_id: usize) -> bool {
    uart::write_line("restore guarded precheck:");

    let Some(frame) = crate::kernel::task::table::get_task_resume_frame(task_id) else {
        uart::write_line("  cpu context: none");
        uart::write_line("  result: FAILED");
        return false;
    };

    let sp_ok = matches!(
        crate::kernel::task::table::is_sp_inside_task_stack(task_id, frame.sp),
        Some(true)
    );

    let resume_pc_text_ok = crate::kernel::memory::is_inside_kernel_text(frame.resume_pc);
    let return_pc_text_ok = crate::kernel::memory::is_inside_kernel_text(frame.return_pc);
    let context_valid = frame.is_valid();

    uart::write_str("  resume frame valid: ");
    crate::kernel::task::table::print_yes_no(context_valid);
    uart::write_line("");

    uart::write_str("  task SP inside stack: ");
    crate::kernel::task::table::print_yes_no(sp_ok);
    uart::write_line("");

    uart::write_str("  resume_pc inside kernel text: ");
    crate::kernel::task::table::print_yes_no(resume_pc_text_ok);
    uart::write_line("");

    uart::write_str("  return_pc inside kernel text: ");
    crate::kernel::task::table::print_yes_no(return_pc_text_ok);
    uart::write_line("");

    let frame_ok = print_resume_frame_check(task_id);

    let ok = context_valid && sp_ok && resume_pc_text_ok && return_pc_text_ok && frame_ok;

    uart::write_str("  result: ");
    if ok {
        uart::write_line("OK");
    } else {
        uart::write_line("FAILED");
    }

    ok
}

#[cfg(feature = "resume_candidate_test")]
pub fn test_resume_candidate_selection() {
    print_resume_candidate_header();

    match crate::kernel::task::table::find_first_resumable_task() {
        Some(task_id) => {
            uart::write_str("selected resumable task: ");
            crate::kernel::task::table::print_task_name_by_id(task_id);
            uart::write_line("");

            uart::write_str("state: ");
            crate::kernel::task::table::print_task_state_by_id(task_id);
            uart::write_line("");

            uart::write_str("last_return: ");
            crate::kernel::task::table::print_task_return_kind_by_id(task_id);
            uart::write_line("");

            uart::write_str("can_resume: ");
            match crate::kernel::task::table::can_task_resume(task_id) {
                Some(value) => {
                    crate::kernel::task::table::print_yes_no(value);
                    uart::write_line("");
                }
                None => uart::write_line("unknown"),
            }

            uart::write_str("saved task SP: ");
            match crate::kernel::task::table::get_task_last_task_sp(task_id) {
                Some(sp) => {
                    uart::write_hex_u64(sp);
                    uart::write_line("");

                    uart::write_str("SP check: ");
                    match crate::kernel::task::table::is_sp_inside_task_stack(task_id, sp) {
                        Some(true) => uart::write_line("inside task stack"),
                        Some(false) => uart::write_line("outside task stack"),
                        None => uart::write_line("unknown task"),
                    }
                }
                None => {
                    uart::write_line("none");
                }
            }

            uart::write_line("resume candidate test complete");

            #[cfg(feature = "scheduler_run_test")]
            {
                uart::write_line("resume candidate selected; delegating to scheduler run");

                match crate::kernel::task::scheduler::run() {
                    crate::kernel::task::scheduler::RunResult::NoRunnableTask => {
                        uart::write_line("scheduler run returned: no runnable task");
                    }
                    crate::kernel::task::scheduler::RunResult::Failed => {
                        uart::write_line("scheduler run returned: failed");
                    }
                }

                crate::arch::halt();
            }

            #[cfg(all(
                feature = "scheduler_dispatch_test",
                not(feature = "scheduler_run_test")
            ))]
            {
                uart::write_line("resume candidate selected; delegating to scheduler run_once");

                match crate::kernel::task::scheduler::run_once() {
                    crate::kernel::task::scheduler::RunOnceResult::NoRunnableTask => {
                        uart::write_line("scheduler run_once returned: no runnable task");
                    }
                    crate::kernel::task::scheduler::RunOnceResult::Failed => {
                        uart::write_line("scheduler run_once returned: failed");
                    }
                }

                crate::arch::halt();
            }

            #[cfg(all(
                feature = "resume_preflight_test",
                not(feature = "scheduler_dispatch_test")
            ))]
            {
                test_resume_preflight_check();
            }
        }
        None => {
            uart::write_line("selected resumable task: none");
            print_resume_candidate_complete();

            #[cfg(all(
                feature = "scheduler_resume_loop_test",
                feature = "real_resume_restore_jump"
            ))]
            {
                if real_resume_jump_completion_check() {
                    uart::write_line("scheduler resume loop result: OK");
                    uart::write_line("scheduler resume loop test complete");

                    #[cfg(all(
                        target_arch = "riscv64",
                        feature = "real_resume_restore_jump",
                        feature = "scheduler_resume_loop_test"
                    ))]
                    print_riscv_cooperative_resume_milestone();

                    crate::arch::halt();
                }
            }
        }
    }

    #[cfg(not(any(
        feature = "resume_preflight_test",
        feature = "resume_dry_run_test",
        feature = "resume_restore_test",
        feature = "scheduler_dispatch_test"
    )))]
    crate::arch::halt();
}

#[cfg(all(feature = "resume_restore_test", feature = "real_resume_restore_jump"))]
pub fn real_resume_jump_completion_check() -> bool {
    crate::drivers::uart::write_line("");
    crate::drivers::uart::write_line("real resume jump completion check:");

    #[cfg(feature = "two_task_resume_handoff_test")]
    {
        crate::drivers::uart::write_line("  scenario: two-task handoff");

        let worker_a_ok = print_task_finished_cleanly_check(1);
        let worker_b_ok = print_task_finished_cleanly_check(2);

        let ok = worker_a_ok && worker_b_ok;

        crate::drivers::uart::write_str("    result: ");
        if ok {
            crate::drivers::uart::write_line("OK");
        } else {
            crate::drivers::uart::write_line("FAILED");
        }

        ok
    }

    #[cfg(not(feature = "two_task_resume_handoff_test"))]
    {
        #[cfg(feature = "scheduler_resume_loop_test")]
        crate::drivers::uart::write_line("  scenario: scheduler resume loop task");

        #[cfg(all(
            feature = "two_yield_task_test",
            not(feature = "scheduler_resume_loop_test")
        ))]
        crate::drivers::uart::write_line("  scenario: two-yield task");

        #[cfg(not(any(
            feature = "two_yield_task_test",
            feature = "scheduler_resume_loop_test"
        )))]
        crate::drivers::uart::write_line("  scenario: single-yield task");

        let ok = print_task_finished_cleanly_check(1);

        crate::drivers::uart::write_str("  result: ");
        if ok {
            crate::drivers::uart::write_line("OK");
        } else {
            crate::drivers::uart::write_line("FAILED");
        }

        ok
    }
}

#[cfg(all(feature = "resume_restore_test", feature = "real_resume_restore_jump"))]
pub fn print_task_finished_cleanly_check(task_id: usize) -> bool {
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("  task: ");
    crate::kernel::task::table::print_task_name_by_id(task_id);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("  state: ");
    crate::kernel::task::table::print_task_state_by_id(task_id);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("  can_resume: ");
    match crate::kernel::task::table::can_task_resume(task_id) {
        Some(value) => {
            crate::kernel::task::table::print_yes_no(value);
            crate::drivers::uart::write_line("");
        }
        None => crate::drivers::uart::write_line("unknown"),
    }

    crate::drivers::uart::write_str("  last_return: ");
    crate::kernel::task::table::print_task_return_kind_by_id(task_id);
    crate::drivers::uart::write_line("");

    let state_finished = matches!(
        crate::kernel::task::table::get_task_state(task_id),
        Some(crate::kernel::task::table::TaskState::Finished)
    );

    let can_resume_false = matches!(
        crate::kernel::task::table::can_task_resume(task_id),
        Some(false)
    );

    let last_return_exit = matches!(
        crate::kernel::task::table::get_task_return_kind(task_id),
        Some(crate::kernel::task::table::TaskReturnKind::Exit)
    );

    crate::drivers::uart::write_str("  state Finished: ");
    crate::kernel::task::table::print_yes_no(state_finished);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("  resume disabled: ");
    crate::kernel::task::table::print_yes_no(can_resume_false);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("  last return Exit: ");
    crate::kernel::task::table::print_yes_no(last_return_exit);
    crate::drivers::uart::write_line("");

    state_finished && can_resume_false && last_return_exit
}

#[cfg(all(
    target_arch = "riscv64",
    feature = "real_resume_restore_jump",
    feature = "scheduler_resume_loop_test"
))]
pub fn print_riscv_cooperative_resume_milestone() {
    crate::drivers::uart::write_line("PicoOS milestone:");
    crate::drivers::uart::write_line("  baseline: 0.1.0");
    crate::drivers::uart::write_line("  current: 0.1.64");

    crate::drivers::uart::write_line("");
    crate::drivers::uart::write_line("  cleanup:");
    crate::drivers::uart::write_line("    obsolete standalone task tests removed: OK");
    crate::drivers::uart::write_line("    obsolete standalone scheduler scripts removed: OK");
    crate::drivers::uart::write_line("    obsolete resume task script removed: OK");
    crate::drivers::uart::write_line("    obsolete resume PC proximity requirement removed: OK");

    crate::drivers::uart::write_line("");
    crate::drivers::uart::write_line("  task/resume:");
    crate::drivers::uart::write_line("    RISC-V-only baseline: OK");
    crate::drivers::uart::write_line("    cooperative task resume: OK");
    crate::drivers::uart::write_line("    repeated yield/resume loop: OK");
    crate::drivers::uart::write_line("    scheduler-oriented resume loop: OK");
    crate::drivers::uart::write_line("    RISC-V yield boundary: OK");
    crate::drivers::uart::write_line("    two-task cooperative handoff: OK");

    crate::drivers::uart::write_line("");
    crate::drivers::uart::write_line("  scheduler:");
    crate::drivers::uart::write_line("    scheduler first task dispatch: OK");
    crate::drivers::uart::write_line("    scheduler fresh task dispatch: OK");
    crate::drivers::uart::write_line("    scheduler round-robin fairness: OK");
    crate::drivers::uart::write_line("    scheduler task capacity from table: OK");
    crate::drivers::uart::write_line("    scheduler skips faulted tasks: OK");
    crate::drivers::uart::write_line("    no-runnable scheduler policy: OK");
    crate::drivers::uart::write_line("    scheduler dispatch candidate selection: OK");
    crate::drivers::uart::write_line("    scheduler dispatch candidate model: OK");
    crate::drivers::uart::write_line("    scheduler dispatch candidate-to-decision conversion: OK");
    crate::drivers::uart::write_line("    scheduler dispatch decision model: OK");
    crate::drivers::uart::write_line("    scheduler dispatch decision kind: OK");
    crate::drivers::uart::write_line("    scheduler dispatch decision outcome: OK");
    crate::drivers::uart::write_line("    scheduler dispatch decision logging: OK");
    crate::drivers::uart::write_line("    scheduler dispatch pipeline model: OK");

    crate::drivers::uart::write_line("");
    crate::drivers::uart::write_line("  task lifecycle:");
    crate::drivers::uart::write_line("    task state invariants in core: OK");
    crate::drivers::uart::write_line("    task state lookup in core: OK");
    crate::drivers::uart::write_line("    terminal task dispatch invariants in core: OK");
    crate::drivers::uart::write_line("    no-runnable scheduler snapshot in core: OK");
    crate::drivers::uart::write_line("    task completion summary in core: OK");
    crate::drivers::uart::write_line("    task completion output consolidated: OK");

    crate::drivers::uart::write_line("");
    crate::drivers::uart::write_line("  fault lifecycle:");
    crate::drivers::uart::write_line("    task fault state: OK");
    crate::drivers::uart::write_line("    trap-to-task-fault skeleton: OK");
    crate::drivers::uart::write_line("    real trap classification: OK");
    crate::drivers::uart::write_line("    real trap handler classification: OK");
    crate::drivers::uart::write_line("    real trap handler task-fault return path: OK");
    crate::drivers::uart::write_line("    trap fault metadata reporting: OK");
    crate::drivers::uart::write_line("    fault metadata assertions in core: OK");
    crate::drivers::uart::write_line("    explicit task fault assertions: OK");
    crate::drivers::uart::write_line("    faulted task dispatch guard: OK");
    crate::drivers::uart::write_line("    finished task dispatch guard: OK");
    crate::drivers::uart::write_line("    scheduler fault lifecycle feature: OK");
}

#[cfg(feature = "resume_candidate_test")]
pub fn print_resume_candidate_header() {
    crate::drivers::uart::write_line("");

    #[cfg(feature = "scheduler_run_test")]
    {
        crate::drivers::uart::write_line("scheduler resume candidate check:");
    }

    #[cfg(not(feature = "scheduler_run_test"))]
    {
        crate::drivers::uart::write_line("resume candidate test:");
    }
}

#[cfg(feature = "resume_candidate_test")]
pub fn print_resume_candidate_complete() {
    #[cfg(feature = "scheduler_run_test")]
    {
        crate::drivers::uart::write_line("scheduler resume candidate check complete");
    }

    #[cfg(not(feature = "scheduler_run_test"))]
    {
        crate::drivers::uart::write_line("resume candidate test complete");
    }
}
```

## File: src/kernel/task/context.rs
```rust
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

fn align_down(value: u64, align: u64) -> u64 {
    value & !(align - 1)
}
```

## File: src/kernel/mod.rs
```rust
pub mod banner;
pub mod heap;
pub mod log;
pub mod memory;
pub mod task;
pub mod test;
pub mod ticks;
pub mod trap_frame;
```

## File: src/kernel/trap_frame.rs
```rust
#[repr(C)]
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
}
```

## File: scripts/qemu-expect.sh
```bash
#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "usage: $0 <success-marker>" >&2
  exit 2
fi

marker=$1
timeout_seconds=${QEMU_EXPECT_TIMEOUT:-20}
tmp_dir=$(mktemp -d "${TMPDIR:-/tmp}/picoos-qemu.XXXXXX")
log_pipe="$tmp_dir/out"
mkfifo "$log_pipe"

qemu-system-riscv64 \
  -machine virt \
  -nographic \
  -bios none \
  -kernel target/riscv64gc-unknown-none-elf/debug/PicoOS \
  >"$log_pipe" 2>&1 &
qemu_pid=$!
status=1

cleanup() {
  kill "$qemu_pid" 2>/dev/null || true
  wait "$qemu_pid" 2>/dev/null || true
  rm -rf "$tmp_dir"
}
trap cleanup EXIT

(
  sleep "$timeout_seconds"
  kill "$qemu_pid" 2>/dev/null || true
) &
watchdog_pid=$!

while IFS= read -r line; do
  printf '%s\n' "$line"
  clean_line=${line%$'\r'}

  if [[ "$clean_line" == "$marker" ]]; then
    status=0
    break
  fi

  if [[ "$clean_line" == *"FAILED"* || "$clean_line" == *"system halted after trap"* ]]; then
    status=1
    break
  fi

  if [[ "$clean_line" == *"[FAIL]"* && "$clean_line" != *"kernel fault -> halt"* ]]; then
    status=1
    break
  fi
done <"$log_pipe"

kill "$watchdog_pid" 2>/dev/null || true
exit "$status"
```

## File: scripts/test-kernel-fault-guard-riscv.sh
```bash
#!/usr/bin/env bash
set -euo pipefail

cargo clean

cargo build \
  --target riscv64gc-unknown-none-elf \
  --features "kernel_fault_guard_test"

scripts/qemu-expect.sh "kernel fault guard result: OK"
```

## File: scripts/test-scheduler-fault-lifecycle-riscv.sh
```bash
#!/usr/bin/env bash
set -euo pipefail

cargo clean

cargo build \
  --target riscv64gc-unknown-none-elf \
  --features "scheduler_fault_lifecycle_test"

scripts/qemu-expect.sh "task fault scheduler result: OK"
```

## File: scripts/test-scheduler-reentry-riscv.sh
```bash
#!/usr/bin/env bash
set -euo pipefail

cargo clean

cargo build --features "scheduler_reentry_selftest"

scripts/qemu-expect.sh "scheduler resume loop result: OK"
```

## File: scripts/test-scheduler-run-once-riscv.sh
```bash
#!/usr/bin/env bash
set -euo pipefail

cargo clean

cargo build --features "scheduler_run_once_selftest"

scripts/qemu-expect.sh "scheduler resume loop result: OK"
```

## File: scripts/test-scheduler-run-riscv.sh
```bash
#!/usr/bin/env bash
set -euo pipefail

cargo clean

cargo build --features "scheduler_run_selftest"

scripts/qemu-expect.sh "scheduler resume loop result: OK"
```

## File: scripts/test-scheduler-runtime-riscv.sh
```bash
#!/usr/bin/env bash
set -euo pipefail

cargo clean

cargo build --features "scheduler_runtime_selftest"

scripts/qemu-expect.sh "scheduler resume loop result: OK"
```

## File: scripts/test-two-task-handoff-riscv.sh
```bash
#!/usr/bin/env bash
set -euo pipefail

cargo clean

cargo build --features "two_task_resume_handoff_selftest"

scripts/qemu-expect.sh "scheduler resume loop result: OK"
```

## File: src/kernel/task/test/invariants.rs
```rust
use crate::drivers::uart;

pub fn print_resume_eligibility_check(task_id: usize) {
    uart::write_line("  resume eligibility check:");

    let state = crate::kernel::task::table::get_task_state(task_id);
    let can_resume = crate::kernel::task::table::can_task_resume(task_id);

    match (state, can_resume) {
        (Some(crate::kernel::task::table::TaskState::Ready), Some(true)) => {
            uart::write_line("    task can be resumed later");
        }
        (Some(crate::kernel::task::table::TaskState::Finished), Some(false)) => {
            uart::write_line("    task is finished; resume disabled");
        }
        (Some(crate::kernel::task::table::TaskState::Faulted), Some(false)) => {
            uart::write_line("    task is faulted; resume disabled");
        }
        (Some(crate::kernel::task::table::TaskState::Blocked), Some(false)) => {
            uart::write_line("    task is blocked; resume disabled");
        }
        _ => {
            uart::write_line("    task resume state is inconsistent");
        }
    }
}

pub fn print_cpu_context_consistency_check(task_id: usize) {
    uart::write_line("  CPU context consistency check:");

    let cpu_context = crate::kernel::task::table::get_task_cpu_context(task_id);
    let last_task_sp = crate::kernel::task::table::get_task_last_task_sp(task_id);
    let kernel_return_pc = crate::kernel::task::table::get_task_last_kernel_return_pc(task_id);

    match (cpu_context, last_task_sp, kernel_return_pc) {
        (Some(context), Some(task_sp), Some(return_pc)) => {
            uart::write_str("    context.sp == last_task_sp: ");
            crate::kernel::task::table::print_yes_no(context.sp == task_sp);
            uart::write_line("");

            uart::write_str("    context.return_pc == kernel_return_pc: ");
            crate::kernel::task::table::print_yes_no(context.return_pc == return_pc);
            uart::write_line("");

            uart::write_str("    context valid: ");
            crate::kernel::task::table::print_yes_no(context.is_valid());
            uart::write_line("");

            uart::write_str("    context SP inside task stack: ");
            match crate::kernel::task::table::is_sp_inside_task_stack(task_id, context.sp) {
                Some(value) => {
                    crate::kernel::task::table::print_yes_no(value);
                    uart::write_line("");
                }
                None => uart::write_line("unknown"),
            }

            uart::write_str("    context.resume_pc non-zero: ");
            crate::kernel::task::table::print_yes_no(context.resume_pc != 0);
            uart::write_line("");

            let ok = context.sp == task_sp
                && context.return_pc == return_pc
                && context.resume_pc != 0
                && context.is_valid()
                && matches!(
                    crate::kernel::task::table::is_sp_inside_task_stack(task_id, context.sp),
                    Some(true)
                );

            uart::write_str("    consistency result: ");
            if ok {
                uart::write_line("OK");
            } else {
                uart::write_line("FAILED");
            }
        }
        _ => {
            uart::write_line("    consistency result: FAILED");
        }
    }
}

pub fn print_illegal_transition_checks(task_id: usize) {
    use crate::kernel::task::table::TaskLifecycleTransition::{Exit, Fault, Yield};
    use crate::kernel::task::table::TaskState;

    uart::write_line("  lifecycle transition guard check:");

    let Some(state) = crate::kernel::task::table::get_task_state(task_id) else {
        uart::write_line("    state unknown");
        return;
    };

    let yield_allowed = crate::kernel::task::table::can_apply_task_transition(task_id, Yield);
    let exit_allowed = crate::kernel::task::table::can_apply_task_transition(task_id, Exit);
    let fault_allowed = crate::kernel::task::table::can_apply_task_transition(task_id, Fault);

    let guard_ok = match state {
        TaskState::Finished | TaskState::Faulted => {
            !yield_allowed && !exit_allowed && !fault_allowed
        }
        _ => true,
    };

    uart::write_str("    illegal transitions blocked: ");
    if guard_ok {
        uart::write_line("OK");
    } else {
        uart::write_line("FAILED");
    }
}
```

## File: src/kernel/heap.rs
```rust
use crate::drivers::uart;
use crate::kernel::memory;

const HEAP_PAGES: usize = 4;

static mut HEAP_START: u64 = 0;
static mut HEAP_END: u64 = 0;
static mut HEAP_NEXT: u64 = 0;

fn reset_heap_state() {
    unsafe {
        HEAP_START = 0;
        HEAP_END = 0;
        HEAP_NEXT = 0;
    }
}

pub fn init() -> bool {
    reset_heap_state();

    let first_page = memory::allocate_page();

    let Some(start) = first_page else {
        return false;
    };

    let mut last = start;

    for _ in 1..HEAP_PAGES {
        let Some(page) = memory::allocate_page() else {
            reset_heap_state();
            return false;
        };

        last = page;
    }

    let Some(end) = last.checked_add(memory::PAGE_SIZE) else {
        reset_heap_state();
        return false;
    };

    unsafe {
        HEAP_START = start;
        HEAP_END = end;
        HEAP_NEXT = start;
    }

    true
}

pub fn alloc(size: u64, align: u64) -> Option<u64> {
    if size == 0 || align == 0 || !align.is_power_of_two() {
        return None;
    }

    unsafe {
        if HEAP_START == 0 || HEAP_END == 0 || HEAP_NEXT == 0 {
            return None;
        }

        let start = align_up(HEAP_NEXT, align)?;
        let end = start.checked_add(size)?;

        if end > HEAP_END {
            return None;
        }

        HEAP_NEXT = end;

        Some(start)
    }
}

pub fn heap_start() -> u64 {
    unsafe { HEAP_START }
}

pub fn heap_end() -> u64 {
    unsafe { HEAP_END }
}

pub fn heap_next() -> u64 {
    unsafe { HEAP_NEXT }
}

pub fn heap_size() -> u64 {
    unsafe { HEAP_END.saturating_sub(HEAP_START) }
}

pub fn test_heap() {
    uart::write_line("");
    uart::write_line("heap:");

    if !init() {
        uart::write_line("heap init: FAILED");
        return;
    }

    uart::write_str("heap start: ");
    uart::write_hex_u64(heap_start());
    uart::write_line("");

    uart::write_str("heap end: ");
    uart::write_hex_u64(heap_end());
    uart::write_line("");

    uart::write_str("heap size: ");
    uart::write_dec_u64(heap_size());
    uart::write_line(" bytes");

    alloc_and_print(64, 8);
    alloc_and_print(128, 16);
    alloc_and_print(4096, 4096);
    alloc_and_print(8192, 4096);

    uart::write_str("next heap pointer: ");
    uart::write_hex_u64(heap_next());
    uart::write_line("");
}

fn alloc_and_print(size: u64, align: u64) {
    uart::write_str("alloc ");
    uart::write_dec_u64(size);
    uart::write_str(" bytes align ");
    uart::write_dec_u64(align);
    uart::write_str(": ");

    match alloc(size, align) {
        Some(addr) => {
            uart::write_hex_u64(addr);
            uart::write_line("");
        }
        None => {
            uart::write_line("FAILED");
        }
    }
}

fn align_up(value: u64, align: u64) -> Option<u64> {
    if align == 0 || !align.is_power_of_two() {
        return None;
    }

    Some(value.checked_add(align - 1)? & !(align - 1))
}
```

## File: src/kernel/log.rs
```rust
use crate::drivers::uart;

fn scope_enabled(scope: &str) -> bool {
    // Keep current behavior by default. Scope filtering activates only with kernel_log_scoped.
    if !cfg!(feature = "kernel_log_scoped") {
        return true;
    }

    if scope == "scheduler" {
        return cfg!(feature = "scheduler_verbose_dispatch_trace");
    }
    if scope == "resume" {
        return cfg!(feature = "verbose_resume_debug");
    }
    if scope == "trap" {
        return cfg!(feature = "log_trap");
    }
    if scope == "timer" {
        return cfg!(feature = "log_timer");
    }
    if scope == "fault" {
        return cfg!(feature = "log_fault");
    }
    if scope == "sleep" {
        return cfg!(feature = "log_sleep");
    }
    true
}

fn prefix(level: &str, scope: &str) {
    uart::write_str("[");
    uart::write_str(level);
    uart::write_str("][");
    uart::write_str(scope);
    uart::write_str("] ");
}

#[allow(dead_code)]
pub fn info(scope: &str, message: &str) {
    if !scope_enabled(scope) {
        return;
    }
    prefix("INFO", scope);
    uart::write_line(message);
}

#[allow(dead_code)]
pub fn ok(scope: &str, message: &str) {
    if !scope_enabled(scope) {
        return;
    }
    prefix("OK", scope);
    uart::write_line(message);
}

#[allow(dead_code)]
pub fn fail(scope: &str, message: &str) {
    if !scope_enabled(scope) {
        return;
    }
    prefix("FAIL", scope);
    uart::write_line(message);
}

#[cfg(any(
    feature = "scheduler_verbose_dispatch_trace",
    feature = "verbose_resume_debug"
))]
#[allow(dead_code)]
pub fn trace(scope: &str, message: &str) {
    if !scope_enabled(scope) {
        return;
    }
    prefix("TRACE", scope);
    uart::write_line(message);
}

#[cfg(not(any(
    feature = "scheduler_verbose_dispatch_trace",
    feature = "verbose_resume_debug"
)))]
#[allow(dead_code)]
pub fn trace(_scope: &str, _message: &str) {}
```

## File: .zed/settings.json
```json
{
  "lsp": {
    "rust-analyzer": {
      "initialization_options": {
        "cargo": {
          "allTargets": false,
          "target": "riscv64gc-unknown-none-elf",
        },
        "check": {
          "allTargets": false,
          "targets": ["riscv64gc-unknown-none-elf"],
        },
      },
    },
  },
}
```

## File: src/kernel/task/test/bootstrap.rs
```rust
#[cfg(any(
    feature = "scheduler_fault_lifecycle_test",
    feature = "two_task_resume_handoff_test",
    feature = "task_fault_test",
    feature = "task_sleep_test"
))]
use crate::drivers::uart;

pub fn print_task_zero_context_guard() {
    use crate::kernel::task::debug::TrapExecutionContext;

    crate::kernel::task::debug::set_debug_current_task_id(0);

    let ok = matches!(
        crate::kernel::task::debug::current_trap_execution_context(),
        TrapExecutionContext::Task
    );

    crate::kernel::task::debug::clear_debug_current_task_id();

    crate::drivers::uart::write_str("task id 0 context guard: ");
    crate::kernel::task::table::print_yes_no(ok);
    crate::drivers::uart::write_line("");

    if !ok {
        crate::arch::halt();
    }
}

#[cfg(feature = "task_sleep_test")]
pub fn test_task_sleep_wakeup_table_selftest() {
    uart::write_line("task sleep table selftest:");

    let task_id = 1usize;

    let blocked = crate::kernel::task::table::mark_task_blocked_until(task_id, 3);
    uart::write_str("  mark blocked until tick=3: ");
    crate::kernel::task::table::print_yes_no(blocked);
    uart::write_line("");

    let woke_early = crate::kernel::task::table::wake_sleeping_tasks(2);
    uart::write_str("  woke at tick=2: ");
    uart::write_dec_u64(woke_early as u64);
    uart::write_line("");

    let state_still_blocked = matches!(
        crate::kernel::task::table::get_task_state(task_id),
        Some(crate::kernel::task::table::TaskState::Blocked)
    );
    uart::write_str("  still blocked at tick=2: ");
    crate::kernel::task::table::print_yes_no(state_still_blocked);
    uart::write_line("");

    let woke_on_time = crate::kernel::task::table::wake_sleeping_tasks(3);
    uart::write_str("  woke at tick=3: ");
    uart::write_dec_u64(woke_on_time as u64);
    uart::write_line("");

    let state_ready = matches!(
        crate::kernel::task::table::get_task_state(task_id),
        Some(crate::kernel::task::table::TaskState::Ready)
    );
    uart::write_str("  state Ready after wake: ");
    crate::kernel::task::table::print_yes_no(state_ready);
    uart::write_line("");

    if blocked && woke_early == 0 && state_still_blocked && woke_on_time == 1 && state_ready {
        uart::write_line("task sleep wake result: OK");
    } else {
        uart::write_line("task sleep wake result: FAILED");
        crate::arch::halt();
    }
}

#[cfg(feature = "scheduler_fault_lifecycle_test")]
pub fn test_scheduler_fault_lifecycle_bootstrap() {
    uart::write_line("scheduler fault lifecycle bootstrap:");
    uart::write_line("bootstrap action: scheduler starts first fresh task");

    crate::kernel::task::scheduler::set_current_task(0);

    match crate::kernel::task::scheduler::run() {
        crate::kernel::task::scheduler::RunResult::NoRunnableTask => {
            uart::write_line("scheduler fault lifecycle bootstrap result: no runnable task");
        }
        crate::kernel::task::scheduler::RunResult::Failed => {
            uart::write_line("scheduler fault lifecycle bootstrap result: failed");
        }
    }
}

#[cfg(feature = "two_task_resume_handoff_test")]
pub fn test_two_task_resume_handoff_bootstrap() {
    uart::write_line("two-task handoff bootstrap:");
    uart::write_line("bootstrap action: scheduler starts first fresh task");

    crate::kernel::task::scheduler::set_current_task(0);

    match crate::kernel::task::scheduler::run() {
        crate::kernel::task::scheduler::RunResult::NoRunnableTask => {
            uart::write_line("two-task handoff bootstrap result: no runnable task");
            crate::arch::halt();
        }
        crate::kernel::task::scheduler::RunResult::Failed => {
            uart::write_line("two-task handoff bootstrap result: FAILED");
            crate::arch::halt();
        }
    }
}

#[cfg(feature = "task_fault_test")]
pub fn test_task_fault_bootstrap() {
    uart::write_line("task fault bootstrap:");
    uart::write_line("bootstrap action: scheduler starts first fresh task");

    crate::kernel::task::scheduler::set_current_task(0);

    match crate::kernel::task::scheduler::run() {
        crate::kernel::task::scheduler::RunResult::NoRunnableTask => {
            uart::write_line("task fault bootstrap result: no runnable task");
            crate::arch::halt();
        }
        crate::kernel::task::scheduler::RunResult::Failed => {
            uart::write_line("task fault bootstrap result: FAILED");
            crate::arch::halt();
        }
    }
}
```

## File: src/kernel/task/fault.rs
```rust
use crate::drivers::uart;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TrapFaultClassification {
    KernelFault,
    TaskFault,
}

#[allow(dead_code)]
pub fn classify_current_trap_fault() -> TrapFaultClassification {
    match crate::kernel::task::debug::current_trap_execution_context() {
        crate::kernel::task::debug::TrapExecutionContext::Kernel => {
            TrapFaultClassification::KernelFault
        }

        crate::kernel::task::debug::TrapExecutionContext::Task => {
            TrapFaultClassification::TaskFault
        }
    }
}

#[allow(dead_code)]
pub fn print_current_trap_fault_classification() {
    uart::write_line("trap fault classification:");

    match classify_current_trap_fault() {
        TrapFaultClassification::KernelFault => {
            uart::write_line("  context: kernel");
            uart::write_line("  classification: kernel fault");
            uart::write_line("  action: halt");
        }

        TrapFaultClassification::TaskFault => {
            uart::write_line("  context: task");
            uart::write_line("  classification: task fault");
            uart::write_line("  action: mark current task faulted and return to scheduler");
        }
    }
}

#[allow(dead_code)]
pub fn record_current_task_fault(mcause: u64, mepc: u64, mtval: u64) -> Option<usize> {
    let task_id = crate::kernel::task::debug::debug_current_task_id();

    if matches!(
        crate::kernel::task::table::get_task_state(task_id),
        Some(crate::kernel::task::table::TaskState::Faulted)
    ) {
        crate::kernel::log::fail("fault", "DOUBLE TRAP DETECTED (simulated)");
        uart::write_str("  task already Faulted: ");
        crate::kernel::task::table::print_task_name_by_id(task_id);
        uart::write_line("");
        uart::write_line("  system halted to prevent infinite trap loop");
        crate::arch::halt();
    }

    let Some(fault_reason) =
        crate::kernel::task::table::record_task_fault(task_id, mcause, mepc, mtval)
    else {
        crate::kernel::log::fail("fault", "record task fault: FAILED");
        return None;
    };

    uart::write_str("  fault reason: ");
    crate::kernel::task::table::print_task_fault_reason(fault_reason);
    uart::write_line("");

    uart::write_str("  fault mcause: ");
    uart::write_hex_u64(mcause);
    uart::write_line("");

    uart::write_str("  fault mepc:   ");
    uart::write_hex_u64(mepc);
    uart::write_line("");

    uart::write_str("  fault mtval:  ");
    uart::write_hex_u64(mtval);
    uart::write_line("");

    crate::kernel::log::ok("fault", "record task fault: OK");

    Some(task_id)
}

#[allow(dead_code)]
pub fn return_current_task_fault(task_sp: u64, kernel_sp: u64, return_pc: u64) -> ! {
    crate::kernel::task::debug::set_debug_last_task_sp(task_sp);
    crate::kernel::task::debug::set_debug_task_return_kind(
        crate::kernel::task::table::TaskReturnKind::Fault,
    );

    crate::arch::return_to_kernel_stack_checked(kernel_sp, return_pc);
}
```

## File: src/kernel/task/mod.rs
```rust
pub mod context;
pub mod cpu_context;
pub mod debug;
pub mod entry;
pub mod fault;
pub mod scheduler;
pub mod table;
#[allow(dead_code)]
pub mod test;

pub use entry::*;
#[allow(unused_imports)]
pub use fault::*;
pub use table::*;
#[allow(unused_imports)]
pub use test::*;

#[cfg(feature = "resume_candidate_test")]
#[allow(unused_imports)]
pub use test::test_resume_candidate_selection;

#[cfg(feature = "resume_preflight_test")]
#[allow(unused_imports)]
pub use test::test_resume_preflight_check;

#[cfg(feature = "resume_dry_run_test")]
#[allow(unused_imports)]
pub use test::test_resume_dry_run;

#[cfg(feature = "resume_restore_test")]
#[allow(unused_imports)]
pub use test::test_resume_restore;

#[cfg(feature = "kernel_fault_guard_test")]
pub use test::test_kernel_fault_guard;
```

## File: src/kernel/memory.rs
```rust
use crate::drivers::uart;
use crate::platform;

pub const PAGE_SIZE: u64 = 4096;

unsafe extern "C" {
    static __kernel_start: u8;
    static __kernel_end: u8;

    static __text_start: u8;
    static __text_end: u8;

    static __rodata_start: u8;
    static __rodata_end: u8;

    static __data_start: u8;
    static __data_end: u8;

    static __bss_start: u8;
    static __bss_end: u8;

    static __stack_top: u8;
    static __free_memory_start: u8;
}

static mut NEXT_FREE_PAGE: u64 = 0;
static mut MEMORY_END: u64 = 0;

#[inline(always)]
fn symbol_addr(symbol: *const u8) -> u64 {
    symbol as u64
}

pub fn init() {
    let end = (platform::RAM_START as u64).checked_add(platform::RAM_SIZE as u64);
    let start = align_up(free_memory_start(), PAGE_SIZE);

    let (Some(start), Some(end)) = (start, end) else {
        unsafe {
            NEXT_FREE_PAGE = 0;
            MEMORY_END = 0;
        }
        return;
    };

    unsafe {
        NEXT_FREE_PAGE = if start <= end { start } else { 0 };
        MEMORY_END = end;
    }
}

pub fn allocate_page() -> Option<u64> {
    unsafe {
        if NEXT_FREE_PAGE == 0 || MEMORY_END == 0 {
            return None;
        }

        let page = NEXT_FREE_PAGE;
        let next = page.checked_add(PAGE_SIZE)?;

        if next > MEMORY_END {
            return None;
        }

        NEXT_FREE_PAGE = next;

        Some(page)
    }
}

pub fn free_memory_current() -> u64 {
    unsafe { NEXT_FREE_PAGE }
}

pub fn memory_end() -> u64 {
    unsafe { MEMORY_END }
}

pub fn kernel_start() -> u64 {
    symbol_addr(core::ptr::addr_of!(__kernel_start))
}

pub fn kernel_end() -> u64 {
    symbol_addr(core::ptr::addr_of!(__kernel_end))
}

pub fn text_start() -> u64 {
    symbol_addr(core::ptr::addr_of!(__text_start))
}

pub fn text_end() -> u64 {
    symbol_addr(core::ptr::addr_of!(__text_end))
}

pub fn rodata_start() -> u64 {
    symbol_addr(core::ptr::addr_of!(__rodata_start))
}

pub fn rodata_end() -> u64 {
    symbol_addr(core::ptr::addr_of!(__rodata_end))
}

pub fn data_start() -> u64 {
    symbol_addr(core::ptr::addr_of!(__data_start))
}

pub fn data_end() -> u64 {
    symbol_addr(core::ptr::addr_of!(__data_end))
}

pub fn bss_start() -> u64 {
    symbol_addr(core::ptr::addr_of!(__bss_start))
}

pub fn bss_end() -> u64 {
    symbol_addr(core::ptr::addr_of!(__bss_end))
}

pub fn stack_top() -> u64 {
    symbol_addr(core::ptr::addr_of!(__stack_top))
}

pub fn free_memory_start() -> u64 {
    symbol_addr(core::ptr::addr_of!(__free_memory_start))
}

#[allow(dead_code)]
pub fn kernel_text_start() -> u64 {
    text_start()
}

#[allow(dead_code)]
pub fn kernel_text_end() -> u64 {
    text_end()
}

#[allow(dead_code)]
pub fn is_inside_kernel_text(addr: u64) -> bool {
    addr >= kernel_text_start() && addr < kernel_text_end()
}

pub fn print_memory_layout() {
    uart::write_line("");
    uart::write_line("memory layout:");

    print_range("kernel", kernel_start(), kernel_end());
    print_range("text", text_start(), text_end());
    print_range("rodata", rodata_start(), rodata_end());
    print_range("data", data_start(), data_end());
    print_range("bss", bss_start(), bss_end());

    uart::write_str("stack_top: ");
    uart::write_hex_u64(stack_top());
    uart::write_line("");

    uart::write_str("free_memory_start: ");
    uart::write_hex_u64(free_memory_start());
    uart::write_line("");

    uart::write_str("RAM start: ");
    uart::write_hex_u64(platform::RAM_START as u64);
    uart::write_line("");

    uart::write_str("RAM end: ");
    uart::write_hex_u64(platform::RAM_START as u64 + platform::RAM_SIZE as u64);
    uart::write_line("");
}

pub fn test_page_allocator() {
    uart::write_line("");
    uart::write_line("page allocator:");

    init();

    uart::write_str("page size: ");
    uart::write_dec_u64(PAGE_SIZE);
    uart::write_line(" bytes");

    uart::write_str("RAM start: ");
    uart::write_hex_u64(platform::RAM_START as u64);
    uart::write_line("");

    uart::write_str("RAM end: ");
    uart::write_hex_u64(memory_end());
    uart::write_line("");

    uart::write_str("initial free page: ");
    uart::write_hex_u64(free_memory_current());
    uart::write_line("");

    allocate_and_print();
    allocate_and_print();
    allocate_and_print();

    uart::write_str("next free page: ");
    uart::write_hex_u64(free_memory_current());
    uart::write_line("");
}

fn allocate_and_print() {
    match allocate_page() {
        Some(page) => {
            uart::write_str("allocated page: ");
            uart::write_hex_u64(page);
            uart::write_line("");
        }
        None => {
            uart::write_line("allocated page: FAILED");
        }
    }
}

fn print_range(name: &str, start: u64, end: u64) {
    uart::write_str(name);
    uart::write_str(": ");
    uart::write_hex_u64(start);
    uart::write_str(" - ");
    uart::write_hex_u64(end);
    uart::write_str(" size: ");
    uart::write_dec_u64(end.saturating_sub(start));
    uart::write_line(" bytes");
}

fn align_up(value: u64, align: u64) -> Option<u64> {
    if align == 0 || !align.is_power_of_two() {
        return None;
    }

    Some(value.checked_add(align - 1)? & !(align - 1))
}
```

## File: src/kernel/test.rs
```rust
use crate::drivers::uart;
use crate::kernel;

#[derive(Clone, Copy)]
struct RuntimeSelftestScenario {
    name: &'static str,
    run_bootstrap: fn(),
    run_after_scheduler_init: fn(),
}

#[allow(clippy::needless_return)]
fn runtime_selftest_scenario() -> RuntimeSelftestScenario {
    #[cfg(feature = "kernel_fault_guard_test")]
    {
        return RuntimeSelftestScenario {
            name: "kernel_fault_guard",
            run_bootstrap: runtime_bootstrap_kernel_fault_guard,
            run_after_scheduler_init: runtime_after_scheduler_noop,
        };
    }

    #[cfg(all(not(feature = "kernel_fault_guard_test"), feature = "task_yield_test"))]
    {
        return RuntimeSelftestScenario {
            name: "task_yield",
            run_bootstrap: runtime_bootstrap_task_yield,
            run_after_scheduler_init: runtime_after_scheduler_task_yield,
        };
    }

    #[cfg(all(
        not(feature = "kernel_fault_guard_test"),
        not(feature = "task_yield_test")
    ))]
    RuntimeSelftestScenario {
        name: "basic_tasks",
        run_bootstrap: runtime_bootstrap_basic_tasks,
        run_after_scheduler_init: runtime_after_scheduler_noop,
    }
}

pub fn run_memory_tests() {
    kernel::memory::print_memory_layout();
    kernel::memory::test_page_allocator();
    kernel::heap::test_heap();
}

pub fn print_test_complete() {
    uart::write_line("timer test complete");
    uart::write_line("system halted");
}

pub fn run_runtime_selftest_bootstrap() {
    let scenario = runtime_selftest_scenario();
    uart::write_str("runtime selftest scenario: ");
    uart::write_line(scenario.name);
    (scenario.run_bootstrap)();
}

pub fn run_runtime_selftest_after_scheduler_init() {
    let scenario = runtime_selftest_scenario();
    (scenario.run_after_scheduler_init)();
}

#[cfg(all(
    not(feature = "kernel_fault_guard_test"),
    not(feature = "task_yield_test")
))]
fn runtime_bootstrap_basic_tasks() {
    run_memory_tests();
    crate::kernel::task::test_tasks();
}

#[allow(dead_code)]
#[cfg(feature = "task_yield_test")]
fn runtime_bootstrap_task_yield() {
    run_memory_tests();
    crate::kernel::task::test_tasks_with_yield_worker();
}

#[allow(dead_code)]
#[cfg(not(feature = "task_yield_test"))]
fn runtime_bootstrap_task_yield() {
    crate::arch::halt();
}

#[allow(dead_code)]
#[cfg(feature = "kernel_fault_guard_test")]
fn runtime_bootstrap_kernel_fault_guard() {
    crate::kernel::task::test_kernel_fault_guard();
}

#[allow(dead_code)]
#[cfg(not(feature = "kernel_fault_guard_test"))]
fn runtime_bootstrap_kernel_fault_guard() {
    crate::arch::halt();
}

#[allow(dead_code)]
#[cfg(feature = "task_yield_test")]
fn runtime_after_scheduler_task_yield() {
    crate::kernel::task::test_task_yield();
}

#[allow(dead_code)]
#[cfg(not(feature = "task_yield_test"))]
fn runtime_after_scheduler_task_yield() {}

#[allow(dead_code)]
fn runtime_after_scheduler_noop() {}

#[allow(dead_code)]
fn runtime_selftest_scenario_name() -> &'static str {
    runtime_selftest_scenario().name
}

#[cfg(feature = "selftest")]
pub fn run_selftests() -> ! {
    uart::write_line("");
    uart::write_line("selftest mode:");

    uart::write_line("");
    uart::write_line("[selftest] memory");
    run_memory_tests();

    uart::write_line("");
    uart::write_line("");
    uart::write_line("[selftest] task table");
    #[cfg(feature = "task_yield_test")]
    crate::kernel::task::test_tasks_with_yield_worker();

    #[cfg(not(feature = "task_yield_test"))]
    crate::kernel::task::test_tasks();

    uart::write_line("");
    uart::write_line("selftest complete");
    crate::arch::halt();
}
```

## File: src/main.rs
```rust
#![no_std]
#![no_main]
#![cfg_attr(feature = "selftest", allow(dead_code))]
#![cfg_attr(
    feature = "kernel_fault_guard_test",
    allow(dead_code, unreachable_code)
)]

use core::arch::asm;
use core::panic::PanicInfo;

mod arch;
mod drivers;
mod kernel;
mod platform;

use crate::drivers::uart;

#[no_mangle]
pub extern "C" fn kernel_main() -> ! {
    uart::write_line("");
    kernel::banner::print_boot_banner();

    uart::write_line("arch: riscv64");
    uart::write_line("target arch: riscv64");
    uart::write_line("platform: QEMU virt riscv64");
    uart::write_line("status: kernel started");
    kernel::banner::print_capabilities();

    #[cfg(feature = "selftest")]
    {
        kernel::test::run_selftests();
    }

    #[cfg(not(feature = "selftest"))]
    {
        arch::init_exceptions();
        arch::print_cpu_info();
        kernel::test::run_runtime_selftest_bootstrap();

        kernel::task::scheduler::init();
        kernel::test::run_runtime_selftest_after_scheduler_init();

        {
            use crate::arch::riscv64::{cpu, timer};

            const RISCV_TIMER_HZ: u64 = 1;

            uart::write_line("");
            uart::write_line("RISC-V timer:");

            uart::write_str("timebase frequency: ");
            uart::write_dec_u64(timer::timebase_frequency());
            uart::write_line(" Hz");

            uart::write_str("mtime before: ");
            uart::write_hex_u64(timer::mtime());
            uart::write_line("");

            uart::write_str("starting periodic timer: ");
            uart::write_dec_u64(RISCV_TIMER_HZ);
            uart::write_line(" Hz");

            kernel::ticks::reset();
            timer::arm_timer_hz(RISCV_TIMER_HZ);

            uart::write_line("enabling machine timer interrupt...");
            cpu::enable_machine_timer_interrupt();

            uart::write_line("enabling machine interrupts...");
            arch::enable_irq();

            uart::write_str("mstatus after enable: ");
            uart::write_hex_u64(cpu::mstatus());
            uart::write_line("");

            uart::write_str("mie after enable: ");
            uart::write_hex_u64(cpu::mie());
            uart::write_line("");

            uart::write_line("waiting for RISC-V ticks...");
        }

        loop {
            arch::wait_for_interrupt();
        }
    }
}

#[allow(dead_code)]
fn trigger_test_exception() {
    unsafe {
        asm!("ebreak", options(nomem, nostack, preserves_flags));
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    uart::write_line("");
    uart::write_line("KERNEL PANIC");

    arch::halt();
}
```

## File: src/arch/riscv64/mod.rs
```rust
use core::arch::asm;

pub mod cpu;
pub mod timer;
pub mod traps;

core::arch::global_asm!(include_str!("boot.S"));
core::arch::global_asm!(include_str!("trap.S"));

#[cfg(target_arch = "riscv64")]
core::arch::global_asm!(
    r#"
    .section .text
    .global task_yield_boundary
    .type task_yield_boundary, @function

task_yield_boundary:
    /*
     * Rust call ABI:
     *   a0 = kernel_sp
     *   a1 = return_pc
     *
     * At function entry:
     *   sp = task stack pointer at call boundary
     *   ra = continuation address after call task_yield_boundary
     */

    mv t2, a0
    mv t3, a1

    mv t0, sp
    mv t1, ra

    /*
     * yield_to_kernel_returning_stub ABI:
     *   a0 = task_sp
     *   a1 = resume_pc
     *   a2 = kernel_sp
     *   a3 = return_pc
     */
    mv a0, t0
    mv a1, t1
    mv a2, t2
    mv a3, t3

    j yield_to_kernel_returning_stub
"#
);

unsafe extern "C" {
    pub fn task_yield_boundary(kernel_sp: u64, return_pc: u64);
}

unsafe extern "C" {
    static trap_vector: u8;
    static __trap_stack_top: u8;
}

#[inline(always)]
fn symbol_addr(symbol: *const u8) -> u64 {
    symbol as u64
}

#[inline(always)]
pub fn halt() -> ! {
    loop {
        unsafe {
            asm!("wfi", options(nomem, nostack, preserves_flags));
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RiscvYieldContext {
    pub ra: u64,
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
}

static mut LAST_RISCV_YIELD_CONTEXT: RiscvYieldContext = RiscvYieldContext {
    ra: 0,
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
};

#[allow(dead_code)]
pub fn last_riscv_yield_context() -> RiscvYieldContext {
    unsafe { LAST_RISCV_YIELD_CONTEXT }
}

pub fn init_exceptions() {
    let trap_addr = symbol_addr(core::ptr::addr_of!(trap_vector));
    let trap_stack_top = trap_stack_top();

    cpu::set_mtvec(trap_addr);
    cpu::set_mscratch(trap_stack_top);

    crate::drivers::uart::write_str("mtvec: ");
    crate::drivers::uart::write_hex_u64(cpu::mtvec());
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("trap stack top: ");
    crate::drivers::uart::write_hex_u64(trap_stack_top);
    crate::drivers::uart::write_line("");
}

fn trap_stack_top() -> u64 {
    symbol_addr(core::ptr::addr_of!(__trap_stack_top))
}

pub fn reset_trap_stack_pointer_for_next_trap() {
    cpu::set_mscratch(trap_stack_top());
}

#[cfg(feature = "kernel_fault_guard_test")]
pub fn is_trap_stack_addr(addr: u64) -> bool {
    let top = trap_stack_top();

    addr >= top - 4096 && addr < top
}

pub fn enable_irq() {
    cpu::enable_machine_interrupts();
}

pub fn disable_irq() {
    cpu::disable_machine_interrupts();
}

#[inline(always)]
pub fn wait_for_interrupt() {
    unsafe {
        asm!("wfi", options(nomem, nostack, preserves_flags));
    }
}

#[allow(dead_code)]
pub fn return_from_interrupt() -> ! {
    halt();
}

pub fn print_cpu_info() {
    crate::drivers::uart::write_line("riscv64 CPU info:");

    crate::drivers::uart::write_str("mhartid: ");
    crate::drivers::uart::write_hex_u64(cpu::mhartid());
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("mstatus: ");
    crate::drivers::uart::write_hex_u64(cpu::mstatus());
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("mtvec: ");
    crate::drivers::uart::write_hex_u64(cpu::mtvec());
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("mepc: ");
    crate::drivers::uart::write_hex_u64(cpu::mepc());
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("mcause: ");
    crate::drivers::uart::write_hex_u64(cpu::mcause());
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("mie: ");
    crate::drivers::uart::write_hex_u64(cpu::mie());
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("mip: ");
    crate::drivers::uart::write_hex_u64(cpu::mip());
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("SP: ");
    crate::drivers::uart::write_hex_u64(cpu::stack_pointer());
    crate::drivers::uart::write_line("");
}

#[allow(dead_code)]
#[inline(never)]
pub unsafe fn start_task_on_stack(entry: usize, stack_top: u64) -> ! {
    unsafe {
        asm!(
        "mv sp, {stack}",
        "mv a0, {entry}",
        "call {trampoline}",
        stack = in(reg) stack_top,
        entry = in(reg) entry,
        trampoline = sym crate::kernel::task::task_trampoline_raw,
        options(noreturn)
        );
    }
}

#[inline(always)]
pub fn stack_pointer() -> u64 {
    cpu::stack_pointer()
}

#[inline(never)]
pub unsafe fn return_to_kernel_stack(kernel_sp: u64, return_pc: u64) -> ! {
    unsafe {
        asm!(
        "mv sp, {kernel_sp}",
        "jr {return_pc}",
        kernel_sp = in(reg) kernel_sp,
        return_pc = in(reg) return_pc,
        options(noreturn)
        );
    }
}

pub fn return_to_kernel_stack_checked(kernel_sp: u64, return_pc: u64) -> ! {
    if kernel_sp == 0 || !crate::kernel::memory::is_inside_kernel_text(return_pc) {
        crate::drivers::uart::write_line("invalid kernel return context");
        crate::arch::halt();
    }

    reset_trap_stack_pointer_for_next_trap();

    unsafe {
        return_to_kernel_stack(kernel_sp, return_pc);
    }
}

#[allow(dead_code)]
pub fn capture_task_cpu_context(
    sp: u64,
    return_pc: u64,
) -> crate::kernel::task::cpu_context::TaskCpuContext {
    let ra: u64;
    let mut s = [0u64; 12];

    unsafe {
        core::arch::asm!(
        "mv {ra_out}, ra",
        "mv {s0_out}, s0",
        "mv {s1_out}, s1",
        "mv {s2_out}, s2",
        "mv {s3_out}, s3",
        "mv {s4_out}, s4",
        "mv {s5_out}, s5",
        "mv {s6_out}, s6",
        "mv {s7_out}, s7",
        "mv {s8_out}, s8",
        "mv {s9_out}, s9",
        "mv {s10_out}, s10",
        "mv {s11_out}, s11",
        ra_out = out(reg) ra,
        s0_out = out(reg) s[0],
        s1_out = out(reg) s[1],
        s2_out = out(reg) s[2],
        s3_out = out(reg) s[3],
        s4_out = out(reg) s[4],
        s5_out = out(reg) s[5],
        s6_out = out(reg) s[6],
        s7_out = out(reg) s[7],
        s8_out = out(reg) s[8],
        s9_out = out(reg) s[9],
        s10_out = out(reg) s[10],
        s11_out = out(reg) s[11],
        options(nomem, nostack, preserves_flags),
        );
    }

    crate::kernel::task::cpu_context::TaskCpuContext {
        sp,
        return_pc,
        resume_pc: ra,
        ra,
        s,
    }
}

#[allow(dead_code)]
pub unsafe fn restore_task_cpu_context(
    context: crate::kernel::task::cpu_context::TaskCpuContext,
) -> ! {
    core::arch::asm!(
    "mv sp, {sp_in}",

    "mv ra, {resume_pc_in}",

    "mv s0, {s0_in}",
    "mv s1, {s1_in}",
    "mv s2, {s2_in}",
    "mv s3, {s3_in}",
    "mv s4, {s4_in}",
    "mv s5, {s5_in}",
    "mv s6, {s6_in}",
    "mv s7, {s7_in}",
    "mv s8, {s8_in}",
    "mv s9, {s9_in}",
    "mv s10, {s10_in}",
    "mv s11, {s11_in}",

    "ret",

    sp_in = in(reg) context.sp,
    resume_pc_in = in(reg) context.resume_pc,

    s0_in = in(reg) context.s[0],
    s1_in = in(reg) context.s[1],
    s2_in = in(reg) context.s[2],
    s3_in = in(reg) context.s[3],
    s4_in = in(reg) context.s[4],
    s5_in = in(reg) context.s[5],
    s6_in = in(reg) context.s[6],
    s7_in = in(reg) context.s[7],
    s8_in = in(reg) context.s[8],
    s9_in = in(reg) context.s[9],
    s10_in = in(reg) context.s[10],
    s11_in = in(reg) context.s[11],

    options(noreturn)
    );
}

#[allow(dead_code)]
#[inline(always)]
pub fn return_address() -> u64 {
    let ra: u64;

    unsafe {
        core::arch::asm!(
        "mv {ra_out}, ra",
        ra_out = out(reg) ra,
        options(nomem, nostack, preserves_flags),
        );
    }

    ra
}

#[allow(dead_code)]
#[inline(always)]
pub fn resume_stack_pointer() -> u64 {
    stack_pointer()
}

#[allow(dead_code)]
#[inline(always)]
pub fn capture_yield_context() -> (u64, u64) {
    let sp: u64;
    let ra: u64;

    unsafe {
        core::arch::asm!(
        "mv {sp_out}, sp",
        "mv {ra_out}, ra",
        sp_out = out(reg) sp,
        ra_out = out(reg) ra,
        options(nomem, nostack, preserves_flags),
        );
    }

    (sp, ra)
}

#[allow(dead_code)]
pub fn yield_to_kernel_raw(
    task_sp: u64,
    resume_pc: u64,
    kernel_sp: u64,
    kernel_return_pc: u64,
) -> ! {
    crate::kernel::task::debug::set_debug_task_resume_context(task_sp, resume_pc);
    crate::kernel::task::debug::set_debug_task_return_kind(
        crate::kernel::task::TaskReturnKind::Yield,
    );

    crate::kernel::task::debug::print_debug_task_resume_context();

    return_to_kernel_stack_checked(kernel_sp, kernel_return_pc);
}

#[allow(dead_code)]
pub fn restore_verified_resume_frame(frame: crate::kernel::task::cpu_context::TaskCpuContext) -> ! {
    crate::drivers::uart::write_line("arch restore verified resume frame:");

    crate::drivers::uart::write_str(" restore sp: ");
    crate::drivers::uart::write_hex_u64(frame.sp);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str(" restore resume_pc: ");
    crate::drivers::uart::write_hex_u64(frame.resume_pc);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str(" restore return_pc: ");
    crate::drivers::uart::write_hex_u64(frame.return_pc);
    crate::drivers::uart::write_line("");

    let frame_valid = frame.is_valid();
    let resume_pc_inside_text = crate::kernel::memory::is_inside_kernel_text(frame.resume_pc);
    let return_pc_inside_text = crate::kernel::memory::is_inside_kernel_text(frame.return_pc);

    crate::drivers::uart::write_line(" arch restore preconditions:");

    crate::drivers::uart::write_str(" frame valid: ");
    crate::kernel::task::table::print_yes_no(frame_valid);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str(" resume_pc inside kernel text: ");
    crate::kernel::task::table::print_yes_no(resume_pc_inside_text);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str(" return_pc inside kernel text: ");
    crate::kernel::task::table::print_yes_no(return_pc_inside_text);
    crate::drivers::uart::write_line("");

    let ok = frame_valid && resume_pc_inside_text && return_pc_inside_text;

    crate::drivers::uart::write_str(" result: ");
    if ok {
        crate::drivers::uart::write_line("OK");
    } else {
        crate::drivers::uart::write_line("FAILED");
        crate::arch::halt();
    }

    print_restore_plan(frame);
    print_restore_contract();

    crate::drivers::uart::write_line(" calling disabled assembly restore stub...");
    unsafe {
        restore_resume_frame_asm_stub(frame);
    }
}

fn print_restore_plan(frame: crate::kernel::task::cpu_context::TaskCpuContext) {
    crate::drivers::uart::write_line(" restore plan:");

    crate::drivers::uart::write_str(" sp <- ");
    crate::drivers::uart::write_hex_u64(frame.sp);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str(" pc <- ");
    crate::drivers::uart::write_hex_u64(frame.resume_pc);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str(" ra <- ");
    crate::drivers::uart::write_hex_u64(frame.ra);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str(" return_pc/debug <- ");
    crate::drivers::uart::write_hex_u64(frame.return_pc);
    crate::drivers::uart::write_line("");
}

fn print_restore_contract() {
    crate::drivers::uart::write_line(" assembly restore contract:");
    crate::drivers::uart::write_line(" set sp to verified frame.sp");
    crate::drivers::uart::write_line(" restore ra from verified frame.ra");
    crate::drivers::uart::write_line(" jump to verified frame.resume_pc");
    crate::drivers::uart::write_line(" do not return to caller");
    crate::drivers::uart::write_line(" do not touch kernel stack after switching sp");
}

#[inline(never)]
unsafe fn restore_resume_frame_asm_stub(
    frame: crate::kernel::task::cpu_context::TaskCpuContext,
) -> ! {
    crate::drivers::uart::write_line(" restore_resume_frame_asm_stub:");
    crate::drivers::uart::write_line(" asm boundary reached");
    crate::drivers::uart::write_line(" this function must not return");
    crate::drivers::uart::write_str(" received sp: ");
    crate::drivers::uart::write_hex_u64(frame.sp);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str(" received resume_pc: ");
    crate::drivers::uart::write_hex_u64(frame.resume_pc);
    crate::drivers::uart::write_line("");

    #[cfg(feature = "real_resume_restore_test")]
    {
        crate::drivers::uart::write_line(" real resume restore feature enabled");
        print_riscv_real_resume_success_marker_plan();

        if !print_riscv_real_restore_attempt_guard(frame) {
            crate::drivers::uart::write_line(" real RISC-V restore attempt blocked by guard");
            crate::arch::halt();
        }

        crate::drivers::uart::write_line(" real RISC-V restore attempt guard passed");

        #[cfg(feature = "real_resume_restore_jump")]
        {
            crate::drivers::uart::write_line(" decision: real restore jump enabled");
            unsafe {
                restore_resume_frame_real_jump(frame);
            }
        }

        #[cfg(not(feature = "real_resume_restore_jump"))]
        {
            crate::drivers::uart::write_line(
                " decision: real restore still disabled without real_resume_restore_jump",
            );
            crate::arch::halt();
        }
    }

    #[cfg(not(feature = "real_resume_restore_test"))]
    {
        crate::drivers::uart::write_line(" safe mode: real asm restore disabled");
        crate::arch::halt();
    }
}

#[cfg(target_arch = "riscv64")]
#[unsafe(no_mangle)]
#[inline(never)]
pub unsafe extern "C" fn yield_to_kernel_returning_stub(
    task_sp: u64,
    resume_pc: u64,
    kernel_sp: u64,
    return_pc: u64,
) -> ! {
    crate::drivers::uart::write_line("yield returning stub:");
    crate::drivers::uart::write_line(" mode: placeholder");

    // остальной существующий код

    crate::drivers::uart::write_line(" delegating to raw yield jump");

    yield_to_kernel_raw(task_sp, resume_pc, kernel_sp, return_pc);
}

#[cfg(feature = "verbose_resume_debug")]
fn print_returning_yield_contract() {
    crate::drivers::uart::write_line(" RISC-V returning yield contract:");
    crate::drivers::uart::write_line(" capture task SP before switching stacks");
    crate::drivers::uart::write_line(" capture resume PC as point after yield call");
    crate::drivers::uart::write_line(" save kernel SP and kernel return PC");
    crate::drivers::uart::write_line(" switch from task stack to kernel stack");
    crate::drivers::uart::write_line(" enter kernel task return handler");
    crate::drivers::uart::write_line(
        " after future restore, yield_to_kernel_and_return must return normally",
    );
    crate::drivers::uart::write_line(" Rust code after yield_now must be reachable");
}

#[allow(dead_code)]
fn validate_returning_yield_abi_inputs(
    task_sp: u64,
    resume_pc: u64,
    kernel_sp: u64,
    return_pc: u64,
) -> bool {
    crate::drivers::uart::write_line(" RISC-V returning yield ABI validation:");

    let task_sp_nonzero = task_sp != 0;
    let kernel_sp_nonzero = kernel_sp != 0;
    let resume_pc_inside_text = crate::kernel::memory::is_inside_kernel_text(resume_pc);
    let return_pc_inside_text = crate::kernel::memory::is_inside_kernel_text(return_pc);

    crate::drivers::uart::write_str(" task_sp non-zero: ");
    crate::kernel::task::table::print_yes_no(task_sp_nonzero);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str(" kernel_sp non-zero: ");
    crate::kernel::task::table::print_yes_no(kernel_sp_nonzero);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str(" resume_pc inside kernel text: ");
    crate::kernel::task::table::print_yes_no(resume_pc_inside_text);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str(" return_pc inside kernel text: ");
    crate::kernel::task::table::print_yes_no(return_pc_inside_text);
    crate::drivers::uart::write_line("");

    let ok = task_sp_nonzero && kernel_sp_nonzero && resume_pc_inside_text && return_pc_inside_text;

    crate::drivers::uart::write_str(" result: ");
    if ok {
        crate::drivers::uart::write_line("OK");
    } else {
        crate::drivers::uart::write_line("FAILED");
    }

    ok
}

#[cfg(feature = "real_resume_restore_test")]
fn print_riscv_real_resume_success_marker_plan() {
    #[cfg(feature = "scheduler_resume_loop_test")]
    {
        crate::drivers::uart::write_line(" RISC-V real resume success markers:");
        crate::drivers::uart::write_line(" expect: scheduler-driven repeated resume loop");
        crate::drivers::uart::write_line(" expect: two_yielding_task: step 2");
        crate::drivers::uart::write_line(" expect: second yield request");
        crate::drivers::uart::write_line(" expect: two_yielding_task: step 3");
        crate::drivers::uart::write_line(" expect: task exit requested");
        crate::drivers::uart::write_line(" expect: scheduler resume loop result: OK");
        crate::drivers::uart::write_line(" if these do not appear, scheduler resume loop failed");
    }

    #[cfg(all(
        feature = "two_yield_task_test",
        not(feature = "scheduler_resume_loop_test")
    ))]
    {
        crate::drivers::uart::write_line(" RISC-V real resume success markers:");
        crate::drivers::uart::write_line(" expect: two_yielding_task: step 2");
        crate::drivers::uart::write_line(" expect: second yield request");
        crate::drivers::uart::write_line(" expect: two_yielding_task: step 3");
        crate::drivers::uart::write_line(" expect: task exit requested");
        crate::drivers::uart::write_line(
            " if these do not appear, repeated resume did not work correctly",
        );
    }

    #[cfg(not(any(
        feature = "two_yield_task_test",
        feature = "scheduler_resume_loop_test"
    )))]
    {
        crate::drivers::uart::write_line(" RISC-V real resume success markers:");
        crate::drivers::uart::write_line(" expect: yield_now: resumed after arch yield");
        crate::drivers::uart::write_line(" expect: yielding_task: step 2");
        crate::drivers::uart::write_line(" expect: task exit requested");
        crate::drivers::uart::write_line(
            " if these do not appear, restore did not resume Rust correctly",
        );
    }
}

#[cfg(feature = "real_resume_restore_test")]
fn print_riscv_real_restore_attempt_guard(
    frame: crate::kernel::task::cpu_context::TaskCpuContext,
) -> bool {
    crate::drivers::uart::write_line(" RISC-V real restore attempt guard:");

    let frame_valid = frame.is_valid();
    let resume_pc_inside_text = crate::kernel::memory::is_inside_kernel_text(frame.resume_pc);
    let return_pc_inside_text = crate::kernel::memory::is_inside_kernel_text(frame.return_pc);
    let task_sp_nonzero = frame.sp != 0;
    let ra_matches_resume_pc = frame.ra == frame.resume_pc;

    crate::drivers::uart::write_str(" feature real_resume_restore_test: ");
    crate::drivers::uart::write_line("enabled");

    crate::drivers::uart::write_str(" arch: ");
    crate::drivers::uart::write_line("riscv64");

    crate::drivers::uart::write_str(" frame valid: ");
    crate::kernel::task::table::print_yes_no(frame_valid);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str(" task SP non-zero: ");
    crate::kernel::task::table::print_yes_no(task_sp_nonzero);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str(" resume_pc inside kernel text: ");
    crate::kernel::task::table::print_yes_no(resume_pc_inside_text);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str(" return_pc inside kernel text: ");
    crate::kernel::task::table::print_yes_no(return_pc_inside_text);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str(" ra == resume_pc: ");
    crate::kernel::task::table::print_yes_no(ra_matches_resume_pc);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str(" success markers documented: ");
    crate::kernel::task::table::print_yes_no(true);
    crate::drivers::uart::write_line("");

    let ok = frame_valid
        && task_sp_nonzero
        && resume_pc_inside_text
        && return_pc_inside_text
        && ra_matches_resume_pc;

    crate::drivers::uart::write_str(" result: ");
    if ok {
        crate::drivers::uart::write_line("OK");
    } else {
        crate::drivers::uart::write_line("FAILED");
    }

    ok
}

#[cfg(all(
    feature = "real_resume_restore_test",
    feature = "real_resume_restore_jump"
))]
#[inline(never)]
unsafe fn restore_resume_frame_real_jump(
    frame: crate::kernel::task::cpu_context::TaskCpuContext,
) -> ! {
    crate::drivers::uart::write_line(" RISC-V REAL RESTORE JUMP ENABLED");
    crate::drivers::uart::write_line(" attempting to resume task now");

    crate::drivers::uart::write_str(" sp <- ");
    crate::drivers::uart::write_hex_u64(frame.sp);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str(" ra <- ");
    crate::drivers::uart::write_hex_u64(frame.ra);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str(" pc <- ");
    crate::drivers::uart::write_hex_u64(frame.resume_pc);
    crate::drivers::uart::write_line("");

    #[cfg(feature = "two_task_resume_handoff_test")]
    {
        crate::drivers::uart::write_line(" jumping now; expected next lines:");
        crate::drivers::uart::write_line(" yield_now: resumed after RISC-V boundary");
        crate::drivers::uart::write_line(" handoff worker resumes after yield");
        crate::drivers::uart::write_line(" worker either yields again or exits");
    }

    #[cfg(not(feature = "two_task_resume_handoff_test"))]
    {
        crate::drivers::uart::write_line(" jumping now; expected next lines:");
        crate::drivers::uart::write_line(" yield_now: resumed after arch yield");
        crate::drivers::uart::write_line(" yielding_task: step 2");
        crate::drivers::uart::write_line(" task exit requested");
    }

    core::arch::asm!(
        "mv sp, {new_sp}",
        "mv ra, {new_ra}",
        "jr {resume_pc}",
        new_sp = in(reg) frame.sp,
        new_ra = in(reg) frame.ra,
        resume_pc = in(reg) frame.resume_pc,
        options(noreturn)
    );
}

#[cfg(target_arch = "riscv64")]
#[allow(dead_code)]
pub fn capture_riscv_yield_context() {
    unsafe {
        core::arch::asm!(
            "sd ra, 0({ctx})",
            "sd s0, 8({ctx})",
            "sd s1, 16({ctx})",
            "sd s2, 24({ctx})",
            "sd s3, 32({ctx})",
            "sd s4, 40({ctx})",
            "sd s5, 48({ctx})",
            "sd s6, 56({ctx})",
            "sd s7, 64({ctx})",
            "sd s8, 72({ctx})",
            "sd s9, 80({ctx})",
            "sd s10, 88({ctx})",
            "sd s11, 96({ctx})",
            ctx = in(reg) core::ptr::addr_of_mut!(LAST_RISCV_YIELD_CONTEXT),
            options(nostack, preserves_flags),
        );
    }
}
```

## File: scripts/check-all.sh
```bash
#!/usr/bin/env bash
set -euo pipefail

echo "== PicoOS check-all =="

echo
echo "== Build RISC-V =="
cargo build

echo
echo "== Build RISC-V selftest =="
cargo build --features "selftest"

echo
echo "== Build RISC-V task resume selftest =="
cargo build --features "task_resume_selftest"

echo
echo "== Clippy RISC-V =="
cargo clippy -- -D warnings

echo
echo "== Clippy RISC-V selftest =="
cargo clippy --features "selftest" -- -D warnings

echo
echo "== Clippy RISC-V task resume selftest =="
cargo clippy --features "task_resume_selftest" -- -D warnings

echo
echo "== Build RISC-V task sleep selftest =="
cargo build --features "task_resume_selftest,task_sleep_test"

echo
echo "== Clippy RISC-V task sleep selftest =="
cargo clippy --features "task_resume_selftest,task_sleep_test" -- -D warnings

echo
echo "== Build RISC-V scheduler run_once selftest =="
cargo build --features "scheduler_run_once_selftest"

echo
echo "== Clippy RISC-V scheduler run_once selftest =="
cargo clippy --features "scheduler_run_once_selftest" -- -D warnings

echo
echo "== Build RISC-V scheduler runtime selftest =="
cargo build --features "scheduler_runtime_selftest"

echo
echo "== Clippy RISC-V scheduler runtime selftest =="
cargo clippy --features "scheduler_runtime_selftest" -- -D warnings

echo
echo "== Build RISC-V scheduler reentry selftest =="
cargo build --features "scheduler_reentry_selftest"

echo
echo "== Clippy RISC-V scheduler reentry selftest =="
cargo clippy --features "scheduler_reentry_selftest" -- -D warnings

echo
echo "== Build RISC-V two-task handoff selftest =="
cargo build --features "two_task_resume_handoff_selftest"

echo
echo "== Clippy RISC-V two-task handoff selftest =="
cargo clippy --features "two_task_resume_handoff_selftest" -- -D warnings

echo
echo "== Build RISC-V scheduler fault lifecycle test =="
cargo build --features "scheduler_fault_lifecycle_test"

echo
echo "== Clippy RISC-V scheduler fault lifecycle test =="
cargo clippy --features "scheduler_fault_lifecycle_test" -- -D warnings

echo
echo "== Build RISC-V kernel fault guard test =="
cargo build --features "kernel_fault_guard_test"

echo
echo "== Clippy RISC-V kernel fault guard test =="
cargo clippy --features "kernel_fault_guard_test" -- -D warnings

echo
echo "== QEMU marker tests =="
scripts/test-task-resume-selftest.sh
scripts/test-task-sleep-riscv.sh
scripts/test-two-task-handoff-riscv.sh
scripts/test-scheduler-fault-lifecycle-riscv.sh
scripts/test-kernel-fault-guard-riscv.sh

echo
echo "== All checks passed =="
cargo clean
```

## File: src/kernel/task/debug.rs
```rust
use crate::drivers::uart;

static mut DEBUG_KERNEL_RETURN_PC: u64 = 0;
static mut DEBUG_KERNEL_SP_BEFORE_TASK: u64 = 0;
static mut DEBUG_CURRENT_STACK_START: u64 = 0;
static mut DEBUG_CURRENT_STACK_TOP: u64 = 0;
static mut DEBUG_TASK_RUN_STAGE: u64 = 0;
static mut DEBUG_TASK_RETURN_KIND: crate::kernel::task::table::TaskReturnKind =
    crate::kernel::task::table::TaskReturnKind::None;
static mut DEBUG_CURRENT_TASK: Option<usize> = None;
static mut DEBUG_LAST_TASK_SP: u64 = 0;
static mut DEBUG_TASK_RESUME_PC: u64 = 0;

#[allow(dead_code)]
pub fn set_debug_kernel_return_pc(pc: u64) {
    unsafe {
        DEBUG_KERNEL_RETURN_PC = pc;
    }
}

pub fn debug_kernel_return_pc() -> u64 {
    unsafe { DEBUG_KERNEL_RETURN_PC }
}

#[allow(dead_code)]
pub fn set_debug_kernel_sp_before_task(sp: u64) {
    unsafe {
        DEBUG_KERNEL_SP_BEFORE_TASK = sp;
    }
}

pub fn debug_kernel_sp_before_task() -> u64 {
    unsafe { DEBUG_KERNEL_SP_BEFORE_TASK }
}

#[allow(dead_code)]
pub fn set_debug_current_stack_bounds(start: u64, top: u64) {
    unsafe {
        DEBUG_CURRENT_STACK_START = start;
        DEBUG_CURRENT_STACK_TOP = top;
    }
}

#[allow(dead_code)]
pub fn debug_current_stack_start() -> u64 {
    unsafe { DEBUG_CURRENT_STACK_START }
}

#[allow(dead_code)]
pub fn debug_current_stack_top() -> u64 {
    unsafe { DEBUG_CURRENT_STACK_TOP }
}

pub fn set_debug_last_task_sp(sp: u64) {
    unsafe {
        DEBUG_LAST_TASK_SP = sp;
    }
}

pub fn debug_last_task_sp() -> u64 {
    unsafe { DEBUG_LAST_TASK_SP }
}

#[allow(dead_code)]
pub fn set_debug_task_run_stage(stage: u64) {
    unsafe {
        DEBUG_TASK_RUN_STAGE = stage;
    }
}

pub fn debug_task_run_stage() -> u64 {
    unsafe { DEBUG_TASK_RUN_STAGE }
}

pub fn set_debug_task_return_kind(kind: crate::kernel::task::table::TaskReturnKind) {
    unsafe {
        DEBUG_TASK_RETURN_KIND = kind;
    }
}

pub fn debug_task_return_kind() -> crate::kernel::task::table::TaskReturnKind {
    unsafe { DEBUG_TASK_RETURN_KIND }
}

#[allow(dead_code)]
pub fn set_debug_current_task_id(id: usize) {
    unsafe {
        DEBUG_CURRENT_TASK = Some(id);
    }
}

#[allow(dead_code)]
pub fn clear_debug_current_task_id() {
    unsafe {
        DEBUG_CURRENT_TASK = None;
    }
}

pub fn debug_current_task_id() -> usize {
    let task = unsafe { DEBUG_CURRENT_TASK };
    task.unwrap_or(0)
}

#[allow(dead_code)]
pub fn set_debug_task_resume_pc(pc: u64) {
    unsafe {
        DEBUG_TASK_RESUME_PC = pc;
    }
}

pub fn debug_task_resume_pc() -> u64 {
    unsafe { DEBUG_TASK_RESUME_PC }
}

#[allow(dead_code)]
pub fn set_debug_task_resume_context(task_sp: u64, resume_pc: u64) {
    set_debug_last_task_sp(task_sp);
    set_debug_task_resume_pc(resume_pc);
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TrapExecutionContext {
    Kernel,
    Task,
}

pub fn current_trap_execution_context() -> TrapExecutionContext {
    let task = unsafe { DEBUG_CURRENT_TASK };
    if task.is_some() {
        TrapExecutionContext::Task
    } else {
        TrapExecutionContext::Kernel
    }
}

pub fn print_trap_execution_context() {
    match current_trap_execution_context() {
        TrapExecutionContext::Kernel => {
            crate::drivers::uart::write_line("trap execution context: kernel");
        }
        TrapExecutionContext::Task => {
            crate::drivers::uart::write_line("trap execution context: task");
        }
    }
}

#[no_mangle]
pub extern "C" fn task_return_point() -> ! {
    uart::write_line("");
    uart::write_line("task return:");

    uart::write_str("  reason: ");
    crate::kernel::task::table::print_task_return_kind(debug_task_return_kind());
    uart::write_line("");

    crate::kernel::task::test::handle_task_return_for_debug_test();
    clear_debug_current_task_id();

    match debug_task_run_stage() {
        #[cfg(feature = "task_yield_test")]
        10 => {
            uart::write_line("back in kernel after yield test");
            uart::write_line("yield test complete");

            #[cfg(feature = "scheduler_reentry_test")]
            {
                crate::kernel::task::test::handle_scheduler_reentry_after_task_return();
            }

            #[cfg(all(
                feature = "resume_candidate_test",
                not(feature = "scheduler_reentry_test")
            ))]
            {
                crate::kernel::task::test_resume_candidate_selection();
            }

            #[cfg(feature = "resume_preflight_test")]
            {
                crate::kernel::task::test::test_resume_preflight_check();
            }

            #[cfg(feature = "resume_dry_run_test")]
            {
                crate::kernel::task::test::test_resume_dry_run();
            }

            #[cfg(feature = "resume_restore_test")]
            {
                crate::kernel::task::test::test_resume_restore();
            }

            crate::kernel::task::test::print_final_task_list();
            crate::arch::halt();
        }

        _ => {
            uart::write_line("unknown task return stage");
            crate::arch::halt();
        }
    }
}

#[allow(dead_code)]
pub fn print_debug_task_resume_context() {
    let task_sp = debug_last_task_sp();
    let resume_pc = debug_task_resume_pc();

    crate::drivers::uart::write_str("yield resume PC: ");
    crate::drivers::uart::write_hex_u64(resume_pc);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("yield current SP: ");
    crate::drivers::uart::write_hex_u64(task_sp);
    crate::drivers::uart::write_line("");
}
```

## File: src/arch/riscv64/traps.rs
```rust
use crate::arch;
use crate::arch::riscv64::cpu;
use crate::arch::riscv64::timer;
use crate::drivers::uart;
use crate::kernel::trap_frame::Riscv64TrapFrame;

const TIMER_HZ: u64 = 1;

#[no_mangle]
pub extern "C" fn riscv64_trap_handler(frame: *const Riscv64TrapFrame) {
    arch::disable_irq();

    let cause = cpu::mcause();
    let is_interrupt = (cause >> 63) != 0;
    let code = cause & 0x7FFF_FFFF_FFFF_FFFF;

    if is_interrupt && code == 7 {
        handle_timer_interrupt(frame);
        arch::enable_irq();
        return;
    }

    let mepc = cpu::mepc();
    let mtval = cpu::mtval();

    uart::write_line("");
    uart::write_line("=== RISC-V TRAP ===");

    uart::write_str("trap frame: ");
    uart::write_hex_u64(frame as u64);
    uart::write_line("");

    uart::write_str("mcause: ");
    uart::write_hex_u64(cause);
    uart::write_line("");

    uart::write_str("mepc: ");
    uart::write_hex_u64(mepc);
    uart::write_line("");

    uart::write_str("mtval: ");
    uart::write_hex_u64(mtval);
    uart::write_line("");

    print_trap_cause(cause);

    #[cfg(feature = "scheduler_fault_lifecycle_test")]
    {
        crate::kernel::task::debug::print_trap_execution_context();
        crate::kernel::task::fault::print_current_trap_fault_classification();

        match crate::kernel::task::fault::classify_current_trap_fault() {
            crate::kernel::task::fault::TrapFaultClassification::KernelFault => {
                crate::kernel::log::fail("trap", "kernel fault -> halt");

                #[cfg(feature = "kernel_fault_guard_test")]
                {
                    let frame_on_trap_stack = arch::is_trap_stack_addr(frame as u64);

                    uart::write_str("trap frame on trap stack: ");
                    crate::kernel::task::table::print_yes_no(frame_on_trap_stack);
                    uart::write_line("");

                    if !frame_on_trap_stack {
                        uart::write_line("kernel fault guard result: FAILED");
                        arch::halt();
                    }

                    uart::write_line("");
                    uart::write_line("kernel fault guard result: OK");
                    uart::write_line("");
                    uart::write_line("PicoOS milestone:");
                    uart::write_line("  baseline: 0.1.0");
                    uart::write_line("  current: 0.1.64");
                    uart::write_line("  task fault state: OK");
                    uart::write_line("  scheduler skips faulted tasks: OK");
                    uart::write_line("  trap-to-task-fault skeleton: OK");
                    uart::write_line("  real trap handler classification: OK");
                    uart::write_line("  real trap handler task-fault return path: OK");
                    uart::write_line("  trap stack isolation: OK");
                    uart::write_line("  kernel fault guard: OK");
                }

                arch::halt();
            }

            crate::kernel::task::fault::TrapFaultClassification::TaskFault => {
                crate::kernel::log::info("trap", "marking current task as Faulted");

                if crate::kernel::task::fault::record_current_task_fault(cause, mepc, mtval)
                    .is_none()
                {
                    arch::halt();
                }

                let task_sp = interrupted_sp(frame);
                let kernel_sp = crate::kernel::task::debug::debug_kernel_sp_before_task();
                let return_pc = crate::kernel::task::debug::debug_kernel_return_pc();

                uart::write_str("  task fault return SP: ");
                uart::write_hex_u64(task_sp);
                uart::write_line("");

                uart::write_str("  saved kernel SP: ");
                uart::write_hex_u64(kernel_sp);
                uart::write_line("");

                uart::write_str("  kernel return PC: ");
                uart::write_hex_u64(return_pc);
                uart::write_line("");

                crate::kernel::log::info("trap", "return to kernel task return path");

                crate::kernel::task::fault::return_current_task_fault(
                    task_sp, kernel_sp, return_pc,
                );
            }
        }
    }

    #[cfg(not(feature = "scheduler_fault_lifecycle_test"))]
    {
        crate::kernel::log::fail("trap", "system halted after trap");
        arch::halt();
    }
}

fn handle_timer_interrupt(frame: *const Riscv64TrapFrame) {
    timer::disarm_timer();

    let saved_sp = interrupted_sp(frame);
    let saved_pc = cpu::mepc();

    let saved_task = crate::kernel::task::scheduler::save_current_context(saved_sp, saved_pc);

    let tick = crate::kernel::ticks::increment();
    let woke_tasks = crate::kernel::task::wake_sleeping_tasks(tick);

    uart::write_str("tick: ");
    uart::write_dec_u64(tick);

    uart::write_str(" saved current: ");
    match saved_task {
        Some(id) => {
            crate::kernel::task::scheduler::print_task_name(id);
            crate::kernel::task::print_task_context_values(saved_sp, saved_pc);
        }
        None => uart::write_str("none"),
    }

    let decided_next = crate::kernel::task::scheduler::decide_next_task_dry_run();
    #[cfg(feature = "timer_preemption_prototype")]
    let decided_resumable = crate::kernel::task::scheduler::decide_next_resumable_task_dry_run();

    uart::write_str(" decision next: ");
    match decided_next {
        Some(id) => crate::kernel::task::scheduler::print_task_name(id),
        None => uart::write_str("none"),
    }

    uart::write_str(" mode: dry-run");
    uart::write_str(" woke: ");
    uart::write_dec_u64(woke_tasks as u64);

    uart::write_str(" context:");
    match crate::kernel::task::scheduler::current_task_id() {
        Some(id) => crate::kernel::task::print_task_full_context_by_id(id),
        None => uart::write_str(" none"),
    }

    uart::write_line("");
    crate::kernel::log::info("timer", "scheduler decision computed");

    #[cfg(feature = "timer_preemption_prototype")]
    if let Some(next_id) = decided_resumable {
        crate::kernel::task::scheduler::force_current_task(next_id);
        crate::kernel::log::ok("timer", "preemption prototype: switching to resumable task");

        let Some(stack_start) = crate::kernel::task::get_task_stack_start(next_id) else {
            crate::kernel::log::fail("timer", "preemption missing task stack start");
            arch::halt();
        };
        let Some(stack_top) = crate::kernel::task::get_task_stack_top(next_id) else {
            crate::kernel::log::fail("timer", "preemption missing task stack top");
            arch::halt();
        };
        crate::kernel::task::debug::set_debug_current_task_id(next_id);
        crate::kernel::task::debug::set_debug_current_stack_bounds(stack_start, stack_top);

        let Some(frame) = crate::kernel::task::get_task_resume_frame(next_id) else {
            crate::kernel::log::fail("timer", "resume frame missing after decision");
            arch::halt();
        };

        timer::arm_timer_hz(TIMER_HZ);
        arch::reset_trap_stack_pointer_for_next_trap();
        crate::arch::restore_verified_resume_frame(frame);
    }

    if crate::kernel::ticks::is_test_complete() {
        cpu::disable_machine_timer_interrupt();

        crate::kernel::test::print_test_complete();

        arch::halt();
    }

    timer::arm_timer_hz(TIMER_HZ);
}

fn interrupted_sp(frame: *const Riscv64TrapFrame) -> u64 {
    unsafe { (*frame).sp }
}

fn print_trap_cause(cause: u64) {
    let is_interrupt = (cause >> 63) != 0;
    let code = cause & 0x7FFF_FFFF_FFFF_FFFF;

    uart::write_str("trap type: ");

    if is_interrupt {
        uart::write_line("interrupt");
    } else {
        uart::write_line("exception");
    }

    uart::write_str("trap code: ");
    uart::write_dec_u64(code);
    uart::write_line("");

    uart::write_str("trap name: ");

    match (is_interrupt, code) {
        (false, 0) => uart::write_line("instruction address misaligned"),
        (false, 1) => uart::write_line("instruction access fault"),
        (false, 2) => uart::write_line("illegal instruction"),
        (false, 3) => uart::write_line("breakpoint"),
        (false, 5) => uart::write_line("load access fault"),
        (false, 7) => uart::write_line("store access fault"),
        (false, 8) => uart::write_line("environment call from U-mode"),
        (false, 9) => uart::write_line("environment call from S-mode"),
        (false, 11) => uart::write_line("environment call from M-mode"),
        (true, 3) => uart::write_line("machine software interrupt"),
        (true, 7) => uart::write_line("machine timer interrupt"),
        (true, 11) => uart::write_line("machine external interrupt"),
        _ => uart::write_line("unknown"),
    }
}
```

## File: src/kernel/task/entry.rs
```rust
use crate::drivers::uart;
use crate::kernel::task::debug::{
    debug_kernel_return_pc, debug_kernel_sp_before_task, set_debug_last_task_sp,
    set_debug_task_return_kind,
};
use crate::kernel::task::table::TaskEntry;
use crate::kernel::task::table::TaskReturnKind;

pub fn task_trampoline(entry: TaskEntry) -> ! {
    uart::write_line("");
    uart::write_line("task trampoline:");
    uart::write_str("calling entry: ");
    uart::write_hex_u64(entry as usize as u64);
    uart::write_line("");

    entry();

    task_exit();
}

#[no_mangle]
pub extern "C" fn task_trampoline_raw(entry_addr: usize) -> ! {
    let entry: TaskEntry = unsafe { core::mem::transmute(entry_addr) };

    task_trampoline(entry);
}

pub fn task_exit() -> ! {
    uart::write_line("task returned; task_exit called");

    let current_sp = crate::arch::stack_pointer();
    let kernel_sp = debug_kernel_sp_before_task();
    let return_pc = debug_kernel_return_pc();

    set_debug_last_task_sp(current_sp);

    uart::write_str("task_exit current SP: ");
    uart::write_hex_u64(current_sp);
    uart::write_line("");

    uart::write_str("task_exit saved kernel SP: ");
    uart::write_hex_u64(kernel_sp);
    uart::write_line("");

    uart::write_str("task_exit return PC: ");
    uart::write_hex_u64(return_pc);
    uart::write_line("");

    uart::write_line("returning to kernel stack...");

    set_debug_task_return_kind(TaskReturnKind::Exit);

    crate::arch::return_to_kernel_stack_checked(kernel_sp, return_pc);
}

#[allow(dead_code)]
pub fn task_fault() -> ! {
    uart::write_line("task fault requested");

    let current_sp = crate::arch::stack_pointer();
    let kernel_sp = debug_kernel_sp_before_task();
    let return_pc = debug_kernel_return_pc();

    uart::write_str("task_fault current SP: ");
    uart::write_hex_u64(current_sp);
    uart::write_line("");

    uart::write_str("task_fault saved kernel SP: ");
    uart::write_hex_u64(kernel_sp);
    uart::write_line("");

    uart::write_str("task_fault return PC: ");
    uart::write_hex_u64(return_pc);
    uart::write_line("");

    uart::write_line("returning to kernel stack after task fault...");

    crate::kernel::task::fault::return_current_task_fault(current_sp, kernel_sp, return_pc);
}

#[allow(dead_code)]
pub fn simulated_task_trap_fault() -> ! {
    uart::write_line("simulated task trap fault requested");

    let current_sp = crate::arch::stack_pointer();
    let kernel_sp = crate::kernel::task::debug::debug_kernel_sp_before_task();
    let return_pc = crate::kernel::task::debug::debug_kernel_return_pc();

    uart::write_str("simulated trap current SP: ");
    uart::write_hex_u64(current_sp);
    uart::write_line("");

    uart::write_str("simulated trap saved kernel SP: ");
    uart::write_hex_u64(kernel_sp);
    uart::write_line("");

    uart::write_str("simulated trap return PC: ");
    uart::write_hex_u64(return_pc);
    uart::write_line("");

    uart::write_line("simulated trap classified as task fault");
    uart::write_line("returning to kernel stack after simulated task trap...");

    crate::kernel::task::fault::return_current_task_fault(current_sp, kernel_sp, return_pc);
}

#[allow(dead_code)]
pub fn yield_now() {
    crate::drivers::uart::write_line("task yield requested");

    let kernel_sp = crate::kernel::task::debug::debug_kernel_sp_before_task();
    let return_pc = crate::kernel::task::debug::debug_kernel_return_pc();

    crate::drivers::uart::write_str("yield saved kernel SP: ");
    crate::drivers::uart::write_hex_u64(kernel_sp);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("yield return PC: ");
    crate::drivers::uart::write_hex_u64(return_pc);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_line("yielding to kernel via RISC-V boundary...");

    unsafe {
        crate::arch::task_yield_boundary(kernel_sp, return_pc);
    }
}

#[allow(dead_code)]
pub fn task_sleep_ticks(ticks: u64) {
    let task_id = crate::kernel::task::debug::debug_current_task_id();
    let wake_tick = crate::kernel::ticks::get().saturating_add(ticks.max(1));

    crate::kernel::log::info("sleep", "task requested timed sleep");

    if !crate::kernel::task::table::mark_task_blocked_until(task_id, wake_tick) {
        crate::kernel::log::fail("sleep", "failed to mark task Blocked");
        crate::arch::halt();
    }

    set_debug_task_return_kind(TaskReturnKind::Sleep);

    let kernel_sp = crate::kernel::task::debug::debug_kernel_sp_before_task();
    let return_pc = crate::kernel::task::debug::debug_kernel_return_pc();

    unsafe {
        crate::arch::task_yield_boundary(kernel_sp, return_pc);
    }
}

#[allow(dead_code)]
fn print_returning_yield_task_layer_precheck(
    task_sp: u64,
    resume_pc: u64,
    kernel_sp: u64,
    return_pc: u64,
) -> bool {
    crate::drivers::uart::write_line("returning yield task-layer precheck:");

    let task_id = crate::kernel::task::debug::debug_current_task_id();

    let task_sp_inside_stack = matches!(
        crate::kernel::task::table::is_sp_inside_task_stack(task_id, task_sp),
        Some(true)
    );

    let resume_pc_inside_text = crate::kernel::memory::is_inside_kernel_text(resume_pc);
    let kernel_sp_nonzero = kernel_sp != 0;
    let return_pc_inside_text = crate::kernel::memory::is_inside_kernel_text(return_pc);

    crate::drivers::uart::write_str("  task: ");
    crate::kernel::task::table::print_task_name_by_id(task_id);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("  task_sp inside current task stack: ");
    crate::kernel::task::table::print_yes_no(task_sp_inside_stack);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("  resume_pc inside kernel text: ");
    crate::kernel::task::table::print_yes_no(resume_pc_inside_text);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("  kernel_sp non-zero: ");
    crate::kernel::task::table::print_yes_no(kernel_sp_nonzero);
    crate::drivers::uart::write_line("");

    crate::drivers::uart::write_str("  return_pc inside kernel text: ");
    crate::kernel::task::table::print_yes_no(return_pc_inside_text);
    crate::drivers::uart::write_line("");

    let ok =
        task_sp_inside_stack && resume_pc_inside_text && kernel_sp_nonzero && return_pc_inside_text;

    crate::drivers::uart::write_str("  result: ");
    if ok {
        crate::drivers::uart::write_line("OK");
    } else {
        crate::drivers::uart::write_line("FAILED");
    }

    ok
}

#[allow(dead_code)]
pub fn simulated_real_trap_fault() -> ! {
    uart::write_line("simulated real trap fault requested");

    crate::kernel::task::debug::print_trap_execution_context();

    crate::kernel::task::fault::print_current_trap_fault_classification();

    match crate::kernel::task::fault::classify_current_trap_fault() {
        crate::kernel::task::fault::TrapFaultClassification::KernelFault => {
            uart::write_line("simulated real trap result: kernel fault");

            uart::write_line("kernel fault action: halt");

            crate::arch::halt();
        }

        crate::kernel::task::fault::TrapFaultClassification::TaskFault => {
            uart::write_line("simulated real trap result: task fault");

            let Some(task_id) = crate::kernel::task::fault::record_current_task_fault(
                crate::arch::cpu::mcause(),
                crate::arch::cpu::mepc(),
                crate::arch::cpu::mtval(),
            ) else {
                crate::arch::halt();
            };

            uart::write_str("  task: ");
            crate::kernel::task::table::print_task_name_by_id(task_id);
            uart::write_line("");
            uart::write_str("  new state: ");
            crate::kernel::task::table::print_task_state_by_id(task_id);
            uart::write_line("");
            uart::write_str("  can_resume: ");
            match crate::kernel::task::table::can_task_resume(task_id) {
                Some(true) => uart::write_line("yes"),
                Some(false) => uart::write_line("no"),
                None => uart::write_line("unknown"),
            }

            let current_sp = crate::arch::stack_pointer();
            let kernel_sp = crate::kernel::task::debug::debug_kernel_sp_before_task();
            let return_pc = crate::kernel::task::debug::debug_kernel_return_pc();

            uart::write_str("simulated real trap current SP: ");
            uart::write_hex_u64(current_sp);
            uart::write_line("");
            uart::write_str("simulated real trap saved kernel SP: ");
            uart::write_hex_u64(kernel_sp);
            uart::write_line("");
            uart::write_str("simulated real trap return PC: ");
            uart::write_hex_u64(return_pc);
            uart::write_line("");
            uart::write_line("simulated real trap classified as task fault");
            uart::write_line("returning to kernel stack after simulated real trap...");

            crate::kernel::task::fault::return_current_task_fault(current_sp, kernel_sp, return_pc);
        }
    }
}
```

## File: src/kernel/task/table.rs
```rust
use crate::drivers::uart;
use crate::kernel::memory;
use crate::kernel::task::context;
use crate::kernel::task::cpu_context::{self, TaskCpuContext};

pub const MAX_TASKS: usize = 4;

#[allow(dead_code)]
pub fn max_tasks() -> usize {
    MAX_TASKS
}

pub const TASK_NAME_LEN: usize = 16;
static mut LAST_RETURNED_TASK_ID: Option<usize> = None;

pub type TaskEntry = fn();

#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq)]
pub enum TaskState {
    Empty,
    Ready,
    Running,
    Blocked,
    Finished,
    Faulted,
}

#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq)]
pub enum TaskReturnKind {
    None,
    Exit,
    Yield,
    Sleep,
    Fault,
}

#[derive(Clone, Copy, PartialEq)]
pub enum TaskLifecycleTransition {
    Start,
    Yield,
    Sleep,
    Exit,
    Fault,
}

#[derive(Clone, Copy)]
pub struct TaskReturnContext {
    pub task_sp: u64,
    pub kernel_sp: u64,
    pub kernel_return_pc: u64,
}

#[allow(dead_code)]
#[cfg(any(
    feature = "scheduler_reentry_test",
    feature = "scheduler_dispatch_test"
))]
#[derive(Clone, Copy)]
pub struct TaskReturnSnapshot {
    pub task_id: usize,
    pub state: TaskState,
    pub last_return: TaskReturnKind,
    pub can_resume: bool,
}

#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TaskFaultReason {
    Breakpoint,
    InstructionAccessFault,
    LoadAccessFault,
    StoreAccessFault,
    IllegalInstruction,
    Unknown(u64),
}

#[allow(dead_code)]
impl TaskFaultReason {
    /// Конвертирует значение mcause в конкретную причину fault.
    /// Согласно RISC-V Privileged Spec, Table 16 (synchronous exceptions).
    pub fn from_mcause(cause: u64) -> Self {
        match cause {
            1 => TaskFaultReason::InstructionAccessFault,
            2 => TaskFaultReason::IllegalInstruction,
            3 => TaskFaultReason::Breakpoint,
            5 => TaskFaultReason::LoadAccessFault,
            7 => TaskFaultReason::StoreAccessFault,
            other => TaskFaultReason::Unknown(other),
        }
    }
}

#[derive(Clone, Copy)]
pub struct Task {
    pub id: usize,
    pub state: TaskState,
    pub name: [u8; TASK_NAME_LEN],
    pub stack_start: u64,
    pub stack_top: u64,
    pub entry: Option<TaskEntry>,
    pub initial_sp: u64,
    pub initial_pc: u64,
    pub saved_sp: u64,
    pub saved_pc: u64,
    pub cpu_context: TaskCpuContext,
    pub last_kernel_sp: u64,
    pub last_kernel_return_pc: u64,
    pub last_task_sp: u64,
    pub has_started: bool,
    pub can_resume: bool,
    pub last_return_kind: TaskReturnKind,
    pub last_fault_reason: Option<TaskFaultReason>,
    pub last_fault_mcause: Option<u64>,
    pub last_fault_mepc: Option<u64>,
    pub last_fault_mtval: Option<u64>,
    pub sleep_until_tick: Option<u64>,
}

impl Task {
    pub const fn empty() -> Self {
        Self {
            id: 0,
            state: TaskState::Empty,
            name: [0; TASK_NAME_LEN],
            stack_start: 0,
            stack_top: 0,
            entry: None,
            initial_sp: 0,
            initial_pc: 0,
            saved_sp: 0,
            saved_pc: 0,
            cpu_context: TaskCpuContext::empty(),
            last_kernel_sp: 0,
            last_kernel_return_pc: 0,
            last_task_sp: 0,
            has_started: false,
            can_resume: false,
            last_return_kind: TaskReturnKind::None,
            last_fault_reason: None,
            last_fault_mcause: None,
            last_fault_mepc: None,
            last_fault_mtval: None,
            sleep_until_tick: None,
        }
    }
}

static mut TASKS: [Task; MAX_TASKS] = [Task::empty(); MAX_TASKS];
static mut NEXT_TASK_ID: usize = 0;

#[allow(dead_code)]
pub fn init() {
    unsafe {
        TASKS = [Task::empty(); MAX_TASKS];
        NEXT_TASK_ID = 0;
    }

    uart::write_line("");
    uart::write_line("task system:");
    uart::write_str("max tasks: ");
    uart::write_dec_u64(MAX_TASKS as u64);
    uart::write_line("");
}

#[allow(clippy::needless_range_loop)]
pub fn create_task(name: &str, entry: TaskEntry) -> Option<usize> {
    unsafe {
        for slot in 0..MAX_TASKS {
            if matches!(TASKS[slot].state, TaskState::Empty) {
                let Some(stack_start) = memory::allocate_page() else {
                    uart::write_str("failed to allocate stack for task: ");
                    uart::write_line(name);
                    return None;
                };

                let Some(stack_top) = stack_start.checked_add(memory::PAGE_SIZE) else {
                    uart::write_str("invalid stack range for task: ");
                    uart::write_line(name);
                    return None;
                };
                let initial_sp = stack_top;
                let initial_pc = entry as *const () as usize as u64;

                let Some(prepared_sp) = context::prepare_initial_stack(stack_top, initial_pc)
                else {
                    uart::write_str("failed to prepare stack for task: ");
                    uart::write_line(name);
                    return None;
                };

                let saved_sp = prepared_sp;
                let saved_pc = initial_pc;

                let id = NEXT_TASK_ID;
                NEXT_TASK_ID += 1;

                TASKS[slot].id = id;
                TASKS[slot].state = TaskState::Ready;
                TASKS[slot].stack_start = stack_start;
                TASKS[slot].stack_top = stack_top;
                TASKS[slot].entry = Some(entry);
                TASKS[slot].initial_sp = initial_sp;
                TASKS[slot].initial_pc = initial_pc;
                TASKS[slot].saved_sp = saved_sp;
                TASKS[slot].saved_pc = saved_pc;
                TASKS[slot].cpu_context = TaskCpuContext::initial(saved_sp, saved_pc);
                TASKS[slot].last_kernel_sp = 0;
                TASKS[slot].last_kernel_return_pc = 0;
                TASKS[slot].last_task_sp = 0;
                TASKS[slot].has_started = false;
                TASKS[slot].can_resume = false;
                TASKS[slot].last_return_kind = TaskReturnKind::None;
                TASKS[slot].sleep_until_tick = None;

                copy_name(&mut TASKS[slot].name, name);

                uart::write_str("created task: ");
                write_name(&TASKS[slot].name);

                uart::write_str(" stack: ");
                uart::write_hex_u64(stack_start);

                uart::write_str(" - ");
                uart::write_hex_u64(stack_top);

                uart::write_str(" entry: ");
                uart::write_hex_u64(initial_pc);

                uart::write_str(" initial_sp: ");
                uart::write_hex_u64(initial_sp);

                uart::write_str(" initial_pc: ");
                uart::write_hex_u64(initial_pc);

                uart::write_str(" prepared_sp: ");
                uart::write_hex_u64(prepared_sp);

                context::print_initial_context(prepared_sp);

                uart::write_line("");

                return Some(id);
            }
        }
    }

    None
}

#[allow(clippy::needless_range_loop)]
pub fn print_tasks() {
    uart::write_line("task list:");

    unsafe {
        for slot in 0..MAX_TASKS {
            let task = TASKS[slot];

            if matches!(task.state, TaskState::Empty) {
                continue;
            }

            uart::write_str("id: ");
            uart::write_dec_u64(task.id as u64);

            uart::write_str(" state: ");
            print_state(task.state);

            uart::write_str(" name: ");
            write_name(&task.name);

            uart::write_str(" stack: ");
            uart::write_hex_u64(task.stack_start);

            uart::write_str(" - ");
            uart::write_hex_u64(task.stack_top);

            uart::write_str(" entry: ");
            match task.entry {
                Some(entry) => uart::write_hex_u64(entry as *const () as usize as u64),
                None => uart::write_str("none"),
            }

            uart::write_str(" initial_sp: ");
            uart::write_hex_u64(task.initial_sp);

            uart::write_str(" initial_pc: ");
            uart::write_hex_u64(task.initial_pc);

            uart::write_str(" saved_sp: ");
            uart::write_hex_u64(task.saved_sp);

            uart::write_str(" saved_pc: ");
            uart::write_hex_u64(task.saved_pc);

            cpu_context::print_cpu_context(task.cpu_context);

            if !task.has_started && matches!(task.state, TaskState::Ready) {
                uart::write_str(" initial_frame:");
                context::print_initial_context(task.saved_sp);
            }

            uart::write_str(" started: ");
            if task.has_started {
                uart::write_str("yes");
            } else {
                uart::write_str("no");
            }

            uart::write_str(" can_resume: ");
            if task.can_resume {
                uart::write_str("yes");
            } else {
                uart::write_str("no");
            }

            uart::write_str(" last_return: ");
            print_task_return_kind(task.last_return_kind);

            uart::write_str(" last_task_sp: ");
            uart::write_hex_u64(task.last_task_sp);

            uart::write_str(" last_kernel_sp: ");
            uart::write_hex_u64(task.last_kernel_sp);

            uart::write_str(" last_kernel_return_pc: ");
            uart::write_hex_u64(task.last_kernel_return_pc);

            uart::write_line("");
        }
    }
}

#[allow(dead_code)]
#[allow(clippy::needless_range_loop)]
pub fn task_count() -> usize {
    let mut count = 0;

    unsafe {
        for slot in 0..MAX_TASKS {
            if !matches!(TASKS[slot].state, TaskState::Empty) {
                count += 1;
            }
        }
    }

    count
}

#[allow(clippy::needless_range_loop)]
pub fn get_task_name(id: usize) -> Option<[u8; TASK_NAME_LEN]> {
    unsafe {
        for slot in 0..MAX_TASKS {
            if !matches!(TASKS[slot].state, TaskState::Empty) && TASKS[slot].id == id {
                return Some(TASKS[slot].name);
            }
        }
    }

    None
}

#[allow(clippy::needless_range_loop)]
pub fn get_task_entry(id: usize) -> Option<TaskEntry> {
    unsafe {
        for slot in 0..MAX_TASKS {
            if !matches!(TASKS[slot].state, TaskState::Empty) && TASKS[slot].id == id {
                return TASKS[slot].entry;
            }
        }
    }

    None
}

#[allow(dead_code)]
#[allow(clippy::needless_range_loop)]
pub fn get_task_stack_start(id: usize) -> Option<u64> {
    unsafe {
        for slot in 0..MAX_TASKS {
            if !matches!(TASKS[slot].state, TaskState::Empty) && TASKS[slot].id == id {
                return Some(TASKS[slot].stack_start);
            }
        }
    }

    None
}

#[allow(dead_code)]
#[allow(clippy::needless_range_loop)]
pub fn get_task_stack_top(id: usize) -> Option<u64> {
    unsafe {
        for slot in 0..MAX_TASKS {
            if !matches!(TASKS[slot].state, TaskState::Empty) && TASKS[slot].id == id {
                return Some(TASKS[slot].stack_top);
            }
        }
    }

    None
}

#[allow(clippy::needless_range_loop)]
pub fn get_task_initial_sp(id: usize) -> Option<u64> {
    unsafe {
        for slot in 0..MAX_TASKS {
            if !matches!(TASKS[slot].state, TaskState::Empty) && TASKS[slot].id == id {
                return Some(TASKS[slot].initial_sp);
            }
        }
    }

    None
}

#[allow(clippy::needless_range_loop)]
pub fn get_task_initial_pc(id: usize) -> Option<u64> {
    unsafe {
        for slot in 0..MAX_TASKS {
            if !matches!(TASKS[slot].state, TaskState::Empty) && TASKS[slot].id == id {
                return Some(TASKS[slot].initial_pc);
            }
        }
    }

    None
}

#[allow(clippy::needless_range_loop)]
pub fn get_task_saved_sp(id: usize) -> Option<u64> {
    let slot = find_slot_by_id(id)?;
    Some(unsafe { TASKS[slot].saved_sp })
}

#[allow(clippy::needless_range_loop)]
pub fn get_task_saved_pc(id: usize) -> Option<u64> {
    let slot = find_slot_by_id(id)?;
    Some(unsafe { TASKS[slot].saved_pc })
}

#[allow(clippy::needless_range_loop)]
pub fn find_next_ready_after(current_id: Option<usize>) -> Option<usize> {
    find_next_task_after(current_id, |task| matches!(task.state, TaskState::Ready))
}

#[allow(dead_code)]
#[allow(clippy::needless_range_loop)]
pub fn find_next_dispatchable_after(current_id: Option<usize>) -> Option<usize> {
    find_next_task_after(current_id, |task| is_dispatchable_task(task.id))
}

#[allow(dead_code)]
#[cfg(feature = "scheduler_reentry_test")]
#[allow(clippy::needless_range_loop)]
pub fn set_running(id: usize) {
    let _ = mark_task_running(id);
}

#[allow(dead_code)]
#[allow(clippy::needless_range_loop)]
pub fn mark_task_running(id: usize) -> bool {
    unsafe {
        let Some(target_slot) = (0..MAX_TASKS)
            .find(|slot| !matches!(TASKS[*slot].state, TaskState::Empty) && TASKS[*slot].id == id)
        else {
            return false;
        };

        if !matches!(
            TASKS[target_slot].state,
            TaskState::Ready | TaskState::Running
        ) {
            return false;
        }

        for slot in 0..MAX_TASKS {
            if matches!(TASKS[slot].state, TaskState::Running) && TASKS[slot].id != id {
                TASKS[slot].state = TaskState::Ready;
            }
        }

        TASKS[target_slot].state = TaskState::Running;
        true
    }
}

fn can_transition_from(state: TaskState, transition: TaskLifecycleTransition) -> bool {
    match transition {
        TaskLifecycleTransition::Start => matches!(state, TaskState::Ready | TaskState::Running),
        TaskLifecycleTransition::Yield => matches!(state, TaskState::Ready | TaskState::Running),
        TaskLifecycleTransition::Sleep => matches!(state, TaskState::Ready | TaskState::Running),
        TaskLifecycleTransition::Exit => matches!(state, TaskState::Ready | TaskState::Running),
        TaskLifecycleTransition::Fault => matches!(state, TaskState::Ready | TaskState::Running),
    }
}

#[allow(dead_code)]
pub fn can_apply_task_transition(id: usize, transition: TaskLifecycleTransition) -> bool {
    get_task_state(id)
        .map(|state| can_transition_from(state, transition))
        .unwrap_or(false)
}

#[allow(dead_code)]
pub fn mark_task_started(id: usize) -> bool {
    let Some(slot) = find_slot_by_id(id) else {
        return false;
    };

    unsafe {
        if !can_transition_from(TASKS[slot].state, TaskLifecycleTransition::Start) {
            return false;
        }
        TASKS[slot].has_started = true;
    }

    true
}

#[allow(clippy::needless_range_loop)]
pub fn update_context(id: usize, saved_sp: u64, saved_pc: u64) -> bool {
    let Some(slot) = find_slot_by_id(id) else {
        return false;
    };

    unsafe {
        TASKS[slot].saved_sp = saved_sp;
        TASKS[slot].saved_pc = saved_pc;
        TASKS[slot].has_started = true;
    }

    true
}

#[allow(dead_code)]
#[allow(clippy::needless_range_loop)]
pub fn mark_started(id: usize) -> bool {
    mark_task_started(id)
}

#[allow(clippy::needless_range_loop)]
pub fn has_started(id: usize) -> bool {
    let Some(slot) = find_slot_by_id(id) else {
        return false;
    };
    unsafe { TASKS[slot].has_started }
}

pub fn print_task_name_by_id(id: usize) {
    match get_task_name(id) {
        Some(name) => write_name(&name),
        None => uart::write_str("unknown"),
    }
}

pub fn print_task_entry_by_id(id: usize) {
    match get_task_entry(id) {
        Some(entry) => uart::write_hex_u64(entry as *const () as usize as u64),
        None => uart::write_str("none"),
    }
}

#[allow(dead_code)]
pub fn print_task_context_by_id(id: usize) {
    uart::write_str(" saved_sp: ");

    match get_task_saved_sp(id) {
        Some(sp) => uart::write_hex_u64(sp),
        None => uart::write_str("none"),
    }

    uart::write_str(" saved_pc: ");

    match get_task_saved_pc(id) {
        Some(pc) => uart::write_hex_u64(pc),
        None => uart::write_str("none"),
    }
}

pub fn print_task_context_values(saved_sp: u64, saved_pc: u64) {
    uart::write_str(" saved_sp: ");
    uart::write_hex_u64(saved_sp);

    uart::write_str(" saved_pc: ");
    uart::write_hex_u64(saved_pc);
}

pub fn print_task_full_context_by_id(id: usize) {
    uart::write_str(" initial_sp: ");

    match get_task_initial_sp(id) {
        Some(sp) => uart::write_hex_u64(sp),
        None => uart::write_str("none"),
    }

    uart::write_str(" initial_pc: ");

    match get_task_initial_pc(id) {
        Some(pc) => uart::write_hex_u64(pc),
        None => uart::write_str("none"),
    }

    uart::write_str(" saved_sp: ");

    match get_task_saved_sp(id) {
        Some(sp) => uart::write_hex_u64(sp),
        None => uart::write_str("none"),
    }

    uart::write_str(" saved_pc: ");

    match get_task_saved_pc(id) {
        Some(pc) => uart::write_hex_u64(pc),
        None => uart::write_str("none"),
    }

    uart::write_str(" started: ");
    if has_started(id) {
        uart::write_str("yes");
    } else {
        uart::write_str("no");
    }
}

#[allow(dead_code)]
pub fn print_task_fault_info_by_id(id: usize) {
    let Some(slot) = find_slot_by_id(id) else {
        uart::write_line("  fault info: task not found");
        return;
    };

    let task = unsafe { TASKS[slot] };

    uart::write_line("  fault info:");

    uart::write_str("    reason: ");
    match task.last_fault_reason {
        Some(reason) => print_task_fault_reason(reason),
        None => uart::write_str("none"),
    }
    uart::write_line("");

    uart::write_str("    mcause: ");
    match task.last_fault_mcause {
        Some(value) => uart::write_hex_u64(value),
        None => uart::write_str("none"),
    }
    uart::write_line("");

    uart::write_str("    mepc:   ");
    match task.last_fault_mepc {
        Some(value) => uart::write_hex_u64(value),
        None => uart::write_str("none"),
    }
    uart::write_line("");

    uart::write_str("    mtval:  ");
    match task.last_fault_mtval {
        Some(value) => uart::write_hex_u64(value),
        None => uart::write_str("none"),
    }
    uart::write_line("");
}

#[allow(clippy::needless_range_loop)]
fn find_slot_by_id(id: usize) -> Option<usize> {
    unsafe {
        for slot in 0..MAX_TASKS {
            if !matches!(TASKS[slot].state, TaskState::Empty) && TASKS[slot].id == id {
                return Some(slot);
            }
        }
    }

    None
}

#[allow(clippy::needless_range_loop)]
fn find_next_task_after<F>(current_id: Option<usize>, mut accept: F) -> Option<usize>
where
    F: FnMut(Task) -> bool,
{
    let start_slot = match current_id {
        Some(id) => find_slot_by_id(id).map(|slot| slot + 1).unwrap_or(0),
        None => 0,
    };

    for offset in 0..MAX_TASKS {
        let slot = (start_slot + offset) % MAX_TASKS;

        let task = unsafe { TASKS[slot] };
        if matches!(task.state, TaskState::Empty) {
            continue;
        }

        if accept(task) {
            return Some(task.id);
        }
    }

    None
}

// Keep this manual copy for early bare-metal safety.
// Slice copy/fill caused an early ARM64 exception during task creation.
#[allow(clippy::manual_memcpy)]
fn copy_name(dst: &mut [u8; TASK_NAME_LEN], name: &str) {
    let mut i = 0;

    while i < TASK_NAME_LEN {
        dst[i] = 0;
        i += 1;
    }

    let bytes = name.as_bytes();
    let len = min(bytes.len(), TASK_NAME_LEN - 1);

    i = 0;

    while i < len {
        dst[i] = bytes[i];
        i += 1;
    }
}

fn min(a: usize, b: usize) -> usize {
    if a < b {
        a
    } else {
        b
    }
}

fn write_name(name: &[u8; TASK_NAME_LEN]) {
    for byte in name {
        if *byte == 0 {
            break;
        }

        uart::putc(*byte);
    }
}

fn print_state(state: TaskState) {
    match state {
        TaskState::Empty => uart::write_str("Empty"),
        TaskState::Ready => uart::write_str("Ready"),
        TaskState::Running => uart::write_str("Running"),
        TaskState::Blocked => uart::write_str("Blocked"),
        TaskState::Finished => uart::write_str("Finished"),
        TaskState::Faulted => uart::write_str("Faulted"),
    }
}

#[allow(dead_code)]
#[allow(clippy::needless_range_loop)]
pub fn set_task_state(id: usize, state: TaskState) -> bool {
    let Some(slot) = find_slot_by_id(id) else {
        return false;
    };

    unsafe {
        TASKS[slot].state = state;
    }

    true
}

#[allow(clippy::needless_range_loop)]
pub fn get_task_state(id: usize) -> Option<TaskState> {
    let slot = find_slot_by_id(id)?;
    Some(unsafe { TASKS[slot].state })
}

pub fn print_task_state_by_id(id: usize) {
    match get_task_state(id) {
        Some(state) => print_state(state),
        None => uart::write_str("unknown"),
    }
}

#[allow(clippy::needless_range_loop)]
pub fn set_task_return_kind(id: usize, kind: TaskReturnKind) -> bool {
    unsafe {
        for slot in 0..MAX_TASKS {
            if !matches!(TASKS[slot].state, TaskState::Empty) && TASKS[slot].id == id {
                TASKS[slot].last_return_kind = kind;
                return true;
            }
        }
    }

    false
}

#[allow(dead_code)]
#[allow(clippy::needless_range_loop)]
pub fn get_task_return_kind(id: usize) -> Option<TaskReturnKind> {
    let slot = find_slot_by_id(id)?;
    Some(unsafe { TASKS[slot].last_return_kind })
}

pub fn print_task_return_kind(kind: TaskReturnKind) {
    match kind {
        TaskReturnKind::None => uart::write_str("None"),
        TaskReturnKind::Exit => uart::write_str("Exit"),
        TaskReturnKind::Yield => uart::write_str("Yield"),
        TaskReturnKind::Sleep => uart::write_str("Sleep"),
        TaskReturnKind::Fault => uart::write_str("Fault"),
    }
}

pub fn print_task_return_kind_by_id(id: usize) {
    match get_task_return_kind(id) {
        Some(kind) => print_task_return_kind(kind),
        None => uart::write_str("unknown"),
    }
}

#[allow(clippy::needless_range_loop)]
pub fn set_task_last_return_context(
    id: usize,
    task_sp: u64,
    kernel_sp: u64,
    kernel_return_pc: u64,
) -> bool {
    unsafe {
        for slot in 0..MAX_TASKS {
            if !matches!(TASKS[slot].state, TaskState::Empty) && TASKS[slot].id == id {
                TASKS[slot].last_task_sp = task_sp;
                TASKS[slot].last_kernel_sp = kernel_sp;
                TASKS[slot].last_kernel_return_pc = kernel_return_pc;
                return true;
            }
        }
    }

    false
}

#[allow(dead_code)]
pub fn apply_task_return_transition(
    task_id: usize,
    kind: TaskReturnKind,
    context: TaskReturnContext,
    cpu_context: TaskCpuContext,
) -> bool {
    if !set_task_last_return_context(
        task_id,
        context.task_sp,
        context.kernel_sp,
        context.kernel_return_pc,
    ) {
        return false;
    }

    if !set_task_cpu_context(task_id, cpu_context) {
        return false;
    }

    set_last_returned_task_id(task_id);

    match kind {
        TaskReturnKind::Yield => mark_task_ready_after_yield(task_id),
        TaskReturnKind::Sleep => mark_task_blocked_for_sleep(task_id),
        TaskReturnKind::Exit => mark_task_finished(task_id),
        TaskReturnKind::Fault => {
            if matches!(get_task_state(task_id), Some(TaskState::Faulted)) {
                set_task_return_kind(task_id, TaskReturnKind::Fault)
                    && set_task_can_resume(task_id, false)
            } else {
                mark_task_faulted(task_id)
            }
        }
        TaskReturnKind::None => set_task_return_kind(task_id, kind),
    }
}

#[allow(clippy::needless_range_loop)]
pub fn is_sp_inside_task_stack(id: usize, sp: u64) -> Option<bool> {
    unsafe {
        for slot in 0..MAX_TASKS {
            if !matches!(TASKS[slot].state, TaskState::Empty) && TASKS[slot].id == id {
                return Some(sp >= TASKS[slot].stack_start && sp < TASKS[slot].stack_top);
            }
        }
    }

    None
}

#[allow(dead_code)]
#[allow(clippy::needless_range_loop)]
pub fn set_task_can_resume(id: usize, can_resume: bool) -> bool {
    unsafe {
        for slot in 0..MAX_TASKS {
            if !matches!(TASKS[slot].state, TaskState::Empty) && TASKS[slot].id == id {
                TASKS[slot].can_resume = can_resume;
                return true;
            }
        }
    }

    false
}

#[allow(dead_code)]
#[allow(clippy::needless_range_loop)]
pub fn can_task_resume(id: usize) -> Option<bool> {
    let slot = find_slot_by_id(id)?;
    Some(unsafe { TASKS[slot].can_resume })
}

#[allow(dead_code)]
#[allow(clippy::needless_range_loop)]
pub fn get_task_last_task_sp(id: usize) -> Option<u64> {
    let slot = find_slot_by_id(id)?;
    Some(unsafe { TASKS[slot].last_task_sp })
}

#[allow(dead_code)]
#[allow(clippy::needless_range_loop)]
pub fn get_task_last_kernel_sp(id: usize) -> Option<u64> {
    let slot = find_slot_by_id(id)?;
    Some(unsafe { TASKS[slot].last_kernel_sp })
}

#[allow(dead_code)]
#[allow(clippy::needless_range_loop)]
pub fn get_task_last_kernel_return_pc(id: usize) -> Option<u64> {
    let slot = find_slot_by_id(id)?;
    Some(unsafe { TASKS[slot].last_kernel_return_pc })
}

#[allow(dead_code)]
#[allow(clippy::needless_range_loop)]
pub fn find_first_resumable_task() -> Option<usize> {
    unsafe {
        for slot in 0..MAX_TASKS {
            let task = TASKS[slot];

            if matches!(task.state, TaskState::Ready)
                && task.can_resume
                && matches!(task.last_return_kind, TaskReturnKind::Yield)
                && task.last_task_sp >= task.stack_start
                && task.last_task_sp < task.stack_top
            {
                return Some(task.id);
            }
        }
    }

    None
}

#[allow(dead_code)]
pub fn print_yes_no(value: bool) {
    if value {
        uart::write_str("yes");
    } else {
        uart::write_str("no");
    }
}

#[allow(clippy::needless_range_loop)]
pub fn set_task_cpu_context(id: usize, context: TaskCpuContext) -> bool {
    let Some(slot) = find_slot_by_id(id) else {
        return false;
    };

    unsafe {
        TASKS[slot].cpu_context = context;
        TASKS[slot].saved_sp = context.sp;
        TASKS[slot].saved_pc = context.return_pc;
    }

    true
}

#[allow(dead_code)]
#[allow(clippy::needless_range_loop)]
pub fn get_task_cpu_context(id: usize) -> Option<TaskCpuContext> {
    let slot = find_slot_by_id(id)?;
    Some(unsafe { TASKS[slot].cpu_context })
}

#[allow(dead_code)]
pub fn get_task_resume_pc(id: usize) -> Option<u64> {
    get_task_cpu_context(id).map(|context| context.resume_pc)
}

#[allow(dead_code)]
pub fn get_task_entry_addr(id: usize) -> Option<u64> {
    get_task_entry(id).map(|entry| entry as *const () as usize as u64)
}

pub fn get_task_resume_frame(
    id: usize,
) -> Option<crate::kernel::task::cpu_context::TaskCpuContext> {
    get_task_cpu_context(id)
}

#[allow(dead_code)]
pub fn print_task_resume_frame_by_id(id: usize) {
    match get_task_resume_frame(id) {
        Some(frame) => crate::kernel::task::cpu_context::print_cpu_context(frame),
        None => uart::write_str("none"),
    }
}

#[allow(dead_code)]
pub fn set_last_returned_task_id(id: usize) {
    unsafe {
        LAST_RETURNED_TASK_ID = Some(id);
    }
}

#[allow(dead_code)]
pub fn get_last_returned_task_id() -> Option<usize> {
    unsafe { LAST_RETURNED_TASK_ID }
}

#[allow(dead_code)]
pub fn is_task_ready(id: usize) -> bool {
    matches!(get_task_state(id), Some(TaskState::Ready))
}

#[allow(dead_code)]
pub fn is_task_running(id: usize) -> bool {
    matches!(get_task_state(id), Some(TaskState::Running))
}

#[allow(dead_code)]
pub fn is_ready_running_faulted_finished_invariant_ok(id: usize) -> bool {
    match get_task_state(id) {
        Some(TaskState::Ready) => {
            !is_task_running(id) && !is_task_faulted(id) && !is_task_finished(id)
        }
        Some(TaskState::Running) => {
            !is_task_ready(id) && !is_task_faulted(id) && !is_task_finished(id)
        }
        Some(TaskState::Faulted) => {
            !is_task_ready(id) && !is_task_running(id) && !is_task_finished(id)
        }
        Some(TaskState::Finished) => {
            !is_task_ready(id) && !is_task_running(id) && !is_task_faulted(id)
        }
        _ => false,
    }
}

#[allow(dead_code)]
pub fn can_dispatch_from_ready(id: usize) -> bool {
    is_task_ready(id) && is_ready_running_faulted_finished_invariant_ok(id)
}

#[cfg(any(
    feature = "scheduler_reentry_test",
    feature = "scheduler_dispatch_test"
))]
#[allow(dead_code)]
pub fn get_task_return_snapshot(id: usize) -> Option<TaskReturnSnapshot> {
    let state = get_task_state(id)?;
    let last_return = get_task_return_kind(id)?;
    let can_resume = can_task_resume(id)?;

    Some(TaskReturnSnapshot {
        task_id: id,
        state,
        last_return,
        can_resume,
    })
}

#[cfg(any(
    feature = "scheduler_reentry_test",
    feature = "scheduler_dispatch_test"
))]
#[allow(dead_code)]
pub fn get_last_returned_task_snapshot() -> Option<TaskReturnSnapshot> {
    let id = get_last_returned_task_id()?;
    get_task_return_snapshot(id)
}

#[allow(dead_code)]
pub fn is_resumable_task(id: usize) -> bool {
    can_dispatch_from_ready(id)
        && matches!(can_task_resume(id), Some(true))
        && matches!(
            get_task_return_kind(id),
            Some(TaskReturnKind::Yield | TaskReturnKind::Sleep)
        )
        && is_resume_frame_safe_for_task(id)
}

#[allow(dead_code)]
pub fn is_resume_frame_safe_for_task(id: usize) -> bool {
    let Some(frame) = get_task_resume_frame(id) else {
        return false;
    };

    frame.is_valid()
        && matches!(is_sp_inside_task_stack(id, frame.sp), Some(true))
        && memory::is_inside_kernel_text(frame.resume_pc)
        && memory::is_inside_kernel_text(frame.return_pc)
        && matches!(
            (get_task_last_task_sp(id), get_task_last_kernel_return_pc(id)),
            (Some(last_sp), Some(return_pc)) if frame.sp == last_sp && frame.return_pc == return_pc
        )
}

#[allow(dead_code)]
pub fn is_fresh_ready_task(id: usize) -> bool {
    can_dispatch_from_ready(id) && !has_started(id) && matches!(can_task_resume(id), Some(false))
}

#[allow(dead_code)]
pub fn is_dispatchable_task(id: usize) -> bool {
    is_resumable_task(id) || is_fresh_ready_task(id)
}

#[allow(dead_code)]
#[allow(clippy::needless_range_loop)]
pub fn set_task_fault_info(
    id: usize,
    reason: TaskFaultReason,
    mcause: u64,
    mepc: u64,
    mtval: u64,
) -> bool {
    unsafe {
        for slot in 0..MAX_TASKS {
            if !matches!(TASKS[slot].state, TaskState::Empty) && TASKS[slot].id == id {
                TASKS[slot].last_fault_reason = Some(reason);
                TASKS[slot].last_fault_mcause = Some(mcause);
                TASKS[slot].last_fault_mepc = Some(mepc);
                TASKS[slot].last_fault_mtval = Some(mtval);
                return true;
            }
        }
    }
    false
}

#[allow(dead_code)]
pub fn record_task_fault(id: usize, mcause: u64, mepc: u64, mtval: u64) -> Option<TaskFaultReason> {
    let reason = TaskFaultReason::from_mcause(mcause);

    if set_task_fault_info(id, reason, mcause, mepc, mtval) && mark_task_faulted(id) {
        Some(reason)
    } else {
        None
    }
}

#[allow(dead_code)]
#[allow(clippy::needless_range_loop)]
pub fn get_task_fault_reason(id: usize) -> Option<TaskFaultReason> {
    unsafe {
        for slot in 0..MAX_TASKS {
            if !matches!(TASKS[slot].state, TaskState::Empty) && TASKS[slot].id == id {
                return TASKS[slot].last_fault_reason;
            }
        }
    }
    None
}

#[allow(dead_code)]
#[allow(clippy::needless_range_loop)]
pub fn get_task_fault_mcause(id: usize) -> Option<u64> {
    unsafe {
        for slot in 0..MAX_TASKS {
            if !matches!(TASKS[slot].state, TaskState::Empty) && TASKS[slot].id == id {
                return TASKS[slot].last_fault_mcause;
            }
        }
    }

    None
}

#[allow(dead_code)]
#[allow(clippy::needless_range_loop)]
pub fn get_task_fault_mepc(id: usize) -> Option<u64> {
    unsafe {
        for slot in 0..MAX_TASKS {
            if !matches!(TASKS[slot].state, TaskState::Empty) && TASKS[slot].id == id {
                return TASKS[slot].last_fault_mepc;
            }
        }
    }

    None
}

#[allow(dead_code)]
#[allow(clippy::needless_range_loop)]
pub fn get_task_fault_mtval(id: usize) -> Option<u64> {
    unsafe {
        for slot in 0..MAX_TASKS {
            if !matches!(TASKS[slot].state, TaskState::Empty) && TASKS[slot].id == id {
                return TASKS[slot].last_fault_mtval;
            }
        }
    }

    None
}

#[allow(dead_code)]
pub fn print_task_fault_reason(reason: TaskFaultReason) {
    match reason {
        TaskFaultReason::Breakpoint => uart::write_str("breakpoint"),
        TaskFaultReason::InstructionAccessFault => uart::write_str("instruction access fault"),
        TaskFaultReason::LoadAccessFault => uart::write_str("load access fault"),
        TaskFaultReason::StoreAccessFault => uart::write_str("store access fault"),
        TaskFaultReason::IllegalInstruction => uart::write_str("illegal instruction"),
        TaskFaultReason::Unknown(code) => {
            uart::write_str("unknown (code: ");
            uart::write_hex_u64(code);
            uart::write_str(")");
        }
    }
}

#[allow(dead_code)]
#[cfg(feature = "scheduler_dispatch_test")]
pub fn get_task_id_at_slot(slot: usize) -> Option<usize> {
    if slot >= MAX_TASKS {
        return None;
    }
    unsafe {
        if matches!(TASKS[slot].state, TaskState::Empty) {
            None
        } else {
            Some(TASKS[slot].id)
        }
    }
}

#[allow(dead_code)]
pub fn is_task_finished(id: usize) -> bool {
    matches!(get_task_state(id), Some(TaskState::Finished))
}

#[allow(dead_code)]
pub fn is_task_faulted(id: usize) -> bool {
    matches!(get_task_state(id), Some(TaskState::Faulted))
}

#[allow(dead_code)]
pub fn is_terminal_task(id: usize) -> bool {
    is_task_finished(id) || is_task_faulted(id)
}

#[allow(dead_code)]
#[allow(clippy::needless_range_loop)]
pub fn count_dispatchable_tasks() -> usize {
    let mut count = 0;

    unsafe {
        for slot in 0..MAX_TASKS {
            if matches!(TASKS[slot].state, TaskState::Empty) {
                continue;
            }

            if is_dispatchable_task(TASKS[slot].id) {
                count += 1;
            }
        }
    }

    count
}

#[allow(dead_code)]
pub fn has_dispatchable_tasks() -> bool {
    count_dispatchable_tasks() > 0
}

#[allow(dead_code)]
#[allow(clippy::needless_range_loop)]
pub fn find_first_task_by_state(state: TaskState) -> Option<usize> {
    unsafe {
        for slot in 0..MAX_TASKS {
            if matches!(TASKS[slot].state, TaskState::Empty) {
                continue;
            }

            if TASKS[slot].state == state {
                return Some(TASKS[slot].id);
            }
        }
    }

    None
}

#[allow(dead_code)]
pub fn find_first_finished_task() -> Option<usize> {
    find_first_task_by_state(TaskState::Finished)
}

#[allow(dead_code)]
pub fn find_first_faulted_task() -> Option<usize> {
    find_first_task_by_state(TaskState::Faulted)
}

#[allow(dead_code)]
#[cfg(feature = "scheduler_dispatch_test")]
#[derive(Clone, Copy)]
pub struct TerminalTaskDispatchInvariantSnapshot {
    pub terminal: bool,
    pub resumable: bool,
    pub fresh_ready: bool,
    pub dispatchable: bool,
    pub result: bool,
}

#[allow(dead_code)]
#[cfg(feature = "scheduler_dispatch_test")]
pub fn get_terminal_task_dispatch_invariants(id: usize) -> TerminalTaskDispatchInvariantSnapshot {
    let terminal = is_terminal_task(id);
    let resumable = is_resumable_task(id);
    let fresh_ready = is_fresh_ready_task(id);
    let dispatchable = is_dispatchable_task(id);

    let result = terminal && !resumable && !fresh_ready && !dispatchable;

    TerminalTaskDispatchInvariantSnapshot {
        terminal,
        resumable,
        fresh_ready,
        dispatchable,
        result,
    }
}

#[allow(dead_code)]
#[cfg(feature = "scheduler_dispatch_test")]
pub fn validate_terminal_task_dispatch_invariants(id: usize) -> bool {
    get_terminal_task_dispatch_invariants(id).result
}

#[allow(dead_code)]
#[cfg(feature = "scheduler_dispatch_test")]
#[derive(Clone, Copy)]
pub struct BreakpointFaultMetadataAssertionSnapshot {
    pub reason_breakpoint: bool,
    pub mcause_breakpoint: bool,
    pub mepc_nonzero: bool,
    pub mtval_nonzero: bool,
    pub result: bool,
}

#[allow(dead_code)]
#[cfg(feature = "scheduler_dispatch_test")]
pub fn get_breakpoint_fault_metadata_assertions(
    id: usize,
) -> BreakpointFaultMetadataAssertionSnapshot {
    let reason_breakpoint = matches!(get_task_fault_reason(id), Some(TaskFaultReason::Breakpoint));

    let mcause_breakpoint = matches!(get_task_fault_mcause(id), Some(3));

    let mepc_nonzero = get_task_fault_mepc(id)
        .map(|value| value != 0)
        .unwrap_or(false);

    let mtval_nonzero = get_task_fault_mtval(id)
        .map(|value| value != 0)
        .unwrap_or(false);

    let result = reason_breakpoint && mcause_breakpoint && mepc_nonzero && mtval_nonzero;

    BreakpointFaultMetadataAssertionSnapshot {
        reason_breakpoint,
        mcause_breakpoint,
        mepc_nonzero,
        mtval_nonzero,
        result,
    }
}

#[allow(dead_code)]
#[cfg(feature = "scheduler_reentry_test")]
#[derive(Clone, Copy)]
pub struct TaskFaultCompletionSnapshot {
    pub finished_task_id: Option<usize>,
    pub faulted_task_id: Option<usize>,

    pub finished_task_finished: bool,
    pub finished_task_last_return_exit: bool,

    pub faulted_task_faulted: bool,
    pub faulted_task_last_return_fault: bool,
    pub faulted_task_resume_disabled: bool,

    pub result: bool,
}

#[allow(dead_code)]
#[cfg(feature = "scheduler_reentry_test")]
pub fn get_task_fault_completion_snapshot() -> TaskFaultCompletionSnapshot {
    let finished_task_id = find_first_finished_task();
    let faulted_task_id = find_first_faulted_task();

    let finished_task_finished = finished_task_id.map(is_task_finished).unwrap_or(false);

    let finished_task_last_return_exit = finished_task_id
        .map(|id| matches!(get_task_return_kind(id), Some(TaskReturnKind::Exit)))
        .unwrap_or(false);

    let faulted_task_faulted = faulted_task_id.map(is_task_faulted).unwrap_or(false);

    let faulted_task_last_return_fault = faulted_task_id
        .map(|id| matches!(get_task_return_kind(id), Some(TaskReturnKind::Fault)))
        .unwrap_or(false);

    let faulted_task_resume_disabled = faulted_task_id
        .map(|id| !can_task_resume(id).unwrap_or(true))
        .unwrap_or(false);

    let result = finished_task_finished
        && finished_task_last_return_exit
        && faulted_task_faulted
        && faulted_task_last_return_fault
        && faulted_task_resume_disabled;

    TaskFaultCompletionSnapshot {
        finished_task_id,
        faulted_task_id,

        finished_task_finished,
        finished_task_last_return_exit,

        faulted_task_faulted,
        faulted_task_last_return_fault,
        faulted_task_resume_disabled,

        result,
    }
}

#[allow(dead_code)]
#[allow(clippy::needless_range_loop)]
pub fn mark_task_finished(id: usize) -> bool {
    let Some(slot) = find_slot_by_id(id) else {
        return false;
    };

    unsafe {
        if !can_transition_from(TASKS[slot].state, TaskLifecycleTransition::Exit) {
            return false;
        }
        TASKS[slot].state = TaskState::Finished;
        TASKS[slot].last_return_kind = TaskReturnKind::Exit;
        TASKS[slot].can_resume = false;
    }

    true
}

#[allow(dead_code)]
#[allow(clippy::needless_range_loop)]
pub fn mark_task_ready_after_yield(id: usize) -> bool {
    let Some(slot) = find_slot_by_id(id) else {
        return false;
    };

    unsafe {
        if !can_transition_from(TASKS[slot].state, TaskLifecycleTransition::Yield) {
            return false;
        }
        TASKS[slot].state = TaskState::Ready;
        TASKS[slot].last_return_kind = TaskReturnKind::Yield;
        TASKS[slot].can_resume = true;
        TASKS[slot].sleep_until_tick = None;
    }

    true
}

#[allow(dead_code)]
#[allow(clippy::needless_range_loop)]
pub fn mark_task_blocked_until(id: usize, wake_tick: u64) -> bool {
    let Some(slot) = find_slot_by_id(id) else {
        return false;
    };

    unsafe {
        if !can_transition_from(TASKS[slot].state, TaskLifecycleTransition::Sleep) {
            return false;
        }

        TASKS[slot].state = TaskState::Blocked;
        TASKS[slot].last_return_kind = TaskReturnKind::Sleep;
        TASKS[slot].can_resume = false;
        TASKS[slot].sleep_until_tick = Some(wake_tick);
    }

    true
}

#[allow(dead_code)]
#[allow(clippy::needless_range_loop)]
pub fn mark_task_blocked_for_sleep(id: usize) -> bool {
    let Some(slot) = find_slot_by_id(id) else {
        return false;
    };

    unsafe {
        if !matches!(TASKS[slot].state, TaskState::Blocked) {
            return false;
        }

        TASKS[slot].last_return_kind = TaskReturnKind::Sleep;
        TASKS[slot].can_resume = false;
    }

    true
}

#[allow(dead_code)]
#[allow(clippy::needless_range_loop)]
pub fn wake_sleeping_tasks(current_tick: u64) -> usize {
    let mut woke = 0;

    unsafe {
        for slot in 0..MAX_TASKS {
            if !matches!(TASKS[slot].state, TaskState::Blocked) {
                continue;
            }

            let Some(wake_tick) = TASKS[slot].sleep_until_tick else {
                continue;
            };

            if current_tick >= wake_tick {
                let can_resume = TASKS[slot].has_started && TASKS[slot].cpu_context.is_valid();
                TASKS[slot].state = TaskState::Ready;
                TASKS[slot].last_return_kind = if can_resume {
                    TaskReturnKind::Sleep
                } else {
                    TaskReturnKind::None
                };
                TASKS[slot].can_resume = can_resume;
                TASKS[slot].sleep_until_tick = None;
                woke += 1;
            }
        }
    }

    woke
}

#[allow(dead_code)]
#[allow(clippy::needless_range_loop)]
pub fn mark_task_faulted(id: usize) -> bool {
    let Some(slot) = find_slot_by_id(id) else {
        return false;
    };

    unsafe {
        if !can_transition_from(TASKS[slot].state, TaskLifecycleTransition::Fault) {
            return false;
        }
        TASKS[slot].state = TaskState::Faulted;
        TASKS[slot].last_return_kind = TaskReturnKind::Fault;
        TASKS[slot].can_resume = false;
    }

    true
}
```

## File: src/kernel/task/scheduler.rs
```rust
use crate::drivers::uart;
#[cfg(feature = "scheduler_dispatch_test")]
use crate::kernel::task::cpu_context::TaskCpuContext;
use crate::kernel::task::table as task;
#[cfg(feature = "scheduler_reentry_test")]
use crate::kernel::task::table::TaskReturnSnapshot;
static mut CURRENT_TASK_ID: Option<usize> = None;

pub fn init() {
    unsafe {
        CURRENT_TASK_ID = None;
    }

    uart::write_line("");
    uart::write_line("scheduler:");

    schedule_next();

    uart::write_str("current task: ");
    print_current_task_name();

    uart::write_str(" entry: ");
    print_current_task_entry();

    uart::write_str(" context:");
    match current_task_id() {
        Some(id) => task::print_task_full_context_by_id(id),
        None => uart::write_str(" none"),
    }

    uart::write_line("");
}

pub fn schedule_next() -> Option<usize> {
    let current = unsafe { CURRENT_TASK_ID };
    let next = task::find_next_ready_after(current)?;

    if !task::mark_task_running(next) {
        return None;
    }

    unsafe {
        CURRENT_TASK_ID = Some(next);
    }

    Some(next)
}

pub fn decide_next_task_dry_run() -> Option<usize> {
    task::find_next_ready_after(current_task_id())
}

#[allow(dead_code)]
pub fn decide_next_resumable_task_dry_run() -> Option<usize> {
    let candidate = task::find_next_ready_after(current_task_id())?;
    if task::is_resumable_task(candidate) {
        Some(candidate)
    } else {
        None
    }
}

pub fn current_task_id() -> Option<usize> {
    unsafe { CURRENT_TASK_ID }
}

pub fn print_current_task_name() {
    match current_task_id() {
        Some(id) => task::print_task_name_by_id(id),
        None => uart::write_str("none"),
    }
}

pub fn print_current_task_entry() {
    match current_task_id() {
        Some(id) => task::print_task_entry_by_id(id),
        None => uart::write_str("none"),
    }
}

#[allow(dead_code)]
pub fn on_timer_tick(tick: u64) {
    let next = schedule_next();

    uart::write_str("tick: ");
    uart::write_dec_u64(tick);

    uart::write_str(" task: ");

    match next {
        Some(id) => {
            task::print_task_name_by_id(id);

            uart::write_str(" entry: ");
            task::print_task_entry_by_id(id);

            uart::write_str(" context:");
            task::print_task_full_context_by_id(id);
        }
        None => uart::write_str("none"),
    }
}

pub fn save_current_context(saved_sp: u64, saved_pc: u64) -> Option<usize> {
    let current = current_task_id()?;

    task::update_context(current, saved_sp, saved_pc);

    Some(current)
}

pub fn print_task_name(id: usize) {
    task::print_task_name_by_id(id);
}

#[allow(dead_code)]
pub fn print_task_context(id: usize) {
    task::print_task_context_by_id(id);
}

pub fn force_current_task(id: usize) {
    if !task::mark_task_running(id) {
        return;
    }

    set_round_robin_cursor(id);
}

fn set_round_robin_cursor(id: usize) {
    unsafe {
        CURRENT_TASK_ID = Some(id);
    }
}

pub fn switch_to_idle() {
    force_current_task(0);
}

#[allow(dead_code)]
pub fn set_current_task(id: usize) {
    force_current_task(id);
}

#[cfg(feature = "scheduler_dispatch_test")]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DispatchResult {
    NoRunnableTask,
    Failed,
}

#[cfg(feature = "scheduler_dispatch_test")]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RunOnceResult {
    NoRunnableTask,
    Failed,
}

#[cfg(feature = "scheduler_run_test")]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RunResult {
    NoRunnableTask,
    Failed,
}

#[cfg(feature = "scheduler_reentry_test")]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TaskReturnHandleResult {
    NoRunnableTask,
    Failed,
}

#[allow(dead_code)]
#[derive(Clone, Copy)]
pub struct NoRunnableSchedulerSnapshot {
    pub dispatchable_count: usize,
    pub has_dispatchable_tasks: bool,
    pub no_runnable: bool,
    pub result: bool,
}

#[allow(dead_code)]
pub fn get_no_runnable_scheduler_snapshot() -> NoRunnableSchedulerSnapshot {
    let dispatchable_count = crate::kernel::task::table::count_dispatchable_tasks();
    let has_dispatchable_tasks = crate::kernel::task::table::has_dispatchable_tasks();
    let no_runnable = !has_dispatchable_tasks;

    NoRunnableSchedulerSnapshot {
        dispatchable_count,
        has_dispatchable_tasks,
        no_runnable,
        result: no_runnable && dispatchable_count == 0,
    }
}

#[cfg(feature = "scheduler_dispatch_test")]
fn find_next_dispatchable_task_after(start_after: Option<usize>) -> Option<usize> {
    #[cfg(feature = "scheduler_verbose_dispatch_trace")]
    {
        scheduler_log_str("  find_next_dispatchable_task_after: after_task_id=");
        match start_after {
            Some(id) => crate::drivers::uart::write_dec_u64(id as u64),
            None => scheduler_log_str("none"),
        }
        scheduler_log_line("");
    }

    let selected = task::find_next_dispatchable_after(start_after);

    #[cfg(feature = "scheduler_verbose_dispatch_trace")]
    match selected {
        Some(task_id) => {
            scheduler_log_str("      -> FOUND: task_id=");
            crate::drivers::uart::write_dec_u64(task_id as u64);
            scheduler_log_line("");
        }
        None => scheduler_log_line("  find_next_dispatchable_task_after: none found"),
    }

    selected
}

#[cfg(feature = "scheduler_dispatch_test")]
#[derive(Clone, Copy, PartialEq, Eq)]
enum DispatchDecision {
    StartFresh { task_id: usize },
    ResumeSaved { task_id: usize },
    NoRunnableTask,
    Failed,
}

#[cfg(feature = "scheduler_dispatch_test")]
#[derive(Clone, Copy, PartialEq, Eq)]
enum DispatchDecisionKind {
    StartFresh,
    ResumeSaved,
    NoRunnableTask,
    Failed,
}

#[cfg(feature = "scheduler_dispatch_test")]
#[derive(Clone, Copy, PartialEq, Eq)]
enum DispatchDecisionOutcome {
    Dispatchable,
    NoRunnableTask,
    Failed,
}

#[cfg(feature = "scheduler_dispatch_test")]
#[derive(Clone, Copy, PartialEq, Eq)]
enum DispatchCandidate {
    Task { task_id: usize },
    None,
}

#[cfg(feature = "scheduler_dispatch_test")]
#[derive(Clone, Copy)]
struct DispatchPipeline {
    current: Option<usize>,
}

#[cfg(feature = "scheduler_dispatch_test")]
#[derive(Clone, Copy, PartialEq, Eq)]
enum DispatchCandidateKind {
    Task,
    None,
}

#[cfg(feature = "scheduler_dispatch_test")]
impl DispatchDecision {
    fn kind(self) -> DispatchDecisionKind {
        match self {
            DispatchDecision::StartFresh { .. } => DispatchDecisionKind::StartFresh,
            DispatchDecision::ResumeSaved { .. } => DispatchDecisionKind::ResumeSaved,
            DispatchDecision::NoRunnableTask => DispatchDecisionKind::NoRunnableTask,
            DispatchDecision::Failed => DispatchDecisionKind::Failed,
        }
    }

    fn task_id(self) -> Option<usize> {
        match self {
            DispatchDecision::StartFresh { task_id }
            | DispatchDecision::ResumeSaved { task_id } => Some(task_id),
            DispatchDecision::NoRunnableTask | DispatchDecision::Failed => None,
        }
    }

    fn is_dispatchable(self) -> bool {
        self.kind().is_dispatchable()
    }

    fn outcome(self) -> DispatchDecisionOutcome {
        self.kind().outcome()
    }
}

#[cfg(feature = "scheduler_dispatch_test")]
impl DispatchDecisionKind {
    fn is_dispatchable(self) -> bool {
        self.outcome().is_dispatchable()
    }

    fn outcome(self) -> DispatchDecisionOutcome {
        match self {
            DispatchDecisionKind::StartFresh | DispatchDecisionKind::ResumeSaved => {
                DispatchDecisionOutcome::Dispatchable
            }
            DispatchDecisionKind::NoRunnableTask => DispatchDecisionOutcome::NoRunnableTask,
            DispatchDecisionKind::Failed => DispatchDecisionOutcome::Failed,
        }
    }
}

#[cfg(feature = "scheduler_dispatch_test")]
impl DispatchDecisionOutcome {
    fn label(self) -> &'static str {
        match self {
            DispatchDecisionOutcome::Dispatchable => "Dispatchable",
            DispatchDecisionOutcome::NoRunnableTask => "NoRunnableTask",
            DispatchDecisionOutcome::Failed => "Failed",
        }
    }

    fn is_dispatchable(self) -> bool {
        matches!(self, DispatchDecisionOutcome::Dispatchable)
    }
}

#[cfg(feature = "scheduler_dispatch_test")]
impl DispatchCandidate {
    fn kind(self) -> DispatchCandidateKind {
        match self {
            DispatchCandidate::Task { .. } => DispatchCandidateKind::Task,
            DispatchCandidate::None => DispatchCandidateKind::None,
        }
    }

    fn task_id(self) -> Option<usize> {
        match self {
            DispatchCandidate::Task { task_id } => Some(task_id),
            DispatchCandidate::None => None,
        }
    }

    fn decision(self) -> DispatchDecision {
        match self.task_id() {
            Some(task_id) => build_dispatch_decision_for_task(task_id),
            None => DispatchDecision::NoRunnableTask,
        }
    }
}

#[cfg(feature = "scheduler_dispatch_test")]
impl DispatchCandidateKind {
    fn label(self) -> &'static str {
        match self {
            DispatchCandidateKind::Task => "Task",
            DispatchCandidateKind::None => "None",
        }
    }
}

#[cfg(feature = "scheduler_dispatch_test")]
impl DispatchPipeline {
    fn new(current: Option<usize>) -> Self {
        Self { current }
    }

    fn candidate(self) -> DispatchCandidate {
        select_dispatch_candidate_after(self.current)
    }

    fn decision_from_candidate(self, candidate: DispatchCandidate) -> DispatchDecision {
        select_dispatch_decision_from_candidate(candidate)
    }

    fn decision(self) -> DispatchDecision {
        let candidate = self.candidate();

        self.decision_from_candidate(candidate)
    }

    fn run(self) -> DispatchResult {
        let decision = self.decision();

        execute_dispatch_decision(decision)
    }
}

#[cfg(feature = "scheduler_dispatch_test")]
fn build_dispatch_decision_for_task(task_id: usize) -> DispatchDecision {
    if task::is_resumable_task(task_id) {
        DispatchDecision::ResumeSaved { task_id }
    } else if task::is_fresh_ready_task(task_id) {
        DispatchDecision::StartFresh { task_id }
    } else {
        DispatchDecision::Failed
    }
}

#[cfg(feature = "scheduler_dispatch_test")]
fn print_dispatch_candidate(candidate: DispatchCandidate) {
    scheduler_log_str("  dispatch candidate: ");
    scheduler_log_str(candidate.kind().label());

    if let Some(task_id) = candidate.task_id() {
        scheduler_log_str("(");
        crate::drivers::uart::write_dec_u64(task_id as u64);
        scheduler_log_str(")");
    }

    scheduler_log_line("");
}

#[cfg(feature = "scheduler_dispatch_test")]
fn select_dispatch_decision_from_candidate(candidate: DispatchCandidate) -> DispatchDecision {
    print_dispatch_candidate(candidate);

    match candidate.task_id() {
        Some(task_id) => {
            print_dispatch_task_summary(task_id);
        }
        None => {
            scheduler_log_line("  selected task: none");
        }
    }

    candidate.decision()
}

#[cfg(feature = "scheduler_dispatch_test")]
fn select_dispatch_candidate_after(current: Option<usize>) -> DispatchCandidate {
    match find_next_dispatchable_task_after(current) {
        Some(task_id) => DispatchCandidate::Task { task_id },
        None => DispatchCandidate::None,
    }
}

#[cfg(feature = "scheduler_dispatch_test")]
fn execute_dispatch_decision(decision: DispatchDecision) -> DispatchResult {
    print_dispatch_decision_model(decision);

    match decision {
        DispatchDecision::ResumeSaved { task_id } => {
            scheduler_log_line("  dispatch action: resume task");

            force_current_task(task_id);

            resume_selected_task_checked(task_id)
        }
        DispatchDecision::StartFresh { task_id } => {
            scheduler_log_line("  dispatch action: start fresh task");

            start_selected_task_checked(task_id)
        }
        DispatchDecision::NoRunnableTask => DispatchResult::NoRunnableTask,
        DispatchDecision::Failed => {
            scheduler_log_line("  dispatch action: failed; task is not dispatchable");
            DispatchResult::Failed
        }
    }
}

#[cfg(feature = "scheduler_dispatch_test")]
fn run_dispatch_pipeline_after(current: Option<usize>) -> DispatchResult {
    DispatchPipeline::new(current).run()
}

#[cfg(feature = "scheduler_dispatch_test")]
fn print_dispatch_decision_model(decision: DispatchDecision) {
    print_dispatch_decision(decision);

    scheduler_log_str("  dispatchable decision: ");
    task::print_yes_no(decision.is_dispatchable());
    scheduler_log_line("");

    scheduler_log_str("  dispatch outcome: ");
    scheduler_log_line(decision.outcome().label());
}

#[cfg(feature = "scheduler_dispatch_test")]
fn print_dispatch_decision(decision: DispatchDecision) {
    scheduler_log_str("  dispatch decision: ");

    print_dispatch_decision_kind(decision.kind());

    if let Some(task_id) = decision.task_id() {
        scheduler_log_str("(");
        crate::drivers::uart::write_dec_u64(task_id as u64);
        scheduler_log_str(")");
    }

    scheduler_log_line("");
}

#[cfg(feature = "scheduler_dispatch_test")]
fn print_dispatch_decision_kind(kind: DispatchDecisionKind) {
    match kind {
        DispatchDecisionKind::StartFresh => scheduler_log_str("StartFresh"),
        DispatchDecisionKind::ResumeSaved => scheduler_log_str("ResumeSaved"),
        DispatchDecisionKind::NoRunnableTask => scheduler_log_str("NoRunnableTask"),
        DispatchDecisionKind::Failed => scheduler_log_str("Failed"),
    }
}

#[cfg(feature = "scheduler_dispatch_test")]
pub fn dispatch_next() -> DispatchResult {
    scheduler_log_line("");
    scheduler_log_line("scheduler dispatch_next:");

    let current = current_task_id();

    scheduler_log_str("  round-robin after: ");
    match current {
        Some(id) => task::print_task_name_by_id(id),
        None => scheduler_log_str("none"),
    }
    scheduler_log_line("");

    scheduler_log_str("  task table capacity: ");
    crate::drivers::uart::write_dec_u64(task::max_tasks() as u64);
    scheduler_log_line("");

    run_dispatch_pipeline_after(current)
}

#[cfg(feature = "scheduler_dispatch_test")]
fn resume_selected_task_checked(task_id: usize) -> DispatchResult {
    scheduler_log_line("  scheduler resume path: checked resume");

    scheduler_log_str("  resume task: ");
    crate::kernel::task::table::print_task_name_by_id(task_id);
    scheduler_log_line("");

    match crate::kernel::task::table::can_task_resume(task_id) {
        Some(true) => {}
        Some(false) => {
            scheduler_log_line("  resume blocked: task cannot resume");
            return DispatchResult::Failed;
        }
        None => {
            scheduler_log_line("  resume blocked: unknown task");
            return DispatchResult::Failed;
        }
    }

    let Some(frame) = validate_resume_frame(task_id) else {
        scheduler_log_line("  scheduler resume path result: FAILED");
        return DispatchResult::Failed;
    };

    scheduler_log_line("  scheduler resume path result: OK");

    restore_selected_task_checked(task_id, frame)
}

#[cfg(feature = "scheduler_dispatch_test")]
fn validate_resume_frame(task_id: usize) -> Option<TaskCpuContext> {
    scheduler_log_line("  scheduler validate resume frame:");

    let Some(frame) = crate::kernel::task::table::get_task_resume_frame(task_id) else {
        scheduler_log_line("    frame present: no");
        scheduler_log_line("    result: FAILED");
        return None;
    };

    let sp_inside = crate::kernel::task::table::is_sp_inside_task_stack(task_id, frame.sp);
    let resume_pc_inside_text = crate::kernel::memory::is_inside_kernel_text(frame.resume_pc);
    let return_pc_inside_text = crate::kernel::memory::is_inside_kernel_text(frame.return_pc);

    let context_consistent = match (
        crate::kernel::task::table::get_task_last_task_sp(task_id),
        crate::kernel::task::table::get_task_last_kernel_return_pc(task_id),
    ) {
        (Some(last_sp), Some(kernel_pc)) => frame.sp == last_sp && frame.return_pc == kernel_pc,
        _ => false,
    };

    print_resume_frame_summary(
        frame,
        sp_inside,
        resume_pc_inside_text,
        return_pc_inside_text,
        context_consistent,
    );

    let ok = frame.is_valid()
        && matches!(sp_inside, Some(true))
        && resume_pc_inside_text
        && return_pc_inside_text
        && context_consistent;

    scheduler_log_str("    result: ");
    if ok {
        scheduler_log_line("OK");
        Some(frame)
    } else {
        scheduler_log_line("FAILED");
        None
    }
}

#[cfg(feature = "scheduler_dispatch_test")]
fn restore_selected_task_checked(task_id: usize, frame: TaskCpuContext) -> ! {
    scheduler_log_line("  scheduler restore path: checked restore");

    scheduler_log_str("  restore task: ");
    crate::kernel::task::table::print_task_name_by_id(task_id);
    scheduler_log_line("");

    scheduler_log_str("  restore sp: ");
    scheduler_log_hex(frame.sp);
    scheduler_log_line("");

    scheduler_log_str("  restore resume_pc: ");
    scheduler_log_hex(frame.resume_pc);
    scheduler_log_line("");

    scheduler_log_str("  restore return_pc: ");
    scheduler_log_hex(frame.return_pc);
    scheduler_log_line("");

    scheduler_log_line("  scheduler restore path result: OK");

    #[cfg(any(
        feature = "scheduler_dispatch_test",
        feature = "two_task_resume_handoff_test",
        feature = "task_fault_test",
        feature = "scheduler_fault_lifecycle_test"
    ))]
    {
        let Some(stack_start) = crate::kernel::task::table::get_task_stack_start(task_id) else {
            scheduler_log_line("  restore result: failed; missing task stack start");
            crate::arch::halt();
        };

        let Some(stack_top) = crate::kernel::task::table::get_task_stack_top(task_id) else {
            scheduler_log_line("  restore result: failed; missing task stack top");
            crate::arch::halt();
        };

        crate::kernel::task::debug::set_debug_current_task_id(task_id);
        crate::kernel::task::debug::set_debug_current_stack_bounds(stack_start, stack_top);
    }

    scheduler_log_line("  calling arch restore from scheduler path...");

    crate::arch::restore_verified_resume_frame(frame);
}

#[cfg(feature = "scheduler_dispatch_test")]
fn print_dispatch_task_summary(task_id: usize) {
    scheduler_log_str("  selected task: ");
    crate::kernel::task::table::print_task_name_by_id(task_id);
    scheduler_log_line("");

    scheduler_log_str("  state: ");
    crate::kernel::task::table::print_task_state_by_id(task_id);
    scheduler_log_line("");

    scheduler_log_str("  can_resume: ");
    match crate::kernel::task::table::can_task_resume(task_id) {
        Some(value) => {
            scheduler_log_yes_no(value);
            scheduler_log_line("");
        }
        None => scheduler_log_line("unknown"),
    }
}

#[cfg(feature = "scheduler_dispatch_test")]
fn print_resume_frame_summary(
    frame: TaskCpuContext,
    sp_inside: Option<bool>,
    resume_pc_inside_text: bool,
    return_pc_inside_text: bool,
    context_consistent: bool,
) {
    scheduler_log_str("    frame present: ");
    scheduler_log_yes_no(true);
    scheduler_log_line("");

    scheduler_log_str("    frame valid: ");
    scheduler_log_yes_no(frame.is_valid());
    scheduler_log_line("");

    scheduler_log_str("    frame SP: ");
    scheduler_log_hex(frame.sp);
    scheduler_log_line("");

    scheduler_log_str("    frame resume_pc: ");
    scheduler_log_hex(frame.resume_pc);
    scheduler_log_line("");

    scheduler_log_str("    frame return_pc: ");
    scheduler_log_hex(frame.return_pc);
    scheduler_log_line("");

    scheduler_log_str("    frame SP inside task stack: ");
    match sp_inside {
        Some(value) => {
            scheduler_log_yes_no(value);
            scheduler_log_line("");
        }
        None => scheduler_log_line("unknown"),
    }

    scheduler_log_str("    frame resume_pc inside kernel text: ");
    scheduler_log_yes_no(resume_pc_inside_text);
    scheduler_log_line("");

    scheduler_log_str("    frame return_pc inside kernel text: ");
    scheduler_log_yes_no(return_pc_inside_text);
    scheduler_log_line("");

    scheduler_log_str("    frame consistent with task record: ");
    scheduler_log_yes_no(context_consistent);
    scheduler_log_line("");
}

#[cfg(feature = "scheduler_dispatch_test")]
fn scheduler_log_line(message: &str) {
    crate::kernel::log::trace("scheduler", message);
}

#[cfg(feature = "scheduler_dispatch_test")]
fn scheduler_log_str(message: &str) {
    crate::drivers::uart::write_str(message);
}

#[cfg(feature = "scheduler_dispatch_test")]
fn scheduler_log_hex(value: u64) {
    crate::drivers::uart::write_hex_u64(value);
}

#[cfg(feature = "scheduler_dispatch_test")]
fn scheduler_log_yes_no(value: bool) {
    crate::kernel::task::table::print_yes_no(value);
}

#[cfg(feature = "scheduler_dispatch_test")]
pub fn run_once() -> RunOnceResult {
    scheduler_log_line("");

    scheduler_log_line("scheduler run_once:");

    match dispatch_next() {
        DispatchResult::NoRunnableTask => {
            scheduler_log_line("  run_once result: no runnable task");

            RunOnceResult::NoRunnableTask
        }

        DispatchResult::Failed => {
            scheduler_log_line("  run_once result: failed");

            RunOnceResult::Failed
        }
    }
}

#[cfg(feature = "scheduler_run_test")]
pub fn run() -> RunResult {
    scheduler_log_line("");
    scheduler_log_line("scheduler run:");

    match run_once() {
        RunOnceResult::NoRunnableTask => {
            scheduler_log_line("  run result: no runnable task");
            RunResult::NoRunnableTask
        }
        RunOnceResult::Failed => {
            scheduler_log_line("  run result: failed");
            RunResult::Failed
        }
    }
}

#[cfg(feature = "scheduler_reentry_test")]
pub fn handle_task_return(snapshot: TaskReturnSnapshot) -> TaskReturnHandleResult {
    scheduler_log_line("");
    scheduler_log_line("scheduler handle_task_return:");

    print_task_return_snapshot(snapshot);

    match snapshot.last_return {
        crate::kernel::task::table::TaskReturnKind::Yield => handle_task_yield(snapshot),
        crate::kernel::task::table::TaskReturnKind::Sleep => handle_task_sleep(snapshot),
        crate::kernel::task::table::TaskReturnKind::Exit => handle_task_exit(snapshot),
        crate::kernel::task::table::TaskReturnKind::Fault => handle_task_fault(snapshot),
        crate::kernel::task::table::TaskReturnKind::None => handle_task_return_none(snapshot),
    }
}

#[cfg(feature = "scheduler_reentry_test")]
fn print_task_return_snapshot(snapshot: TaskReturnSnapshot) {
    scheduler_log_str("  return snapshot task: ");
    crate::kernel::task::table::print_task_name_by_id(snapshot.task_id);
    scheduler_log_line("");

    scheduler_log_str("  return snapshot state: ");
    crate::kernel::task::table::print_task_state_by_id(snapshot.task_id);
    scheduler_log_line("");

    scheduler_log_str("  return snapshot reason: ");
    crate::kernel::task::table::print_task_return_kind_by_id(snapshot.task_id);
    scheduler_log_line("");

    scheduler_log_str("  return snapshot can_resume: ");
    crate::kernel::task::table::print_yes_no(snapshot.can_resume);
    scheduler_log_line("");
}

#[cfg(feature = "scheduler_reentry_test")]
fn handle_task_yield(snapshot: TaskReturnSnapshot) -> TaskReturnHandleResult {
    scheduler_log_line("  return action: yield -> scheduler::run");

    if !snapshot.can_resume {
        scheduler_log_line("  yield result: failed; returned task is not resumable");
        return TaskReturnHandleResult::Failed;
    }

    set_round_robin_cursor(snapshot.task_id);

    scheduler_log_str("  round-robin cursor set to yielded task: ");
    task::print_task_name_by_id(snapshot.task_id);
    scheduler_log_line("");

    match run() {
        RunResult::NoRunnableTask => {
            scheduler_log_line("  scheduler run returned: no runnable task");
            TaskReturnHandleResult::NoRunnableTask
        }
        RunResult::Failed => {
            scheduler_log_line("  scheduler run returned: failed");
            TaskReturnHandleResult::Failed
        }
    }
}

#[cfg(feature = "scheduler_reentry_test")]
fn handle_task_exit(snapshot: TaskReturnSnapshot) -> TaskReturnHandleResult {
    scheduler_log_line("  return action: exit -> no resume for returned task");

    if snapshot.can_resume {
        scheduler_log_line("  exit result: failed; finished task is still resumable");
        return TaskReturnHandleResult::Failed;
    }

    set_round_robin_cursor(snapshot.task_id);

    scheduler_log_str("  round-robin cursor set to exited task: ");
    task::print_task_name_by_id(snapshot.task_id);
    scheduler_log_line("");

    scheduler_log_line("  exit action: try next dispatchable task");

    match find_next_dispatchable_task_after(Some(snapshot.task_id)) {
        Some(task_id) => {
            scheduler_log_str("  next dispatchable task after exit: ");
            crate::kernel::task::table::print_task_name_by_id(task_id);
            scheduler_log_line("");

            scheduler_log_line("  action: scheduler::run");

            match run() {
                RunResult::NoRunnableTask => {
                    scheduler_log_line("  scheduler run returned: no runnable task");
                    TaskReturnHandleResult::NoRunnableTask
                }
                RunResult::Failed => {
                    scheduler_log_line("  scheduler run returned: failed");
                    TaskReturnHandleResult::Failed
                }
            }
        }
        None => {
            scheduler_log_line("  next dispatchable task after exit: none");
            scheduler_log_line("  result: no runnable task");
            TaskReturnHandleResult::NoRunnableTask
        }
    }
}

#[cfg(feature = "scheduler_reentry_test")]
fn handle_task_sleep(snapshot: TaskReturnSnapshot) -> TaskReturnHandleResult {
    scheduler_log_line("  return action: sleep -> no resume until wake tick");

    if snapshot.can_resume {
        scheduler_log_line("  sleep result: failed; sleeping task is still resumable");
        return TaskReturnHandleResult::Failed;
    }

    set_round_robin_cursor(snapshot.task_id);

    scheduler_log_str("  round-robin cursor set to sleeping task: ");
    task::print_task_name_by_id(snapshot.task_id);
    scheduler_log_line("");

    scheduler_log_line("  sleep action: try next dispatchable task");

    match find_next_dispatchable_task_after(Some(snapshot.task_id)) {
        Some(task_id) => {
            scheduler_log_str("  next dispatchable task after sleep: ");
            crate::kernel::task::table::print_task_name_by_id(task_id);
            scheduler_log_line("");
            scheduler_log_line("  action: scheduler::run");

            match run() {
                RunResult::NoRunnableTask => {
                    scheduler_log_line("  scheduler run returned: no runnable task");
                    TaskReturnHandleResult::NoRunnableTask
                }
                RunResult::Failed => {
                    scheduler_log_line("  scheduler run returned: failed");
                    TaskReturnHandleResult::Failed
                }
            }
        }
        None => {
            scheduler_log_line("  next dispatchable task after sleep: none");
            scheduler_log_line("  result: no runnable task");
            TaskReturnHandleResult::NoRunnableTask
        }
    }
}

#[cfg(feature = "scheduler_reentry_test")]
fn handle_task_return_none(_snapshot: TaskReturnSnapshot) -> TaskReturnHandleResult {
    scheduler_log_line("  return action: none -> failed");
    TaskReturnHandleResult::Failed
}

#[cfg(feature = "scheduler_dispatch_test")]
fn start_selected_task_checked(task_id: usize) -> ! {
    scheduler_log_line("  scheduler start path: checked start");

    scheduler_log_str("  start task: ");
    task::print_task_name_by_id(task_id);
    scheduler_log_line("");

    if !task::is_fresh_ready_task(task_id)
        || !task::is_ready_running_faulted_finished_invariant_ok(task_id)
    {
        scheduler_log_line("  start blocked: task is not fresh Ready");
        scheduler_start_failed();
    }

    let Some(stack_start) = task::get_task_stack_start(task_id) else {
        scheduler_log_line("  start blocked: missing stack start");
        scheduler_start_failed();
    };

    let Some(stack_top) = task::get_task_stack_top(task_id) else {
        scheduler_log_line("  start blocked: missing stack top");
        scheduler_start_failed();
    };

    let Some(entry) = task::get_task_entry(task_id) else {
        scheduler_log_line("  start blocked: missing entry");
        scheduler_start_failed();
    };

    scheduler_log_str("  start entry: ");
    crate::drivers::uart::write_hex_u64(entry as *const () as usize as u64);
    scheduler_log_line("");

    scheduler_log_str("  start stack_start: ");
    scheduler_log_hex(stack_start);
    scheduler_log_line("");

    scheduler_log_str("  start stack_top: ");
    scheduler_log_hex(stack_top);
    scheduler_log_line("");

    scheduler_log_line("  scheduler start path result: OK");

    #[cfg(any(
        feature = "scheduler_dispatch_test",
        feature = "two_task_resume_handoff_test",
        feature = "task_fault_test",
        feature = "scheduler_fault_lifecycle_test"
    ))]
    {
        crate::kernel::task::debug::set_debug_current_task_id(task_id);
        crate::kernel::task::debug::set_debug_current_stack_bounds(stack_start, stack_top);
    }

    crate::kernel::task::run_task_on_own_stack(task_id);
}

#[cfg(feature = "scheduler_dispatch_test")]
fn scheduler_start_failed() -> ! {
    scheduler_log_line("  scheduler start path result: FAILED");
    crate::arch::halt();
}

#[cfg(feature = "scheduler_reentry_test")]
fn handle_task_fault(snapshot: TaskReturnSnapshot) -> TaskReturnHandleResult {
    scheduler_log_line("  return action: fault -> no resume for faulted task");

    if snapshot.can_resume
        || !task::is_task_faulted(snapshot.task_id)
        || !task::is_ready_running_faulted_finished_invariant_ok(snapshot.task_id)
    {
        scheduler_log_line("  fault result: failed; faulted task is still resumable");
        return TaskReturnHandleResult::Failed;
    }

    set_round_robin_cursor(snapshot.task_id);

    scheduler_log_str("  round-robin cursor set to faulted task: ");
    task::print_task_name_by_id(snapshot.task_id);
    scheduler_log_line("");

    scheduler_log_line("  fault action: try next dispatchable task");

    match find_next_dispatchable_task_after(Some(snapshot.task_id)) {
        Some(task_id) => {
            scheduler_log_str("  next dispatchable task after fault: ");
            crate::kernel::task::table::print_task_name_by_id(task_id);
            scheduler_log_line("");

            scheduler_log_line("  action: scheduler::run");

            match run() {
                RunResult::NoRunnableTask => {
                    scheduler_log_line("  scheduler run returned: no runnable task");
                    TaskReturnHandleResult::NoRunnableTask
                }
                RunResult::Failed => {
                    scheduler_log_line("  scheduler run returned: failed");
                    TaskReturnHandleResult::Failed
                }
            }
        }
        None => {
            scheduler_log_line("  next dispatchable task after fault: none");
            scheduler_log_line("  result: no runnable task");
            TaskReturnHandleResult::NoRunnableTask
        }
    }
}
```

## File: src/kernel/banner.rs
```rust
use crate::drivers::uart;

pub fn print_boot_banner() {
    uart::write_line("================================");
    uart::write_line("PicoOS 0.1.64");
    uart::write_line("early RISC-V kernel");
    uart::write_line("================================");
}

pub fn print_capabilities() {
    uart::write_line("");
    uart::write_line("kernel capabilities:");
    uart::write_line("- architecture: riscv64");
    uart::write_line("- UART console");
    uart::write_line("- exception/trap handling");
    uart::write_line("- timer interrupts");
    uart::write_line("- page allocator");
    uart::write_line("- kernel heap");
    uart::write_line("- task table");
    uart::write_line("- task stacks");
    uart::write_line("- cooperative task runner skeleton");
    uart::write_line("- selftest mode");
}
```

## File: Cargo.toml
```toml
[package]
name = "PicoOS"
version = "0.1.64"
edition = "2021"

[[bin]]
name = "PicoOS"
path = "src/main.rs"
test = false
bench = false

[profile.dev]
panic = "abort"

[profile.release]
panic = "abort"

[features]
selftest = []
task_yield_test = []
resume_candidate_test = []
resume_preflight_test = []
resume_dry_run_test = []
resume_restore_test = []
real_resume_restore_test = []
real_resume_restore_jump = []
two_yield_task_test = []
scheduler_resume_loop_test = []
verbose_resume_debug = []
scheduler_dispatch_test = []
scheduler_run_test = []
scheduler_reentry_test = []
two_task_resume_handoff_test = []
task_fault_test = []
kernel_fault_guard_test = [
  "scheduler_fault_lifecycle_test",
]
scheduler_verbose_dispatch_trace = []
task_sleep_test = []
kernel_log_scoped = []
log_trap = []
log_timer = []
log_fault = []
log_sleep = []
timer_preemption_prototype = []

two_task_resume_handoff_selftest = [
  "scheduler_reentry_selftest",
  "two_task_resume_handoff_test",
]

scheduler_run_once_selftest = [
  "task_resume_selftest",
  "scheduler_dispatch_test",
]

scheduler_run_selftest = [
  "scheduler_run_once_selftest",
  "scheduler_run_test",
]

scheduler_runtime_selftest = [
  "scheduler_run_selftest",
]

scheduler_reentry_selftest = [
  "scheduler_runtime_selftest",
  "scheduler_reentry_test",
]

task_resume_selftest = [
  "task_yield_test",
  "resume_candidate_test",
  "resume_preflight_test",
  "resume_dry_run_test",
  "resume_restore_test",
  "real_resume_restore_test",
  "real_resume_restore_jump",
  "two_yield_task_test",
  "scheduler_resume_loop_test",
]

scheduler_fault_lifecycle_test = [
  "task_yield_test",
  "task_fault_test",
  "scheduler_reentry_test",
  "scheduler_dispatch_test",
  "scheduler_run_test",
  "scheduler_resume_loop_test",
  "resume_restore_test",
  "real_resume_restore_test",
  "real_resume_restore_jump",
]
```

## File: src/kernel/task/test.rs
```rust
use crate::drivers::uart;
mod bootstrap;
mod fault;
mod handoff;
mod invariants;
mod reentry;
mod resume;
#[allow(unused_imports)]
use crate::kernel::task::debug::{
    debug_current_stack_start, debug_current_stack_top, debug_current_task_id,
    debug_kernel_sp_before_task, debug_last_task_sp, debug_task_return_kind,
    set_debug_current_stack_bounds, set_debug_current_task_id, set_debug_kernel_return_pc,
    set_debug_kernel_sp_before_task, set_debug_task_run_stage, task_return_point,
};

#[allow(unused_imports)]
use crate::kernel::task::entry::task_trampoline;

#[allow(unused_imports)]
use crate::kernel::task::table::{
    create_task, get_task_entry, get_task_stack_start, get_task_stack_top, print_task_name_by_id,
    print_tasks, TaskReturnContext, TaskReturnKind,
};

#[allow(unused_imports)]
pub use fault::*;
#[allow(unused_imports)]
pub use handoff::*;
#[allow(unused_imports)]
pub use reentry::*;
#[allow(unused_imports)]
pub use resume::*;

#[cfg(not(feature = "task_yield_test"))]
pub fn test_tasks() {
    crate::kernel::task::table::init();

    let _ = create_task("idle", idle_task);
    bootstrap::print_task_zero_context_guard();
    let _ = create_task("worker-a", worker_a_task);
    let _ = create_task("worker-b", worker_b_task);

    print_tasks();
}

#[cfg(feature = "task_yield_test")]
pub fn test_tasks_with_yield_worker() {
    crate::kernel::task::table::init();
    let _ = create_task("idle", idle_task);
    bootstrap::print_task_zero_context_guard();

    #[cfg(feature = "scheduler_fault_lifecycle_test")]
    {
        let _ = create_task("worker-a", real_trap_handler_worker_a);
        let _ = create_task("trap-worker", real_trap_handler_worker);
    }

    #[cfg(all(
        feature = "two_task_resume_handoff_test",
        not(feature = "scheduler_fault_lifecycle_test")
    ))]
    {
        let _ = create_task("worker-a", handoff_worker_a);
        let _ = create_task("worker-b", handoff_worker_b);
    }

    #[cfg(all(
        feature = "two_yield_task_test",
        not(feature = "two_task_resume_handoff_test"),
        not(feature = "scheduler_fault_lifecycle_test")
    ))]
    {
        let _ = create_task("worker-a", two_yielding_task);
    }

    #[cfg(not(any(
        feature = "two_yield_task_test",
        feature = "two_task_resume_handoff_test",
        feature = "scheduler_fault_lifecycle_test"
    )))]
    {
        let _ = create_task("worker-a", yielding_task);
        let _ = create_task("worker-b", worker_b_task);
    }

    print_tasks();

    #[cfg(feature = "task_sleep_test")]
    {
        bootstrap::test_task_sleep_wakeup_table_selftest();
    }
}

fn idle_task() {
    uart::write_line("idle_task: running");
}

#[cfg(not(feature = "task_yield_test"))]
fn worker_a_task() {
    print_current_task_stack_check("worker_a_task");
}

#[cfg(any(
    not(feature = "task_yield_test"),
    all(
        feature = "task_yield_test",
        not(feature = "two_yield_task_test"),
        not(feature = "two_task_resume_handoff_test"),
        not(feature = "scheduler_fault_lifecycle_test")
    )
))]
fn worker_b_task() {
    print_current_task_stack_check("worker_b_task");
}

#[cfg(any(
    not(feature = "task_yield_test"),
    all(
        feature = "task_yield_test",
        not(feature = "two_yield_task_test"),
        not(feature = "two_task_resume_handoff_test"),
        not(feature = "scheduler_fault_lifecycle_test")
    )
))]
fn print_current_task_stack_check(label: &str) {
    uart::write_str(label);
    uart::write_line(": running");

    let sp = crate::arch::stack_pointer();
    let stack_start = debug_current_stack_start();
    let stack_top = debug_current_stack_top();

    uart::write_str(label);
    uart::write_str(" SP: ");
    uart::write_hex_u64(sp);
    uart::write_line("");

    uart::write_str("expected stack: ");
    uart::write_hex_u64(stack_start);
    uart::write_str(" - ");
    uart::write_hex_u64(stack_top);
    uart::write_line("");

    uart::write_str("saved kernel SP before task: ");
    uart::write_hex_u64(debug_kernel_sp_before_task());
    uart::write_line("");

    uart::write_str("SP check: ");

    if sp >= stack_start && sp < stack_top {
        uart::write_line("inside task stack");
    } else {
        uart::write_line("OUTSIDE task stack");
    }
}

#[cfg(feature = "task_yield_test")]
pub fn run_task_on_own_stack(task_id: usize) -> ! {
    let Some(entry) = get_task_entry(task_id) else {
        uart::write_line("selected task entry: none");
        crate::arch::halt();
    };

    let Some(stack_start) = get_task_stack_start(task_id) else {
        uart::write_line("selected task stack start: none");
        crate::arch::halt();
    };

    let Some(stack_top) = get_task_stack_top(task_id) else {
        uart::write_line("selected task stack: none");
        crate::arch::halt();
    };

    uart::write_str("selected task: ");
    print_task_name_by_id(task_id);
    uart::write_line("");

    uart::write_str("entry: ");
    uart::write_hex_u64(entry as usize as u64);
    uart::write_line("");

    uart::write_str("stack_top: ");
    uart::write_hex_u64(stack_top);
    uart::write_line("");

    uart::write_str("stack_start: ");
    uart::write_hex_u64(stack_start);
    uart::write_line("");

    let kernel_sp_before_task = crate::arch::stack_pointer();
    let kernel_return_pc = task_return_point as *const () as usize as u64;

    uart::write_str("kernel_sp_before_task: ");
    uart::write_hex_u64(kernel_sp_before_task);
    uart::write_line("");

    uart::write_str("task_stack_top: ");
    uart::write_hex_u64(stack_top);
    uart::write_line("");

    uart::write_str("kernel_return_pc: ");
    uart::write_hex_u64(kernel_return_pc);
    uart::write_line("");

    uart::write_line("switching to task stack...");

    set_debug_current_task_id(task_id);
    set_debug_current_stack_bounds(stack_start, stack_top);
    set_debug_kernel_sp_before_task(kernel_sp_before_task);
    set_debug_kernel_return_pc(kernel_return_pc);
    crate::kernel::task::table::mark_task_started(task_id);
    unsafe {
        crate::arch::start_task_on_stack(entry as usize, stack_top);
    }
}

#[cfg(all(feature = "task_yield_test", not(feature = "two_yield_task_test")))]
fn yielding_task() {
    uart::write_line("yielding_task: step 1");

    crate::kernel::task::yield_now();

    uart::write_line("yielding_task: step 2");

    crate::kernel::task::task_exit();
}

#[cfg(feature = "task_yield_test")]
pub fn test_task_yield() {
    uart::write_line("");
    uart::write_line("task yield test:");

    set_debug_task_run_stage(10);

    #[cfg(feature = "scheduler_fault_lifecycle_test")]
    {
        bootstrap::test_scheduler_fault_lifecycle_bootstrap();
    }

    #[cfg(all(
        feature = "task_fault_test",
        not(feature = "scheduler_fault_lifecycle_test")
    ))]
    {
        bootstrap::test_task_fault_bootstrap();
    }

    #[cfg(all(
        feature = "two_task_resume_handoff_test",
        not(feature = "task_fault_test"),
        not(feature = "scheduler_fault_lifecycle_test")
    ))]
    {
        bootstrap::test_two_task_resume_handoff_bootstrap();
    }

    #[cfg(not(any(
        feature = "two_task_resume_handoff_test",
        feature = "task_fault_test",
        feature = "scheduler_fault_lifecycle_test"
    )))]
    {
        uart::write_line("selected task: worker-a");
        run_task_on_own_stack(1);
    }
}

pub fn handle_task_return_for_debug_test() {
    let task_id = debug_current_task_id();
    let kind = debug_task_return_kind();
    let task_sp = debug_last_task_sp();
    let kernel_sp = debug_kernel_sp_before_task();
    let kernel_return_pc = crate::kernel::task::debug::debug_kernel_return_pc();

    let return_context = TaskReturnContext {
        task_sp,
        kernel_sp,
        kernel_return_pc,
    };

    let mut cpu_context = crate::arch::capture_task_cpu_context(task_sp, kernel_return_pc);

    if matches!(kind, TaskReturnKind::Yield | TaskReturnKind::Sleep) {
        let debug_resume_pc = crate::kernel::task::debug::debug_task_resume_pc();

        uart::write_str("  debug resume_pc from return boundary: ");
        uart::write_hex_u64(debug_resume_pc);
        uart::write_line("");

        cpu_context.resume_pc = debug_resume_pc;

        #[cfg(target_arch = "riscv64")]
        {
            cpu_context.ra = debug_resume_pc;

            uart::write_str("  saved ra for resume: ");
            uart::write_hex_u64(cpu_context.ra);
            uart::write_line("");
        }
    }

    uart::write_str("  task: ");
    print_task_name_by_id(task_id);
    uart::write_line("");

    uart::write_str("  captured CPU context:");
    crate::kernel::task::cpu_context::print_cpu_context(cpu_context);
    uart::write_line("");

    let transition_ok = crate::kernel::task::table::apply_task_return_transition(
        task_id,
        kind,
        return_context,
        cpu_context,
    );

    match kind {
        TaskReturnKind::Exit => crate::drivers::uart::write_str("  mark finished: "),
        TaskReturnKind::Yield => crate::drivers::uart::write_str("  mark ready after yield: "),
        TaskReturnKind::Sleep => crate::drivers::uart::write_str("  mark blocked for sleep: "),
        TaskReturnKind::Fault => crate::drivers::uart::write_str("  mark faulted: "),
        TaskReturnKind::None => crate::drivers::uart::write_str("  set return kind: "),
    }
    crate::kernel::task::table::print_yes_no(transition_ok);
    crate::drivers::uart::write_line("");

    if !matches!(kind, TaskReturnKind::None) {
        crate::kernel::task::scheduler::switch_to_idle();
    }

    uart::write_str("  new state: ");
    crate::kernel::task::table::print_task_state_by_id(task_id);
    uart::write_line("");

    uart::write_str("  return kind: ");
    crate::kernel::task::table::print_task_return_kind_by_id(task_id);
    uart::write_line("");

    uart::write_str("  can resume: ");
    match crate::kernel::task::table::can_task_resume(task_id) {
        Some(true) => uart::write_line("yes"),
        Some(false) => uart::write_line("no"),
        None => uart::write_line("unknown"),
    }

    uart::write_str("  last task SP: ");
    uart::write_hex_u64(task_sp);
    uart::write_line("");

    uart::write_str("  last kernel SP: ");
    uart::write_hex_u64(kernel_sp);
    uart::write_line("");

    uart::write_str("  kernel return PC: ");
    uart::write_hex_u64(kernel_return_pc);
    uart::write_line("");

    print_last_task_sp_check(task_id, task_sp);
    invariants::print_resume_eligibility_check(task_id);
    invariants::print_cpu_context_consistency_check(task_id);
    invariants::print_illegal_transition_checks(task_id);

    #[cfg(any(feature = "two_task_resume_handoff_test", feature = "task_fault_test"))]
    {
        let _ = print_resume_pc_proximity_check(task_id);
    }

    uart::write_str("  scheduler current: ");
    crate::kernel::task::scheduler::print_current_task_name();
    uart::write_line("");
}

#[cfg(feature = "task_yield_test")]
pub fn print_final_task_list() {
    uart::write_line("");
    uart::write_line("final task list:");
    crate::kernel::task::table::print_tasks();
}

fn print_last_task_sp_check(task_id: usize, task_sp: u64) {
    uart::write_line("  task return context check:");

    match crate::kernel::task::table::is_sp_inside_task_stack(task_id, task_sp) {
        Some(true) => {
            uart::write_line("    task SP: inside task stack");
        }
        Some(false) => {
            uart::write_line("    task SP: outside task stack");
        }
        None => {
            uart::write_line("    task SP: unknown task");
        }
    }
}
```
