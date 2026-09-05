# PicoOS 0.1.4 (0.4) — brief plan

Full specification (English, reviewed to 0 open issues):
[`docs/picoos-0.1.4-frame-kernel.md`](docs/picoos-0.1.4-frame-kernel.md)

Prior milestone (shipped 0.3.0):
[`docs/picoos-0.3-frame-kernel.md`](docs/picoos-0.3-frame-kernel.md)

**Thesis:** Honest extra RISC-V harts + Frame-native Sv39 + PMP as last belt + no compile-time `MAX_TASKS`. A thread is a Frame with a home hart and a page table. Isolation in 0.1.4 is Sv39 user windows (no kernel in U, no identity map); unlocked PMP is the physical belt if SATP is hostile. No kernel spinlock.

**Target:** RISC-V 64, QEMU `virt`, `-bios none`. M-mode kernel, U-mode frames. Banner at freeze: `PicoOS 0.1.4` (not semver; informal alias **0.4**).

## From 0.3 (already shipped)

M-mode, `.usertext` PMP X-range, spawn/join zombies, copy-IPC 32 B, one always-on binary, `.boot_contract` byte, `MAX_TASKS = 8`, one hart, identity map, `IrqCell`. Leftovers: no SATP, compile-time table, one trap stack, UART not SMP-safe.

## 0.1.4 outcome

- **SMP without mutex:** `HartLocal`, partitioned bitmap, packed `TaskId` (generation starts at 1; id 0 reserved). Cross-hart IPC is SPSC + IPI; block the Frame, never spin in M.
- **Sv39 for U only** (`satp` from M, keep `mtvec`/`mret`). Three windows in one L0: UTEXT `0x100000`, URODATA at LMA delta, USTACK `0x180000`. No kernel in U, no DRAM identity.
- **PMP last belt:** pmp5 NAPOT RW current stack, pmp6 NAPOT R 128 MiB DRAM. Hostile identity PTE still faults.
- **`MAX_TASKS` deleted.** Page-backed per-hart table, cap `min(local_free/8, 512)`.
- Stay `-bios none`. No OpenSBI. No S-mode kernel (PMP cannot split S vs U).

Default image stays on `-smp 1`. New contracts: hart-park, Sv39 windows, PMP belt, `smp two harts`, `smp ipc`.

## Phases

| Phase | PRs | Outcome |
| --- | --- | --- |
| 1 Hart topology | 1–3 | park, `HartLocal`, per-hart CLINT/`mscratch`, hart-0 ticks |
| 2 Runtime table | 4–5 | partitioned bitmap; page-backed table; no `MAX_TASKS` |
| 3 Isolation | 6–8 | `.userrodata`, Sv39 windows, PMP belt |
| 4 Two harts | 9–11 | `-smp 2` share-nothing, UART ring, cross-hart IPC |
| 5 Freeze 0.1.4 | 12 | banner matching tested capabilities |

Ordered series. After every PR: `scripts/check-all.sh`. Parallel spines: `{PR3, PR4}` after PR2; PR6 after PR3 (overlap PR5).

## Isolation (do not fake)

- U tables: UTEXT / URODATA / USTACK only. No kernel `.text` in U. No identity DRAM in U.
- PMP still on: walker can **read** pool PTEs; U cannot **write** other stacks/PTs.
- Product limits that stay in 0.1.4: shared physical `.usertext` RX in every frame, U may read the pool, UART panic-path THR poll, `HART_SLOTS = 8`, ASID=0, no migration.

## PR series

1. Park secondaries; `HART_SLOTS`; park smoke on `-smp 2`
2. `HartLocal`; delete `IrqCell` / duplicate current-id
3. Per-hart CLINT; `trap.S` `HART_TRAP_TOP`; hart-0 ticks; IPI stub
4. Partition page bitmap into `HART_SLOTS` ranges
5. Page-backed table; packed `TaskId`; delete `MAX_TASKS`
6. `.userrodata` + pmp5/pmp6 NAPOT (no SATP yet)
7. Sv39 walker, per-frame tables, `satp` on `mret`
8. PMP last-belt contract (hostile identity PTE)
9. Bring up hart 1 + UART log ring
10. Cross-hart SPSC IPC + remote join
11. Panic-path UART exception documented
12. Banner PicoOS 0.1.4

## Done when

QEMU virt `-bios none`: Sv39 unmapped kernel store faults the task, PMP belt holds a hostile identity PTE, two harts in U concurrently, cross-hart copy-IPC, no `MAX_TASKS`, `check-all.sh` green, banner `0.1.4`.

## Deferred (after 0.1.4)

S-mode kernel, OpenSBI, ASID, frame migration, VFS (userspace-only if/when; kernel pages never files), net, POSIX, ELF, `-smp 4` in `check-all`.

## Open (defaults bind)

- S-mode kernel in this series? Default: **no** (stay M + Sv39 U).
- ASID? Default: **no**.
- Frame migration? Default: **no**.
- `-smp 4` in `check-all.sh`? Default: **no** (extra script only).
