#!/usr/bin/env bash
set -euo pipefail

cargo clean

cargo build --features "scenario_preempt"

scripts/qemu-expect.sh "timer preemption result: OK"
