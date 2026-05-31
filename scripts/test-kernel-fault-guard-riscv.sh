#!/usr/bin/env bash
set -euo pipefail

cargo clean

cargo build \
  --target riscv64gc-unknown-none-elf \
  --features "kernel_fault_guard_test"

scripts/qemu-expect.sh "kernel fault guard result: OK"
