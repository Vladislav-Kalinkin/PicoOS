#!/usr/bin/env bash
set -euo pipefail

cargo clean
cargo build --features "task_yield_test resume_candidate_test resume_preflight_test resume_dry_run_test resume_restore_test"

qemu-system-aarch64 \
  -M virt \
  -cpu cortex-a57 \
  -display none \
  -serial stdio \
  -kernel target/aarch64-unknown-none/debug/PicoOS
