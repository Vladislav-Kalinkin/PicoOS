#!/usr/bin/env bash
set -euo pipefail

scripts/qemu-expect.sh "task sleep wake result: OK" 3
