#!/usr/bin/env bash
set -euo pipefail

echo "== PicoOS check-all =="
echo ""

echo "== Build ARM64 =="
cargo build

echo ""
echo "== Build RISC-V =="
cargo build --target riscv64gc-unknown-none-elf

echo ""
echo "== Build ARM64 selftest =="
cargo build --features selftest

echo ""
echo "== Build RISC-V selftest =="
cargo build --target riscv64gc-unknown-none-elf --features selftest

echo ""
echo "== Clippy ARM64 =="
cargo clippy

echo ""
echo "== Clippy RISC-V =="
cargo clippy --target riscv64gc-unknown-none-elf

echo ""
echo "== Clippy ARM64 selftest =="
cargo clippy --features selftest

echo ""
echo "== Clippy RISC-V selftest =="
cargo clippy --target riscv64gc-unknown-none-elf --features selftest

echo ""
echo "== All checks passed =="

cargo clean
