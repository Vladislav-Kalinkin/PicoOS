#!/usr/bin/env bash
set -euo pipefail

cargo build --features "selftest"

scripts/qemu-expect.sh "mm leak check: OK"
