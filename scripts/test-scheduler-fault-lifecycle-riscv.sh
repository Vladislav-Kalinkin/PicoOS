#!/usr/bin/env bash
set -euo pipefail

cargo clean

cargo build \
  --target riscv64gc-unknown-none-elf \
  --features "scenario_fault"

scripts/qemu-expect.sh "task fault scheduler result: OK"
