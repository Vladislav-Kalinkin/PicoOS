#!/usr/bin/env bash
set -euo pipefail

cargo clean

cargo build \
  --target riscv64gc-unknown-none-elf \
  --features "scenario_kernel_fault"

scripts/qemu-expect.sh "kernel fault guard result: OK"
