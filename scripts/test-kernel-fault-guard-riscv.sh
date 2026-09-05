#!/usr/bin/env bash
set -euo pipefail

scripts/qemu-expect.sh "kernel fault guard result: OK" 7
