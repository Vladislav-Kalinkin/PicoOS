# PicoOS 0.3 — brief plan

Full specification (English, reviewed to 0 open issues):
[`docs/picoos-0.3-frame-kernel.md`](docs/picoos-0.3-frame-kernel.md)

Prior milestone (shipped 0.2.0):
[`docs/picoos-0.2-frame-kernel.md`](docs/picoos-0.2-frame-kernel.md)

**Thesis:** Complete the Frame Kernel identity on one hart: dispatch + PMP X-range + copy-IPC routing. A thread is a Frame. Isolation in 0.3 is unlocked PMP (U-X only `.usertext`) + ResumeContract, not Sv39.

**Target:** RISC-V 64, QEMU `virt`, `-bios none`. One hart. M-mode kernel, U-mode frames. Milestone ends at 0.3; Sv39/S-mode is 0.4.

## From 0.2 (already shipped)

M-mode kernel, U-mode frames, unlocked PMP, four `ecall`s, always-on scheduler, `mret` preemption, bitmap + reap, `MAX_TASKS = 8`, seven `scenario_*` binaries. Leftover: U can `jal` handler bytes because `.text` is RX.

## 0.3 outcome

- `.usertext` + PMP: U execute only there; kernel `.text` fetch-deny is a task fault.
- Spawn / join / gettid. Always-zombie until join. One joiner.
- Copy-IPC rendezvous (32 bytes). Not on the default image.
- One `cargo build` + `.boot_contract` byte (objcopy). UART stays TX-only.
- No VFS. No SMP. No Sv39.

Default image (contract byte 0): idle + yield + sleep + pmp-deny + spawn/join child.

## Phases

| Phase | PRs | Outcome |
| --- | --- | --- |
| 1 Always-on verbs | 1–2 | all U stubs/workers compile; `BlockReason` |
| 2 Isolation + threads | 3–5 | `.usertext` PMP, asm trampoline, spawn/join |
| 3 IPC + timer | 6–7 | copy-IPC (code); 100 Hz quiet timer |
| 4 One binary | 8–10 | unify `kernel_main`, objcopy contract, delete `scenario_*` |
| 5 Freeze 0.3 | 11 | banner `0.3.0` matching tested capabilities |

Ordered series. After every PR: `scripts/check-all.sh`. Parallel: `{PR1, PR2, PR7}`; PR8 after PR1 (can overlap 3–7).

## Isolation (do not fake)

- U-X only `.usertext` (not `.text.user` — live `*(.text*)` would swallow that name).
- pmp1 TOR end = `__user_text_start` (align gap is deny). Stack NAPOT is pmp4 (`pmpcfg0` bit 32).
- ResumeContract: `mepc` in user text, `sp` in own stack.
- Product limits that stay in 0.3: one identity map, one NAPOT stack, user-text gadgets, no IPC caps.

## PR series

1. Always compile U stubs and workers  
2. `BlockReason` (parallel with 1)  
3. `.usertext` linker + PMP X-range + ResumeContract  
4. Asm trampoline; delete `transmute`  
5. `sys_spawn` / `sys_join` / `sys_gettid`  
6. Copy-IPC (code only; QEMU gate in PR9)  
7. Quiet 100 Hz (parallel after PR1)  
8. Unify `kernel_main`  
9. `.boot_contract` byte; one build for all QEMU tests  
10. Delete `scenario_*`  
11. Banner PicoOS 0.3.0  

## Done when

Single `cargo build` on QEMU virt: `.usertext` fetch-deny, spawn+join (child may exit first), copy-IPC contract, no `scenario_*`, `check-all.sh` green, banner `0.3.0`.

## Deferred (0.4+)

Sv39, S-mode, OpenSBI vs homegrown M-trampoline (`-bios none` recommended), SMP, VFS (userspace-only if/when; kernel pages never files), net, POSIX.

## Open (not blocking 0.3)

- 0.4 bootstrap: homegrown M-trampoline vs OpenSBI (default: homegrown, `-bios none`).
- IPC payload 32 vs 8 (default: 32).
- Raise `MAX_TASKS` to 16 (default: stay 8).
