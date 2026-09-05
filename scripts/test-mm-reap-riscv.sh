#!/usr/bin/env bash
set -euo pipefail

scripts/qemu-expect.sh "mm leak check: OK" 6
