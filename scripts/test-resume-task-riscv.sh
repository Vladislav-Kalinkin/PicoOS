#!/usr/bin/env bash
set -euo pipefail

cargo clean
cargo build --target riscv64gc-unknown-none-elf --features "task_yield_test resume_candidate_test resume_preflight_test resume_task_test"

qemu-system-riscv64 \
  -M virt \
  -m 128M \
  -nographic \
  -bios none \
  -kernel target/riscv64gc-unknown-none-elf/debug/PicoOS
