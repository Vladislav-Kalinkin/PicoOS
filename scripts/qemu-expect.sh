#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "usage: $0 <success-marker>" >&2
  exit 2
fi

marker=$1
timeout_seconds=${QEMU_EXPECT_TIMEOUT:-20}
tmp_dir=$(mktemp -d "${TMPDIR:-/tmp}/picoos-qemu.XXXXXX")
log_pipe="$tmp_dir/out"
mkfifo "$log_pipe"

qemu-system-riscv64 \
  -machine virt \
  -nographic \
  -bios none \
  -kernel target/riscv64gc-unknown-none-elf/debug/PicoOS \
  >"$log_pipe" 2>&1 &
qemu_pid=$!
status=1

cleanup() {
  kill "$qemu_pid" 2>/dev/null || true
  wait "$qemu_pid" 2>/dev/null || true
  rm -rf "$tmp_dir"
}
trap cleanup EXIT

(
  sleep "$timeout_seconds"
  kill "$qemu_pid" 2>/dev/null || true
) &
watchdog_pid=$!

while IFS= read -r line; do
  printf '%s\n' "$line"
  clean_line=${line%$'\r'}

  if [[ "$clean_line" == "$marker" ]]; then
    status=0
    break
  fi

  if [[ "$clean_line" == *"FAILED"* || "$clean_line" == *"system halted after trap"* ]]; then
    status=1
    break
  fi
done <"$log_pipe"

kill "$watchdog_pid" 2>/dev/null || true
exit "$status"
