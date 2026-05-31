#!/usr/bin/env bash
set -euo pipefail

cargo clean

cargo build \
  --target riscv64gc-unknown-none-elf \
  --features "scheduler_fault_lifecycle_test"

scripts/qemu-expect.sh "task fault scheduler result: OK"
