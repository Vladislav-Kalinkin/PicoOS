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
echo "== Build RISC-V task sleep selftest =="
cargo build --features "task_resume_selftest,task_sleep_test"

echo
echo "== Clippy RISC-V task sleep selftest =="
cargo clippy --features "task_resume_selftest,task_sleep_test" -- -D warnings

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
echo "== Build RISC-V two-task handoff selftest =="
cargo build --features "two_task_resume_handoff_selftest"

echo
echo "== Clippy RISC-V two-task handoff selftest =="
cargo clippy --features "two_task_resume_handoff_selftest" -- -D warnings

echo
echo "== Build RISC-V scheduler fault lifecycle test =="
cargo build --features "scheduler_fault_lifecycle_test"

echo
echo "== Clippy RISC-V scheduler fault lifecycle test =="
cargo clippy --features "scheduler_fault_lifecycle_test" -- -D warnings

echo
echo "== Build RISC-V kernel fault guard test =="
cargo build --features "kernel_fault_guard_test"

echo
echo "== Clippy RISC-V kernel fault guard test =="
cargo clippy --features "kernel_fault_guard_test" -- -D warnings

echo
echo "== QEMU marker tests =="
scripts/test-task-resume-selftest.sh
scripts/test-task-sleep-riscv.sh
scripts/test-two-task-handoff-riscv.sh
scripts/test-scheduler-fault-lifecycle-riscv.sh
scripts/test-kernel-fault-guard-riscv.sh

echo
echo "== All checks passed =="
cargo clean
