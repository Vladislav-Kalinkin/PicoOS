#!/usr/bin/env bash
set -euo pipefail

cargo clean

cargo build

scripts/qemu-expect.sh "spawn join leak: OK"
