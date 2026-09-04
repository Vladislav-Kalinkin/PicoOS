#!/usr/bin/env bash
set -euo pipefail

cargo clean

cargo build --features "scenario_sleep"

scripts/qemu-expect.sh "task sleep runtime e2e result: OK"
