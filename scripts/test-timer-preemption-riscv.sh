#!/usr/bin/env bash
set -euo pipefail

scripts/qemu-expect.sh "timer preemption result: OK" 5
