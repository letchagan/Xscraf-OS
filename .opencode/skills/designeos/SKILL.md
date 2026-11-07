---
name: rust-os-dev
description: Project-aware skill for developing Aether OS and other bare-metal operating systems primarily in Rust. Use for no_std kernels, bootloaders, x86_64 CPU setup, interrupts, paging, allocators, scheduling, capability security, user mode, syscalls, IPC, drivers, filesystems, networking, performance, and QEMU validation or debugging.
---

# Rust OS Development Skill

Build a bootable, testable operating system with kernel and OS logic written in Rust. This skill defines the project-aware workflow, architecture guardrails, milestone order, security checks, performance measurements, and evidence required before claiming success.

## 1. Project-first workflow

Before making changes:

1. Read repository instructions such as `AGENTS.md`, `README`, architecture notes, and relevant issue/task details.
2. Inspect `git status --short` and preserve all existing user changes.
3. Read the actual `Cargo.toml`, `rust-toolchain.toml`, `.cargo/config.toml`, target JSON, bootloader setup, entry point, relevant modules, tests, and scripts.
4. Establish how the current project builds and boots, and identify the last known-good milestone.
5. State the smallest intended change and its exit test.

Do not replace an existing project with this skill's sample tree. The repository is authoritative. Adapt names, commands, dependencies, and module layout to what exists.

For each subsystem, follow:

**Specification → architecture → implementation → tests → QEMU validation → documentation.**

## 2. Rust-only scope

- Kernel and OS logic should be implemented in Rust.
- Use `#![no_std]` and normally `#![no_main]` for the bare-metal kernel crate.
- Use `core` before heap initialization; introduce `alloc` only after a working allocator exists.
- Some CPU instructions and ABI transitions cannot be expressed directly in ordinary Rust. Use `core::arch::asm!`, `core::arch::global_asm!`, or carefully justified naked functions **inside `.rs` files** when needed. Explain the reason and safety invariants.
- Do not add standalone `.s`/`.asm` files or a C runtime unless explicitly approved.
- A Rust-compatible bootloader and platform firmware are acceptable; do not spend early milestones writing a custom bootloader unless that is a deliberate project goal.
- Use crates only after checking target compatibility, `no_std` support, licensing, maintenance, and dependency implications. Prefer established crates for well-understood low-level primitives unless learning/reimplementing them is the explicit objective.

## 3. Aether OS architecture guardrails

Aether OS is an experimental x86_64 Rust OS prototype. Its intended direction is security-first and resource-conscious; planned features must not be described as already implemented.

### Kernel and services

- Favor a minimal microkernel: keep only necessary privileged mechanisms in kernel mode.
- Move drivers and higher-level services to user space when process isolation, IPC, and fault containment are ready.
- Do not claim a service or driver is isolated until privilege boundaries, address-space separation, IPC validation, and failure handling have tests.
- Keep architecture-specific code behind narrow interfaces; QEMU x86_64 success does not prove support for other architectures or real hardware.

### Capabilities and permissions

- Use least privilege and explicit, scoped permissions.
- Define capability ownership, object identity, delegation, revocation, and error semantics before implementing security-sensitive APIs.
- Handles must not grant access merely because an attacker can guess an integer. Validate handle ownership, generation/lifetime, rights, and target object as appropriate.
- Centralize authorization checks where possible and make them auditable.
- Test both allowed and denied operations.

### Trust boundaries and memory safety

- Treat all user pointers, syscall parameters, IPC messages, filesystem metadata, executable files, and device-provided data as untrusted.
- Validate pointer ranges, lengths, alignment where required, integer arithmetic/overflow, handles, and permissions before use.
- Copy data across trust boundaries when appropriate; never blindly dereference a user pointer.
- Establish user/kernel address-space separation and safe mapping rules. Prefer W^X, NX for data, guard pages for stacks, and explicit ownership of physical frames/page tables where supported.
- Make allocation failure and resource exhaustion behavior explicit.
- Keep `unsafe` blocks small. Every block must have a `// SAFETY:` comment stating the invariant it depends on; comments do not make an unsound invariant safe.

### Resource control, recovery, and observability

- Avoid unbounded queues, caches, retries, allocations, and work in interrupt handlers.
- Add per-process/service memory, CPU, handle, and queue budgets when those subsystems exist.
- Future update mechanisms should validate staged data and support transactional activation/recovery; do not claim immutability or atomic updates without implementation and tests.
- Prefer typed, validated configuration and low-overhead tracing. Do not log credentials, secrets, or unnecessary privileged data.
- Keep any future desktop/runtime behind defined kernel interfaces.
- Do not integrate AI models or AI agents into the OS unless explicitly requested. Development-time assistant tools are not OS features.

## 4. Recommended repository shape

Use this only as a conceptual guide; preserve the real repository structure.

