#!/usr/bin/env bash
set -euo pipefail

scripts/qemu-expect.sh "task fault scheduler result: OK" 4
