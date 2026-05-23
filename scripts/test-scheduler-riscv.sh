#!/usr/bin/env bash
set -euo pipefail

cargo clean
cargo build --target riscv64gc-unknown-none-elf --features scheduler_driven_task_test

qemu-system-riscv64 \
  -M virt \
  -m 128M \
  -nographic \
  -bios none \
  -kernel target/riscv64gc-unknown-none-elf/debug/PicoOS
