#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "usage: $0 <success-marker> [contract-byte]" >&2
  exit 2
fi

marker=$1
contract_byte=${2:-0}
timeout_seconds=${QEMU_EXPECT_TIMEOUT:-20}
kernel_src=target/riscv64gc-unknown-none-elf/debug/PicoOS

if [[ ! -f "$kernel_src" ]]; then
  echo "qemu-expect: missing $kernel_src (run cargo build first)" >&2
  exit 1
fi

find_tool() {
  local name=$1
  shift
  local cand
  for cand in "$@"; do
    if command -v "$cand" >/dev/null 2>&1; then
      printf '%s\n' "$cand"
      return 0
    fi
    if [[ -x "$cand" ]]; then
      printf '%s\n' "$cand"
      return 0
    fi
  done
  echo "qemu-expect: $name not found" >&2
  exit 1
}

objdump_bin=$(find_tool llvm-objdump \
  llvm-objdump \
  /opt/homebrew/opt/llvm/bin/llvm-objdump \
  /usr/bin/llvm-objdump \
  rust-objdump)
objcopy_bin=$(find_tool llvm-objcopy \
  llvm-objcopy \
  /opt/homebrew/opt/llvm/bin/llvm-objcopy \
  /usr/bin/llvm-objcopy \
  rust-objcopy)

tmp_dir=$(mktemp -d "${TMPDIR:-/tmp}/picoos-qemu.XXXXXX")
kernel_copy="$tmp_dir/PicoOS"
payload="$tmp_dir/boot_contract.bin"
log_pipe="$tmp_dir/out"
cp "$kernel_src" "$kernel_copy"

section_size=$(python3 - "$objdump_bin" "$kernel_copy" <<'PY'
import subprocess
import sys

objdump, elf = sys.argv[1], sys.argv[2]
out = subprocess.check_output([objdump, "-h", elf], text=True, errors="replace")
for line in out.splitlines():
    if ".boot_contract" not in line:
        continue
    parts = line.split()
    # llvm-objdump -h: Idx Name Size VMA ...
    if len(parts) >= 3 and parts[1] == ".boot_contract":
        print(int(parts[2], 16))
        sys.exit(0)
    for i, part in enumerate(parts):
        if part == ".boot_contract" and i + 1 < len(parts):
            print(int(parts[i + 1], 16))
            sys.exit(0)
raise SystemExit("qemu-expect: missing .boot_contract section")
PY
)

if [[ -z "$section_size" || "$section_size" -lt 1 ]]; then
  echo "qemu-expect: .boot_contract size invalid" >&2
  exit 1
fi

python3 -c 'import sys; n=int(sys.argv[1]); b=int(sys.argv[2]); open(sys.argv[3],"wb").write(bytes([b])+bytes(n-1))' \
  "$section_size" "$contract_byte" "$payload"

"$objcopy_bin" --update-section ".boot_contract=$payload" "$kernel_copy"

mkfifo "$log_pipe"

qemu-system-riscv64 \
  -machine virt \
  -nographic \
  -bios none \
  -kernel "$kernel_copy" \
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

  if [[ "$clean_line" == *"[FAIL]"* && "$clean_line" != *"kernel fault -> halt"* ]]; then
    status=1
    break
  fi
done <"$log_pipe"

kill "$watchdog_pid" 2>/dev/null || true
exit "$status"
