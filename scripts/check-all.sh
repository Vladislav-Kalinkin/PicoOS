#!/usr/bin/env bash
set -euo pipefail

# Aggressive Clippy bar for PicoOS Hygiene.
# Cargo.toml [lints.clippy] already denies these groups; the flags are
# repeated so a bare `cargo clippy` invocation in this script cannot
# silently drop back to warn-only.
CLIPPY_FLAGS=(
  -D warnings
  -D clippy::all
  -D clippy::pedantic
  -D clippy::nursery
  -D clippy::cargo
  -D clippy::dbg_macro
  -D clippy::todo
  -D clippy::unimplemented
  -D clippy::unwrap_used
  -D clippy::expect_used
  -D clippy::float_arithmetic
  -D clippy::lossy_float_literal
  -D clippy::mem_forget
  -A clippy::fn_to_numeric_cast_any
  -D clippy::rest_pat_in_fully_bound_structs
  -D clippy::empty_structs_with_brackets
  -D clippy::same_name_method
  -D clippy::wildcard_dependencies
  -D clippy::exit
  -A clippy::cargo_common_metadata
  -A clippy::multiple_crate_versions
  -A clippy::inline_always
  -A clippy::wildcard_imports
  -A clippy::too_many_lines
  -A clippy::missing_errors_doc
  -A clippy::missing_panics_doc
  -A clippy::must_use_candidate
  -A clippy::missing_safety_doc
  -A clippy::undocumented_unsafe_blocks
  -A clippy::multiple_unsafe_ops_per_block
  -A clippy::module_name_repetitions
  -A clippy::large_stack_arrays
  -A clippy::cast_possible_truncation
  -A clippy::cast_possible_wrap
  -A clippy::cast_sign_loss
  -A clippy::cast_lossless
  -A clippy::unreadable_literal
  -A clippy::ptr_as_ptr
  -A clippy::borrow_as_ptr
  -A clippy::similar_names
  -A clippy::struct_excessive_bools
  -A clippy::unused_self
  -A clippy::option_if_let_else
  -A clippy::map_unwrap_or
  -A clippy::single_match
  -A clippy::single_match_else
  -A clippy::match_single_binding
  -A clippy::missing_const_for_fn
  -A clippy::redundant_pub_crate
  -A clippy::use_self
)

clippy_riscv() {
  local label=$1
  shift
  echo
  echo "== Clippy RISC-V ${label} =="
  cargo clippy "$@" -- "${CLIPPY_FLAGS[@]}"
}

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

clippy_riscv "default"
clippy_riscv "selftest" --features "selftest"
clippy_riscv "task resume selftest" --features "task_resume_selftest"

echo
echo "== Build RISC-V task sleep selftest =="
cargo build --features "task_resume_selftest,task_sleep_test"

clippy_riscv "task sleep selftest" --features "task_resume_selftest,task_sleep_test"

echo
echo "== Build RISC-V task sleep runtime e2e selftest =="
cargo build --features "task_sleep_runtime_e2e_selftest"

clippy_riscv "task sleep runtime e2e selftest" --features "task_sleep_runtime_e2e_selftest"

echo
echo "== Build RISC-V scheduler run_once selftest =="
cargo build --features "scheduler_run_once_selftest"

clippy_riscv "scheduler run_once selftest" --features "scheduler_run_once_selftest"

echo
echo "== Build RISC-V scheduler runtime selftest =="
cargo build --features "scheduler_runtime_selftest"

clippy_riscv "scheduler runtime selftest" --features "scheduler_runtime_selftest"

echo
echo "== Build RISC-V scheduler reentry selftest =="
cargo build --features "scheduler_reentry_selftest"

clippy_riscv "scheduler reentry selftest" --features "scheduler_reentry_selftest"

echo
echo "== Build RISC-V two-task handoff selftest =="
cargo build --features "two_task_resume_handoff_selftest"

clippy_riscv "two-task handoff selftest" --features "two_task_resume_handoff_selftest"

echo
echo "== Build RISC-V scheduler fault lifecycle test =="
cargo build --features "scheduler_fault_lifecycle_test"

clippy_riscv "scheduler fault lifecycle test" --features "scheduler_fault_lifecycle_test"

echo
echo "== Build RISC-V kernel fault guard test =="
cargo build --features "kernel_fault_guard_test"

clippy_riscv "kernel fault guard test" --features "kernel_fault_guard_test"

echo
echo "== QEMU marker tests =="
scripts/test-task-resume-selftest.sh
scripts/test-task-sleep-riscv.sh
scripts/test-task-sleep-runtime-e2e-riscv.sh
scripts/test-two-task-handoff-riscv.sh
scripts/test-scheduler-fault-lifecycle-riscv.sh
scripts/test-kernel-fault-guard-riscv.sh

echo
echo "== All checks passed =="
cargo clean
