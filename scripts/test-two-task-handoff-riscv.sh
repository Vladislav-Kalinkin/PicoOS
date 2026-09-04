#!/usr/bin/env bash
set -euo pipefail

cargo clean

cargo build --features "scenario_handoff"

scripts/qemu-expect.sh "scheduler resume loop result: OK"
