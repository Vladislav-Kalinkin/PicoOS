#!/usr/bin/env bash
set -euo pipefail

cargo clean

cargo build \
  --target riscv64gc-unknown-none-elf \
  --features "task_yield_test task_fault_test trap_to_task_fault_test real_trap_handler_classification_test scheduler_reentry_test scheduler_dispatch_test scheduler_run_test scheduler_resume_loop_test resume_restore_test real_resume_restore_test real_resume_restore_jump"

qemu-system-riscv64 \
  -machine virt \
  -nographic \
  -bios none \
  -kernel target/riscv64gc-unknown-none-elf/debug/PicoOS
