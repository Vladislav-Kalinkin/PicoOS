#!/usr/bin/env bash
set -euo pipefail

scripts/qemu-expect.sh "spawn join leak: OK" 0
