#!/usr/bin/env bash
set -euo pipefail

cargo build --features "scenario_reap"

scripts/qemu-expect.sh "mm leak check: OK"
