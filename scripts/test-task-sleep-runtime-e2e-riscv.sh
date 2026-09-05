#!/usr/bin/env bash
set -euo pipefail

scripts/qemu-expect.sh "task sleep runtime e2e result: OK" 3
