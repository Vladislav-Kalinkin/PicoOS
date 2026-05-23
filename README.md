# PicoOS

PicoOS is a small experimental operating system kernel written in Rust for **RISC-V 64-bit**.

The project is focused on learning and building core OS concepts step by step: boot process, UART output, traps, timer interrupts, memory layout, page allocation, heap allocation, task stacks, cooperative task switching, and task resume.

PicoOS is currently designed to run on **QEMU RISC-V virt**.

## Status

PicoOS is now a **RISC-V-first** project.

Earlier experimental ARM64 support was removed from the main codebase to keep the kernel smaller, cleaner, and easier to evolve. The ARM64/multiarch version is preserved separately as legacy history.

Current working milestone:

- RISC-V boot in QEMU
- UART console output
- trap/exception handling
- timer interrupt support
- page allocator
- simple kernel heap
- task table
- separate task stacks
- cooperative `yield`
- task resume after yield
- repeated yield/resume test
- scheduler-oriented resume loop test

## Architecture

Current target:

```text
riscv64gc-unknown-none-elf
```

The project is configured to use the RISC-V target by default, so most commands do not need an explicit `--target` flag.

## Requirements

You need:

- Rust nightly or a Rust toolchain capable of building `no_std` bare-metal code
- RISC-V target:

```bash
rustup target add riscv64gc-unknown-none-elf
```

- QEMU with RISC-V support:

```bash
qemu-system-riscv64
```

## Build

```bash
cargo build
```

Because `.cargo/config.toml` sets the default target to RISC-V, this builds for:

```text
riscv64gc-unknown-none-elf
```

## Run in QEMU

A typical QEMU command:

```bash
qemu-system-riscv64 \
  -machine virt \
  -nographic \
  -bios none \
  -kernel target/riscv64gc-unknown-none-elf/debug/PicoOS
```

Depending on the current test scenario, use the scripts in:

```text
scripts/
```

## Checks

Run all available checks:

```bash
./scripts/check-all.sh
```

This currently builds and checks the RISC-V kernel, including selftest configurations.

## Feature Flags

PicoOS uses feature flags for experimental kernel tests.

Common features:

```text
selftest
task_bootstrap_test
task_stack_switch_test
sequential_task_test
task_yield_test
scheduler_skip_finished_test
scheduler_driven_task_test
resume_candidate_test
resume_preflight_test
resume_dry_run_test
resume_restore_test
real_resume_restore_test
real_resume_restore_jump
two_yield_task_test
scheduler_resume_loop_test
verbose_resume_debug
```

Example:

```bash
cargo build --features "task_yield_test resume_candidate_test resume_preflight_test resume_dry_run_test resume_restore_test"
```

## Current Cooperative Task Resume Milestone

The current important milestone is a working RISC-V cooperative task resume flow:

```text
task -> yield -> kernel -> restore -> task -> yield -> kernel -> restore -> task -> exit -> kernel
```

This proves that PicoOS can:

- start a task on its own stack
- save task context during yield
- return to the kernel
- restore the task
- continue execution after yield
- yield again
- resume again
- exit cleanly
- mark the task as finished

## Project Direction

The next development direction is to continue building PicoOS around RISC-V:

- cleaner cooperative scheduler
- better task state transitions
- timer-driven scheduling experiments
- syscall layer
- user mode experiments
- memory protection
- filesystem and storage experiments later

The goal is not to build a production OS immediately, but to create a small, understandable kernel where each subsystem is built and tested step by step.

## License

License is not defined yet.
