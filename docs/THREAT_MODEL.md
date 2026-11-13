# Xscraf-OS Threat Model

## System Overview

**Xscraf-OS** is a Rust-based, **capability-oriented microkernel** for x86_64 with a distinctive **task-based execution model**. Unlike traditional Unix/Linux (fork/exec) or Windows (process/thread), Xscraf-OS uses:

- **Lightweight tasks** (not heavy processes) with sealed address spaces
- **Priority-budgets**: Each task has a CPU time quota + priority level
- **Hierarchical task trees**: Parent/child relationships with capability delegation
- **Message-passing IPC**: No shared memory; all inter-task communication via kernel-copied messages
- **Mandatory access control**: Capabilities are the only way to grant permissions

This threat model covers Stages 3–4+ (user mode, paging, scheduling) — not the current Stage 2 which runs in Ring 0.

---

## Xscraf-OS Execution Model (Unique Design)

### Task vs Process
Unlike Unix processes or Windows processes, Kairos-11 **tasks** are:
- **Lightweight**: Minimal memory overhead (just address space + capability set)
- **Sealed**: Complete memory isolation; no cross-task shared memory
- **Hierarchical**: Parent tasks create and control child tasks
- **Budget-aware**: Each task has CPU time allocation + priority
- **Capability-based**: Only via explicit capability grants

### Example Task Tree
```
                    [Init Task]
                   (parent cap)
                   /          \
        [App A]              [App B]
      (cpuset)            (cpuset + message cap)
       /     \                  \
    [A1]   [A2]               [B1]
 (message)                  (read cap)
```

Task A1 can **only**:
- Read/write own memory
- Send messages to its parent (App A)
- Call capabilities explicitly granted by parent

### Priority Budget Scheduling
Each task has:
- **Priority**: 0–255 (higher = more CPU time, but not exclusive)
- **Budget**: N milliseconds of CPU time per scheduling round
- **Fair queuing**: All same-priority tasks get equal time, then lower-priority

**Difference from traditional priority**: Tasks never starve completely (minimum fairness).

---

## Assets (What We Protect)

| Asset | Description | Sensitivity |
|-------|-------------|-------------|
| **Kernel memory** | Code, data, page tables, interrupt tables | Critical — compromise = full system control |
| **User process memory** | Heap, stack, code, capabilities | High — per-process isolation |
| **Capability table** | Kernel-managed object references | Critical — forge = privilege escalation |
| **Device registers** | MMIO/port I/O for hardware | High — DMA, firmware persistence |
| **Filesystem data** | User files, metadata, keys | High — confidentiality/integrity |
| **Boot integrity** | Kernel image, bootloader config | Critical — persistent compromise |

---

## Trust Boundaries

```
┌─────────────────────────────────────────────────────────────┐
│                        HARDWARE                              │
│  CPU (Ring 0/3)  │  MMU  │  PIC/APIC  │  Devices (DMA)     │
└─────────────────────────────────────────────────────────────┘
                              │
        ┌─────────────────────┼─────────────────────┐
        ▼                     ▼                     ▼
┌───────────────┐    ┌───────────────┐    ┌───────────────┐
│  KERNEL       │    │  KERNEL       │    │  KERNEL       │
│  (Ring 0)     │    │  SERVICES     │    │  DRIVERS      │
│  - Scheduler  │    │  (Ring 0)     │    │  (Ring 0)     │
│  - VMM        │    │  - Cap Mgr    │    │  - VirtIO     │
│  - IPC        │    │  - VFS        │    │  - AHCI       │
│  - Syscalls   │    │  - Proc Mgr   │    │  - USB HID    │
└───────┬───────┘    └───────┬───────┘    └───────┬───────┘
        │                    │                    │
        └────────────────────┼────────────────────┘
                             │  CAPABILITY INVOCATION (syscall)
                             ▼
┌─────────────────────────────────────────────────────────────┐
│  USER SPACE (Ring 3)                                        │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐        │
│  │ Process │  │ Process │  │ Process │  │ Process │  ...   │
│  │   A     │  │   B     │  │   C     │  │   D     │        │
│  └────┬────┘  └────┬────┘  └────┬────┘  └────┬────┘        │
│       │            │            │            │              │
│       └────────────┴────────────┴────────────┘              │
│                    CAPABILITY REFERENCES                      │
└─────────────────────────────────────────────────────────────┘
```

### Boundary Properties

| Boundary | Enforcement Mechanism | Current Status |
|----------|----------------------|----------------|
| **Ring 0 ↔ Ring 3** | x86_64 paging (U/S bit) + `syscall`/`sysret` | ❌ Not implemented |
| **Process ↔ Process** | Separate page tables + capability checks | ❌ Not implemented |
| **Kernel ↔ Device** | IOMMU (future) / port I/O restriction | ❌ Not implemented |
| **Capability ↔ Object** | Kernel-managed capability table (CNode) | ❌ Not implemented |

---

## Attack Surfaces

### 1. Keyboard Input (Current Stage 2)
- **Vector**: PS/2 scancodes → interrupt handler → queue → main loop
- **Risk**: Buffer overflow (mitigated: fixed 256-entry ring buffer), malformed scancode sequences
- **Mitigation**: Bounded queue, no allocation in IRQ, `pc_keyboard` validates sequences

### 2. Serial Port (COM1)
- **Vector**: UART 16550 → interrupt (polled) → `serial_println`
- **Risk**: Format string (mitigated: `write_fmt` with static format), buffer exhaustion
- **Mitigation**: No user input processed; output-only

