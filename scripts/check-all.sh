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
  -D clippy::too_many_lines
  -D clippy::manual_memcpy
  -A clippy::missing_errors_doc
  -A clippy::missing_panics_doc
  -A clippy::must_use_candidate
  -D clippy::missing_safety_doc
  -D clippy::undocumented_unsafe_blocks
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

echo "== PicoOS check-all =="

echo
echo "== Build RISC-V =="
cargo build

echo
echo "== usertext symbol contract =="
scripts/check-usertext.sh

echo
echo "== Clippy RISC-V default =="
cargo clippy -- "${CLIPPY_FLAGS[@]}"

echo
echo "== QEMU marker tests =="
scripts/test-default-riscv.sh
scripts/test-task-resume-selftest.sh
scripts/test-two-task-handoff-riscv.sh
scripts/test-task-sleep-riscv.sh
scripts/test-task-sleep-runtime-e2e-riscv.sh
scripts/test-scheduler-fault-lifecycle-riscv.sh
scripts/test-timer-preemption-riscv.sh
scripts/test-mm-reap-riscv.sh
scripts/test-kernel-fault-guard-riscv.sh
scripts/test-ipc-riscv.sh
scripts/test-user-text-riscv.sh

echo
echo "== All checks passed =="
