#!/usr/bin/env bash
set -euo pipefail

cargo clean
cargo build --features scheduler_driven_task_test

qemu-system-aarch64 \
  -M virt \
  -cpu cortex-a57 \
  -display none \
  -serial stdio \
  -kernel target/aarch64-unknown-none/debug/PicoOS
