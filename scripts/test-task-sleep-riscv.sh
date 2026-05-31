#!/usr/bin/env bash
set -euo pipefail

cargo clean

cargo build --features "task_resume_selftest,task_sleep_test"

scripts/qemu-expect.sh "task sleep wake result: OK"
