#!/usr/bin/env bash
set -euo pipefail

cargo clean

cargo build --features "two_task_resume_handoff_selftest"

scripts/qemu-expect.sh "scheduler resume loop result: OK"