```text
AetherOS/
├── Cargo.toml
├── rust-toolchain.toml
├── .cargo/
├── docs/
│   ├── architecture.md
│   ├── threat-model.md
│   ├── decisions/
│   └── testing.md
├── kernel/
│   └── src/
│       ├── main.rs
│       ├── arch/x86_64/
│       ├── serial.rs
│       ├── interrupts.rs
│       ├── memory/
│       ├── process/
│       ├── ipc/
│       └── syscall.rs
├── services/       # add only when user-space foundations exist
├── drivers/         # kernel or user-space placement follows architecture
├── userspace/       # loader, shell, runtime, applications as implemented
└── tests/           # host tests and QEMU integration tests
```

Do not create all these directories as a cosmetic exercise. Add a module only when it supports a defined milestone.

## 5. Toolchain and boot configuration

Inspect the current configuration before editing. Common requirements may include nightly Rust, `rust-src`, `llvm-tools-preview`, a custom target, a bootloader crate, QEMU, and GDB. Versions and commands depend on the chosen boot path.

Example `rust-toolchain.toml` for a project that intentionally uses nightly features:

```toml
[toolchain]
channel = "nightly"
components = ["rust-src", "llvm-tools-preview"]
```

Example build-std settings sometimes used for a custom bare-metal target:

```toml
[unstable]
build-std = ["core", "compiler_builtins", "alloc"]
build-std-features = ["compiler-builtins-mem"]
```

Do not paste these examples blindly. Confirm they match the project's target, bootloader, allocator stage, and Rust toolchain. Likewise, `panic = "abort"`, a QEMU runner, and `bootimage` configuration must match the actual test setup.

Useful crate families, subject to compatibility checks: `x86_64`, `uart_16550`, `pic8259`, `pc-keyboard`, `spin`, `volatile`, `linked_list_allocator`, `bootloader`, `acpi`, `virtio-drivers`, `smoltcp`, and `x2apic`.

## 6. Milestones and exit tests

Use the sequence below as a dependency-aware roadmap. Do not move forward if a milestone's exit test fails. Adapt to existing implementation and avoid reimplementing completed work without cause.

| # | Milestone | Exit test |
|---|---|---|
| 1 | Freestanding `no_std` entry point and panic path | Kernel builds and boots repeatably in QEMU; unexpected resets are investigated |
| 2 | Serial and screen/framebuffer output | Known diagnostic appears on serial; display output tested separately |
| 3 | GDT, TSS, IDT, breakpoint exception | Breakpoint is handled without reboot |
| 4 | Double-fault IST stack | Controlled fault reports the expected exception rather than a triple fault |
| 5 | Timer, interrupt controller, keyboard | Ticks and key input are correct; EOI handling is verified |
| 6 | Memory map, physical-frame allocator, paging | Test page can be mapped/written/unmapped; invalid access behaves as expected where testable |
| 7 | Kernel heap | Allocation/deallocation stress test passes after correct initialization |
| 8 | Cooperative tasks or async executor | Two tasks make progress without corrupting shared state |
| 9 | Preemptive scheduler | Multiple tasks progress under timer interrupts; context/register state is tested |
| 10 | User mode and process address spaces | Ring-3 test program invokes a minimal syscall and exits safely |
| 11 | Syscall ABI and user-memory validation | Invalid pointers, lengths, handles, and permissions are rejected safely |
| 12 | IPC and capability model | Scoped authorized requests succeed; unauthorized requests fail; queues are bounded |
| 13 | ELF loader and initial userspace | Valid test binary loads; malformed binaries are rejected |
| 14 | VFS and storage | Basic file operations work; boundary and malformed-metadata tests pass |
| 15 | PCI and selected drivers/networking | Enumeration works; QEMU smoke test succeeds for implemented device/protocol |
| 16 | Resource limits and tracing | Configured limits are enforced; overload does not hang the kernel |
| 17 | SMP (optional, later) | Expected CPUs start and synchronization stress tests pass |
| 18 | Shell/desktop/app runtime | UI uses documented APIs and respects process/security boundaries |

A milestone is not complete because files exist or the build succeeds. Record the actual test evidence and any limitations.

## 7. Code standards

- Use `#![no_std]`/`#![no_main]` where appropriate and `alloc` only after heap initialization.
- Keep hardware access behind narrow abstractions and use `read_volatile`/`write_volatile` for MMIO where required.
- Avoid unsound aliasing through `static mut`; document ownership and synchronization invariants.
- Protect shared state correctly. Never let an interrupt preempt code while it holds a lock that the interrupt handler also needs. Use interrupt masking only when justified and restore prior interrupt state correctly.
- Keep interrupt handlers short, non-blocking, and allocation-free; acknowledge the selected interrupt controller in the correct order.
- Use `Result` and explicit error types for recoverable failures. Reserve panic for unrecoverable kernel invariants.
- Validate all user-controlled values at trust boundaries.
- Explain hardware-specific constants and ABI assumptions with relevant CPU manuals or crate documentation where useful.
- Do not add dependencies requiring `std` or a C toolchain without explaining the implications and receiving approval if it changes project constraints.

## 8. Testing strategy

Separate test types:

