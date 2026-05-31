#!/usr/bin/env bash
set -euo pipefail

cargo clean

cargo build --features "scheduler_runtime_selftest"

scripts/qemu-expect.sh "scheduler resume loop result: OK"
