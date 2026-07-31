#!/usr/bin/env bash
set -euo pipefail

cargo clean

cargo build --features "task_sleep_runtime_e2e_selftest"

scripts/qemu-expect.sh "task sleep runtime e2e result: OK"
