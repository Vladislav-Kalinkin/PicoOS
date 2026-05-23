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
echo "== All checks passed =="
cargo clean
