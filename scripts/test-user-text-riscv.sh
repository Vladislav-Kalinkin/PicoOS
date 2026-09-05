#!/usr/bin/env bash
set -euo pipefail

scripts/qemu-expect.sh "user text: kernel fetch deny OK" 9
