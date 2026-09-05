#!/usr/bin/env bash
set -euo pipefail

scripts/qemu-expect.sh "scheduler resume loop result: OK" 2
