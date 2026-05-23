#!/usr/bin/env bash
set -euo pipefail

cargo clean

cargo build \
  --features "task_yield_test two_yield_task_test scheduler_resume_loop_test resume_candidate_test resume_preflight_test resume_dry_run_test resume_restore_test real_resume_restore_test real_resume_restore_jump"

qemu-system-riscv64 \
  -machine virt \
  -nographic \
  -bios none \
  -kernel target/riscv64gc-unknown-none-elf/debug/PicoOS
