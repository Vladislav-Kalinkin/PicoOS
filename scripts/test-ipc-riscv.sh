#!/usr/bin/env bash
set -euo pipefail

scripts/qemu-expect.sh "ipc rendezvous: OK" 8
