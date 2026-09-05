# Project Vision and Rules for AI Agents (PicoOS)

PicoOS is a custom microkernel (Frame Kernel) written in Rust for the RISC-V 64 architecture, developed using a clean-slate (Greenfield) approach and tested inside the QEMU emulator.

---

## Core Vision and Goals

The project is built upon two fundamental pillars:

### 1. Hardened Hardware and Software Security

The PicoOS kernel must remain completely isolated and secure from the boot boundary.

- **Kernel Space Hermeticity:** The kernel code is strictly restricted to thread dispatching, memory protection configurations, and IPC routing.
- **Subsystem Isolation:** Future subsystems—including the file system (FS), device drivers, and the network stack—must not have direct access to kernel memory space. All peripheral management, file systems, and user applications are strictly isolated within User Mode (U-mode) using hardware-level PMP/MMU protection boundaries. If any userspace service fails, the kernel must isolate the fault immediately and maintain continuous uptime.
- **Minimalist Footprint:** The architecture must remain linear, concise, and 100% observable. No redundant abstraction layers, shadow registers, double-buffered contexts in memory, or dead/silent logging paths are permitted.

### 2. Absolute Independence (Clean Slate Design)

The project intentionally rejects legacy computing infrastructure.

- **Rejection of Legacy Standards:** PicoOS explicitly forbids POSIX compatibility, the C standard library (`libc`), the ELF executable format, GNU command-line utilities, Unix-style file paths, and shells executing commands like `sh` and `cd`.
- **Rust/RISC-V Native Ecosystem:** The objective is to redefine the operating system model using only native Rust and bare-metal RISC-V assembly. Third-party crates are permitted only if they natively support `#![no_std]`, introduce zero external C dependencies, and are strictly required to replace highly complex elements (such as a graph-based lightweight Git alternative).

---

## Mandatory Engineering Rules for AI Agents

Any AI agent contributing code to PicoOS must comply with the following operational constraints. Code modifications violating these rules will be rejected.

### Strict Requirements (What You MUST Do)

- **Clippy Enforcement:** The project must compile with zero warnings after any code modification. Running `cargo clippy -- -D warnings` on the `default` target and across all 7 scenario configurations must always yield a clean pass.
- **Unsafe Documentation:** The use of `unsafe` blocks is restricted to the bare-metal architecture layer (`src/arch/`) where direct hardware access (CSR management, MMIO, assembly traps) is unavoidable. Every single `unsafe` block must be explicitly documented with a `// SAFETY:` comment explaining why the operation is safe at the hardware or compiler level. The `deny undocumented_unsafe_blocks` lint is permanently active.
- **Dead Code Eliminating:** Global or local `#[allow(dead_code)]` attributes are heavily discouraged inside `.rs` files. Unused functions, fields, or variables must be completely deleted from the codebase rather than hidden under compiler bypasses. Test helpers must be enclosed in standard `cfg` flags instead of `allow` attributes.
- **Dynamic Runtime Design:** Introducing new compile-time features via `#[cfg(feature = "...")]` inside the core kernel code is prohibited. The project aims to eliminate compile-time execution branches entirely. All operations must execute dynamically at runtime by default. Existing public `scenario_*` selectors in `Cargo.toml` must only be used to select user workers and UART test contracts.

### Forbidden Practices (What You MUST NOT Do)

- **DO NOT** introduce `.c` or `.cpp` source files into the repository.
- **DO NOT** add third-party Rust crates that rely on Foreign Function Interface (FFI) bindings (`extern "C"`) to call underlying C code or external binary libraries.
- **DO NOT** use standard memory allocation mechanisms (`std::alloc`) or any logic that depends on a standard operating system environment.
- **DO NOT** introduce duplicate state tracking variables or redundant copies of CPU frames across context-switching boundaries (always use the unified `TrapImage`).
