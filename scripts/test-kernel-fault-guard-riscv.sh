#!/usr/bin/env bash
set -euo pipefail

cargo clean

cargo build \
  --target riscv64gc-unknown-none-elf \
  --features "kernel_fault_guard_test"

qemu-system-riscv64 \
  -machine virt \
  -nographic \
  -bios none \
  -kernel target/riscv64gc-unknown-none-elf/debug/PicoOS
