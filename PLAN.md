# PicoOS 0.2 — brief plan

Full specification (English, reviewed to 0 open issues):
[`docs/picoos-0.2-frame-kernel.md`](docs/picoos-0.2-frame-kernel.md)

**Thesis:** Contract-Checked Frame Kernel. A frame is stack + trap image + lifecycle + resume contract. Every transfer (boot, trap, yield, sleep, preempt, fault) is a named, UART-observable contract. Not mini-Linux, not xv6.

**Target:** RISC-V 64, QEMU `virt`, `-bios none`. One hart. Milestone ends at 0.2; Sv39/S-mode is 0.3.

## Current tree (audit)

Hobby kernel: edition 2021, 35 Cargo features, 204 `#[allow]`, 144 `unsafe`, 20 `static mut`. Default boot arms a 1 Hz timer and **halts after 5 ticks**. Real dispatch/restore live behind features. Allocators never free. Everything runs in M-mode.

Keep: `src/arch`, `src/drivers`, `src/platform`, `src/kernel/task`, trap stack, lifecycle FSM, resume checks, QEMU UART scripts as ABI.

## Phases

| Phase | PRs | Outcome |
| --- | --- | --- |
| 0 Hygiene | 0–3 | rustc 1.97.1, edition **2024**, Clippy **`-D warnings`** (not pedantic/nursery as deny), dead code gone, English comments |
| 1 Cleanup | 4–9 | `MAX_TASKS = 8`, split arch files, `Cpu`, nestable `without_interrupts`, no production `static mut`, panic + UART LSR |
| 2 Correct base | 10–20 | bitmap free, reap, always-on scheduler, yield `s0–s11`, default idle+workers, preemption via `mret`, then **U-mode + PMP + `ecall`** |
| 3 Freeze 0.2 | 21–23 | collapse features, full `check-all.sh` matrix, banner `0.2.0` matching the binary |

This is an **ordered series**, not 24 independently mergeable PRs. After every PR: `scripts/check-all.sh`. Parallel: `{PR8, PR9}` after PR1; `{PR4, PR5}` after PR3; PR14 and PR18 can start before PR13/PR15.

## Isolation (do not fake)

- **0.2:** M-mode kernel, U-mode frames, unlocked PMP, four `ecall`s (`yield` / `sleep` / `exit` / `log`). Kernel prints UART markers.
- PMP: TOR chain first (pmp0 bound/deny `[0, __text_start)`, pmp1 `.text` **RX**, pmp2 `.rodata` R), then NAPOT current stack. Miss = deny for `.data`/UART/CLINT.
- `ecall` resume: **`mepc += 4`**. Timer does not. Handler `mstatus`: `MIE=0`, `MPIE=1`. PR16 `MPP=M`; PR19 `MPP=U`.
- Idle is M-mode `wfi`, not a `Task`. Idle-exit resets `mscratch`, no `mret`.
- Banner may claim PMP only after a provoked U-mode store faults.

## PR series

0. Pin rustc 1.97.1  
1. Edition 2024  
2. `clippy.toml`; CI stays `-D warnings`  
3. Dead code; English comments; drop unused allows  
4. Index table by id; `MAX_TASKS = 8`  
5. Split `yield.rs` / `restore.rs`  
6. `Cpu` replaces debug globals  
7. Nestable IRQ mask; `UnsafeCell`; `static mut` → 0  
8. Panic logs `PanicInfo` *(parallel)*  
9. UART LSR poll *(parallel)*  
10. Bitmap page allocator with free  
11. Remove bump heap  
12. Reap Finished/Faulted → Empty; leak marker  
13. Always compile dispatch  
14. Yield saves `s0–s11` (interim M-mode)  
15. Default image: idle + `worker_yield` + `worker_sleep` (no 5-tick halt)  
16. Preemption via trap frame + `mret` (`MPP=M`); add timer script to `check-all.sh`  
17. Sleep wake fixtures (no-image stays `can_resume=false`)  
18. PMP dump + split trap stack from `.bss`  
19. U-mode + `u_sys_*` + PMP deny + store-fault test  
20. Fault classify by privilege/region  
21. Feature collapse → scenario selectors  
22. Full QEMU matrix; no `cargo clean`  
23. Banner PicoOS 0.2.0  

## Done when

Default `cargo build` on QEMU virt: always-on scheduler, U-mode workers, real preemption, sleep/wake, task fault ≠ kernel halt, `mm leak check: OK`, no halt at tick 5, existing marker scripts green.

## Deferred (0.3+)

Sv39, S-mode, OpenSBI vs `-bios none`, `.text.user`, SMP, VFS, net, POSIX.

## Open (not blocking 0.2)

- OpenSBI in 0.3 vs staying `-bios none`.
- Reap from scheduler (recommended) vs idle vs ISR.