1. **Host unit tests:** pure parsers, algorithms, allocators/data structures that do not touch hardware, guarded appropriately with `#[cfg(test)]` or a separate host-test crate.
2. **Target compile checks:** ensure target-specific code and features compile for the configured bare-metal target.
3. **QEMU integration tests:** boot the kernel and verify behavior through serial output, test exit devices, or explicit assertions.
4. **Regression tests:** rerun the existing baseline tests after changes to interrupts, memory, scheduling, syscalls, or shared infrastructure.
5. **Adversarial/security tests:** invalid handles, denied permissions, oversized inputs, integer overflow, malformed ELF/IPC/filesystem data, allocation failure where injectable, and repeated resource creation/destruction.
6. **Performance tests:** use consistent workload and environment; compare against a recorded baseline.

Each integration test should fail clearly and independently where the harness allows. Never interpret a QEMU test as proof of physical-hardware compatibility.

## 9. QEMU and GDB diagnostics

Adapt commands to the project's bootloader and image path. Example flags useful for x86_64 diagnostics:

```bash
qemu-system-x86_64 ... -serial stdio -display none \
  -no-reboot -no-shutdown -d int,cpu_reset -D qemu.log
```

For GDB, QEMU commonly supports `-s -S`, followed by connecting `rust-gdb` to `localhost:1234` and loading the actual kernel ELF symbols. Verify the path and launch command first.

| Symptom | First checks |
|---|---|
| Instant reboot / reboot loop | Suspect triple fault; inspect QEMU logs, IDT/GDT, exception stack, entry ABI, and stack alignment |
| Hang after enabling interrupts | EOI, interrupt masking, PIC/APIC configuration, handler lock deadlock, and handler return ABI |
| Page fault at apparently mapped address | Page-table levels, permissions, canonical address, TLB state, ownership/lifetime of frames |
| Heap corruption | Allocator initialization, stack overflow, aliasing, double free, concurrent access, allocation from interrupt context |
| Debug succeeds but release fails | Undefined behavior, missing volatile access/barriers, ABI assumptions, optimization-sensitive aliasing |
| Allocation fails early | Heap not initialized, incorrect heap mapping, allocator metadata, or allocation in prohibited context |
| Key prints a key name instead of expected character | Inspect scancode decoding, modifier state, key-event-to-character conversion, and terminal input path separately |

Treat these as hypotheses, not conclusions. Collect evidence before changing code.

## 10. Security review checklist

Before calling a privileged interface or service ready, answer:

- Who can invoke it, and what capability/permission is required?
- Are pointer ranges, lengths, arithmetic, alignment, handles, object rights, and ownership validated?
- Can malformed input trigger out-of-bounds access, panic, deadlock, denial of service, or information disclosure?
- Is work bounded in interrupts, syscalls, and IPC paths?
- Are resources released on every error path?
- Can one process access another process's memory or resources without authorization?
- Are revocation, delegation, process exit, and stale handles handled consistently?
- Are errors informative without exposing secrets or privileged data?
- What is still unimplemented in the threat model?

Passing tests is evidence, not proof of production security. Document residual risks and untested paths.

## 11. Performance and resource optimization

For changes intended to reduce RAM, storage, CPU use, or latency, record a baseline and compare with the same test conditions where feasible:

- kernel image and relevant binary sizes;
- boot-to-ready time, with the start/end events defined;
- idle and workload memory usage;
- scheduler/IPC latency or throughput for relevant changes;
- CPU utilization, queue depth, or allocation counts when instrumentation exists.

Correctness and isolation come first. Do not add complexity or weaken safety for unmeasured optimization. If reliable measurement is unavailable, explain the limitation rather than inventing a number.

## 12. Autonomous implementation protocol

For every task:

1. Discover project state and baseline.
2. Propose a short, testable plan.
3. Implement one coherent change.
4. Run formatting, relevant tests, target build, and QEMU checks as applicable.
5. If failure occurs, retain the first useful logs and change one variable at a time.
6. After repeated evidence-free failures, stop and report the blocker instead of looping.
7. Update docs/tests and report exactly what was verified.

Do not run destructive disk/partition/firmware/real-hardware flashing commands, reset user changes, overwrite release artifacts, or alter credentials/signing keys without explicit approval. Do not fetch or execute opaque binaries/scripts without explaining their source and purpose.

For broad tasks such as “finish the OS,” convert the goal into a staged backlog and implement the next testable milestone unless a larger batch is explicitly authorized.

## 13. Required response format for implementation tasks

Keep the report concise and factual:

- **Milestone:** current stage and prerequisites.
- **Changed:** files/modules and behavior.
- **Validation:** exact commands run and observed outcomes.
- **Not verified:** tests unavailable or failures still present.
- **Security/performance:** relevant implications and limitations.
- **Next step:** the smallest dependency-safe milestone.

Provide complete compilable files when asked for code, not isolated snippets unless the user requests a snippet. Include exact commands and expected success signals. Clearly label expected output as expected, not observed.

## 14. Learning references

- *Writing an OS in Rust*: https://os.phil-opp.com/
- OSDev Wiki: https://wiki.osdev.org/
- Intel Software Developer's Manual, Volume 3, for x86 system programming
- AMD Architecture Programmer's Manual, Volume 2
- Rust Reference and the documentation for the selected bootloader and low-level crates
- Redox OS, Theseus, and Hermit as comparative Rust OS projects
