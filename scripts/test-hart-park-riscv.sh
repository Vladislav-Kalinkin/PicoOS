#!/usr/bin/env bash
set -euo pipefail

# Park smoke: -smp 2 must keep secondaries out of kernel_main.
# Marker is hart 0's topology print; a second banner line is a fail.
QEMU_SMP=2 QEMU_BANNER_LIMIT=1 scripts/qemu-expect.sh "harts present: 1" 0
