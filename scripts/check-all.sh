#!/usr/bin/env bash
set -euo pipefail

echo "== PicoOS check-all =="
echo ""


echo "== Build RISC-V =="
cargo build


echo ""
echo "== Build RISC-V selftest =="
cargo build --features selftest


echo ""
echo "== Clippy RISC-V =="
cargo clippy


echo ""
echo "== Clippy RISC-V selftest =="
cargo clippy --features selftest

echo ""
echo "== All checks passed =="

cargo clean
