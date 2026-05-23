#!/usr/bin/env bash
set -euo pipefail

cargo clean

cargo build --features "scheduler_run_once_selftest"

qemu-system-riscv64 \
  -machine virt \
  -nographic \
  -bios none \
  -kernel target/riscv64gc-unknown-none-elf/debug/PicoOS
