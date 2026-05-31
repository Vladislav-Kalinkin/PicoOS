#!/usr/bin/env bash
set -euo pipefail

cargo clean

cargo build --features "scheduler_run_once_selftest"

scripts/qemu-expect.sh "scheduler resume loop result: OK"
