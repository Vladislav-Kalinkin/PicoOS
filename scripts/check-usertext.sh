#!/usr/bin/env bash
set -euo pipefail

ELF=${1:-target/riscv64gc-unknown-none-elf/debug/PicoOS}

if [[ ! -f "$ELF" ]]; then
  echo "check-usertext: missing $ELF" >&2
  exit 1
fi

objdump_bin=""
for cand in llvm-objdump /opt/homebrew/opt/llvm/bin/llvm-objdump /usr/bin/llvm-objdump; do
  if command -v "$cand" >/dev/null 2>&1 || [[ -x "$cand" ]]; then
    objdump_bin=$cand
    break
  fi
done
if [[ -z "$objdump_bin" ]] && command -v rust-objdump >/dev/null 2>&1; then
  objdump_bin=rust-objdump
fi

if [[ -z "$objdump_bin" ]]; then
  echo "check-usertext: llvm-objdump not found" >&2
  exit 1
fi

python3 - "$ELF" "$objdump_bin" <<'PY'
import subprocess
import sys

elf, objdump = sys.argv[1], sys.argv[2]
out = subprocess.check_output([objdump, "-t", elf], text=True, errors="replace")
syms = {}
for line in out.splitlines():
    parts = line.split()
    if len(parts) < 2:
        continue
    try:
        addr = int(parts[0], 16)
    except ValueError:
        continue
    name = parts[-1]
    if name.startswith("0x") or name.startswith("."):
        continue
    syms[name] = addr

def need(name):
    if name not in syms:
        raise SystemExit(f"check-usertext: missing symbol {name}")
    return syms[name]

user_start = need("__user_text_start")
user_end = need("__user_text_end")
text_end = need("__text_end")
rodata_start = need("__rodata_start")
tramp = need("user_trampoline")

if not (user_start < tramp < user_end):
    raise SystemExit(
        f"check-usertext: user_trampoline {tramp:#x} not in "
        f"[{user_start:#x}, {user_end:#x})"
    )
if tramp < text_end:
    raise SystemExit(
        f"check-usertext: user_trampoline {tramp:#x} < __text_end {text_end:#x}"
    )
if user_end > rodata_start:
    raise SystemExit(
        f"check-usertext: __user_text_end {user_end:#x} > "
        f"__rodata_start {rodata_start:#x}"
    )

user_syms = [
    "u_sys_yield",
    "u_sys_sleep",
    "u_sys_exit",
    "u_sys_log",
    "u_sys_spawn",
    "u_sys_join",
    "u_sys_gettid",
    "u_sys_send",
    "u_sys_recv",
    "worker_yield_main",
    "worker_sleep_main",
    "worker_pmp_deny",
    "worker_two_yield",
    "worker_handoff_a",
    "worker_handoff_b",
    "worker_clean_exit",
    "worker_ebreak",
    "worker_sleep_e2e",
    "worker_kernel_fetch",
    "worker_spawn_main",
    "child_exit",
    "worker_ipc_recv",
    "worker_ipc_send",
    "worker_ipc_parent",
]
for name in user_syms:
    addr = need(name)
    if not (user_start <= addr < user_end):
        raise SystemExit(
            f"check-usertext: {name} {addr:#x} not in usertext "
            f"[{user_start:#x}, {user_end:#x})"
        )

kernel_syms = [
    "kernel_fetch_probe_target",
    "riscv64_trap_handler",
]
for name in kernel_syms:
    addr = need(name)
    if addr >= text_end:
        raise SystemExit(
            f"check-usertext: {name} {addr:#x} not in kernel .text "
            f"(end {text_end:#x})"
        )

print("usertext symbol contract: OK")
print(f"  usertext: {user_start:#x} - {user_end:#x}")
print(f"  user_trampoline: {tramp:#x}")
PY
