#!/usr/bin/env bash
set -euo pipefail

echo "== PicoOS check-all =="

echo
echo "== Build RISC-V =="
cargo build

echo
echo "== Build RISC-V selftest =="
cargo build --features "selftest"

echo
echo "== Build RISC-V task resume selftest =="
cargo build --features "task_resume_selftest"

echo
echo "== Clippy RISC-V =="
cargo clippy -- -D warnings

echo
echo "== Clippy RISC-V selftest =="
cargo clippy --features "selftest" -- -D warnings

echo
echo "== Clippy RISC-V task resume selftest =="
cargo clippy --features "task_resume_selftest" -- -D warnings

echo
echo "== Build RISC-V scheduler run_once selftest =="
cargo build --features "scheduler_run_once_selftest"

echo
echo "== Clippy RISC-V scheduler run_once selftest =="
cargo clippy --features "scheduler_run_once_selftest" -- -D warnings

echo
echo "== Build RISC-V scheduler runtime selftest =="
cargo build --features "scheduler_runtime_selftest"

echo
echo "== Clippy RISC-V scheduler runtime selftest =="
cargo clippy --features "scheduler_runtime_selftest" -- -D warnings

echo
echo "== Build RISC-V scheduler reentry selftest =="
cargo build --features "scheduler_reentry_selftest"

echo
echo "== Clippy RISC-V scheduler reentry selftest =="
cargo clippy --features "scheduler_reentry_selftest" -- -D warnings

echo
echo "== All checks passed =="
cargo clean