### 3. VGA Text Buffer
- **Vector**: `print!`/`println!` → `Writer` → 0xB8000
- **Risk**: Cursor position manipulation, buffer overflow (mitigated: bounds-checked)
- **Mitigation**: `volatile` writes, column/row bounds enforced

### 4. Future: Syscall Interface
- **Vector**: `syscall` instruction → IDT gate → kernel entry
- **Risk**: Invalid user pointers, integer overflow in args, capability forgery
- **Planned Mitigation**: 
  - Validate all user pointers (`UserPtr<T>` wrapper)
  - Bounded syscall argument structs
  - Capability validation on every invocation

### 5. Future: Device Drivers (VirtIO, AHCI, USB)
- **Vector**: MMIO/port I/O → DMA → physical memory
- **Risk**: DMA to arbitrary physical pages, device firmware exploits
- **Planned Mitigation**:
  - IOMMU (VT-d/AMD-Vi) for DMA isolation
  - Drivers in user space with capability-granted device access
  - Bounded command queues, validated descriptors

### 6. Future: Filesystem
- **Vector**: Disk images → block driver → VFS → user processes
- **Risk**: Path traversal, metadata corruption, symlink races, TOCTOU
- **Planned Mitigation**:
  - Capability-based open (no string paths in syscalls)
  - Bounded path components, validated UTF-8
  - Copy-on-write / journaling for metadata

---

## Threat Actors

| Actor | Capability | Motivation |
|-------|------------|------------|
| **Local unprivileged user** | Ring 3 code execution, syscalls | Privilege escalation, data theft |
| **Compromised user process** | Own memory + capabilities | Lateral movement, kernel exploit |
| **Malicious device** | DMA, firmware, interrupts | Kernel memory read/write, persistence |
| **Supply chain** | Modified kernel image, bootloader | Persistent rootkit |
| **Physical access** | USB, PCIe, JTAG | Firmware implant, cold boot |

---

## Security Objectives (Priority Order)

### P0 — Critical (Must Have Before User Mode)
1. **Memory isolation**: No user process can read/write kernel memory or other processes' memory
2. **Control flow integrity**: Kernel code pages read-only; user code never executes in Ring 0
3. **Capability unforgeability**: Capabilities only created/transferred by kernel
4. **Validated syscall entry**: All user→kernel transitions validated

### P1 — High (Required for Multi-Process)
5. **Process isolation**: Separate address spaces, no shared writable mappings by default
6. **Least-privilege device access**: Drivers granted only needed MMIO/IRQ ranges
7. **Kernel heap integrity**: Allocator metadata protected (guard pages, canaries)
8. **IPC mediation**: All cross-process communication via kernel-validated capabilities

### P2 — Medium (Hardening)
9. **KASLR**: Kernel base randomized at boot
10. **Stack canaries**: Kernel stack overflow detection
11. **W^X enforcement**: No writable+executable pages
12. **Audit logging**: Structured, tamper-evident security events

### P3 — Defense in Depth
13. **IOMMU**: DMA isolation for devices
14. **SELinux-style labels**: Mandatory access control on capabilities
15. **Verified boot**: Kernel image signature verification
16. **Remote attestation**: TPM-based platform integrity

---

## Current Gaps (Stage 2 → Stage 3)

| Gap | Risk if Unaddressed | Stage 3 Task |
|-----|---------------------|--------------|
| No paging / Ring 3 | All code Ring 0; any bug = total compromise | Page tables + user/kernel split |
| No heap allocator | No dynamic kernel objects; fixed arrays only | `alloc` + `linked_list_allocator` |
| No syscall mechanism | No controlled Ring 3 → Ring 0 transition | `syscall`/`sysret` + IDT gate |
| No capability system | No fine-grained authority delegation | CNode + capability syscalls |
| No process abstraction | No isolation unit | `Process` + `AddressSpace` structs |
| No IOMMU | DMA can overwrite kernel | VT-d/AMD-Vi driver (later) |

---

## Assumptions

1. **Hardware trust**: CPU, MMU, TPM behave per spec (no microcode backdoors)
2. **Bootloader integrity**: `bootloader` crate + UEFI Secure Boot (future) verify kernel
3. **Rust compiler trust**: No miscompilation introducing UB (mitigated: `rustc` + `cargo audit`)
4. **No side-channel mitigation yet**: Spectre/Meltdown not addressed (documented limitation)
5. **Single-core initially**: SMP adds race conditions; start uniprocessor

---

## Validation Strategy

| Objective | Test Method |
|-----------|-------------|
| Memory isolation | QEMU: user process tries to read kernel address → page fault |
| Capability unforgeability | Fuzz syscall args with invalid capability handles → ENOTCAPABLE |
| Syscall validation | Pass invalid user pointers → EFAULT, no kernel crash |
| Process isolation | Two processes, each writes own memory → no cross-corruption |
| Driver least privilege | VirtIO driver granted only its MMIO region → other MMIO faults |

---

## References

- [seL4 Capability Model](https://sel4.systems/Info/CapabilityModel/)
- [Capsicum Capability System](https://www.cl.cam.ac.uk/research/security/capsicum/)
- [x86_64 Paging](https://wiki.osdev.org/Paging)
- [Intel VT-d / AMD-Vi](https://www.intel.com/content/www/us/en/architecture-and-technology/virtualization/vt-directed-io.html)
- [Rust Security Audit](https://github.com/RustSec/rustsec)