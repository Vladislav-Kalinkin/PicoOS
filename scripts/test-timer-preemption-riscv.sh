#!/usr/bin/env bash
set -euo pipefail

cargo clean

cargo build --features "timer_preemption_selftest"

scripts/qemu-expect.sh "timer preemption result: OK"
