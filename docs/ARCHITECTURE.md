# AetherOS Architecture Documentation

## Overview

AetherOS is a Rust-based, bare-metal x86_64 operating system kernel. It follows a minimal, security-focused design with no_std/no_main, targeting the `x86_64-unknown-none` platform via a custom target specification.

## System Components

### 1. Boot Process
- **Bootloader**: `bootloader` crate v0.9.35 (BIOS/UEFI compatible)
- **Entry Point**: `_start` in `src/main.rs`
- **Target Spec**: `x86_64-aether_kernel.json` (no OS, no redzone, panic=abort)
- **Build Tool**: `cargo bootimage` creates bootable disk image

### 2. CPU Initialization (`src/main.rs`)
- SSE enable (CR0: clear EM, set MP; CR4: set OSFXSR, OSXMMEXCPT_ENABLE)
- GDT initialization with TSS and double-fault IST
- IDT initialization (exceptions + hardware IRQs)
- PIC initialization (8259, IRQ0/IRQ1 unmasked)
- PS/2 keyboard controller (i8042) initialization
- Enable CPU interrupts (`sti`)
- Main loop: process input queue + `hlt`

### 3. Global Descriptor Table (`src/gdt.rs`)
- Kernel code segment (64-bit)
- Task State Segment (TSS) with IST[0] for double-fault handling
- IST stack: 20 KiB static allocation (5 × 4 KiB pages)
- **Limitation**: No guard page for IST stack (requires paging)

### 4. Interrupt Descriptor Table (`src/interrupts.rs`)
- **Exceptions**: Breakpoint (int3), Double Fault (with IST)
- **Hardware IRQs**: Timer (IRQ0), Keyboard (IRQ1)
- **PIC**: Chained 8259, offsets 32/40
- **Keyboard Handler**: Reads scancode from port 0x60, enqueues to lock-free ring buffer, sends EOI immediately
- **Timer Handler**: Sends EOI only (no VGA/serial output)

### 5. Input Subsystem (`src/input.rs`)
- **ScancodeQueue**: Lock-free SPSC ring buffer (256 entries, power-of-2)
  - Producer: keyboard interrupt handler (single IRQ)
  - Consumer: main loop (single thread)
  - Atomic head/tail indices with Acquire/Release ordering
  - `UnsafeCell` for buffer + manual `Sync` impl (safe: single producer/consumer)
- **KeyboardProcessor**: `pc_keyboard` state machine (US 104-key, Scancode Set 1)
  - Runs in main loop, protected by `spin::Mutex`
  - Produces `DecodedKey::Unicode` or `DecodedKey::RawKey`

### 6. VGA Text Output (`src/vga_buffer.rs`)
- 80×25 text buffer at 0xB8000 (volatile writes)
- Colors: LightGreen on Black (configurable)
- **Features**:
  - Printable ASCII (0x20–0x7E)
  - Newline (`\n`) with scrolling
  - Tab (`\t`) → 8-column alignment
  - Backspace (0x08) → move left, clear, wrap to previous line
  - Hardware cursor via CRTC ports 0x3D4/0x3D5
- **Thread Safety**: `spin::Mutex<Writer>` + `interrupts::without_interrupts` in `_print`

### 7. Serial Output (`src/serial.rs`)
- UART 16550 at COM1 (0x3F8)
- `spin::Mutex<SerialPort>` + `interrupts::without_interrupts`
- Used for early boot logs and keyboard diagnostics

## Memory Layout (Current)

| Region | Address Range | Purpose |
|--------|---------------|---------|
| VGA Text Buffer | 0xB8000–0xB8FA0 | 80×25×2 bytes |
| Kernel Code/Data | 0x100000+ | Loaded by bootloader |
| Double-Fault IST | Static array | 20 KiB in `.bss` |
| Scancode Queue | Static array | 256 bytes in `.bss` |
| Keyboard State | `lazy_static` | `pc_keyboard` internal state |

## Concurrency Model

- **No preemption**: Single-threaded main loop + interrupt handlers
- **Interrupt Safety**:
  - Scancode queue: lock-free atomics (SPSC)
  - VGA/Serial: `Mutex` + `without_interrupts` (disables IRQs during critical section)
  - Keyboard processor: `Mutex` (main loop only)
  - PIC: `Mutex` (interrupt handlers + init)
- **No heap allocator yet** — all allocations static

## Security Boundaries (Current)

| Boundary | Status |
|----------|--------|
| Kernel/User mode | ❌ Not implemented (ring 0 only) |
| Memory isolation | ❌ No paging, flat physical map |
| Syscall interface | ❌ Not implemented |
| Device access control | ❌ Direct port I/O in kernel |
| Capability system | ❌ Planned (Stage 3) |

## Known Limitations

1. **No virtual memory** — identity mapping only, no page tables
2. **No heap allocator** — `alloc` crate not configured
3. **No user mode** — all code runs at CPL 0
4. **No filesystem** — storage stack not started
5. **No USB HID** — PS/2 only (laptop keyboards may not work on real hardware)
6. **No guard page for IST** — double-fault stack overflow undetected
7. **No ACPI** — timer calibration, power management unavailable
8. **Serial-only diagnostics** — no on-screen debug overlay

## Build & Test Commands

```bash
# Check compilation
cargo check

# Build bootable image
cargo bootimage

# Run basic keyboard test (headless)
powershell -ExecutionPolicy Bypass -File test_keyboard.ps1

# Run extended keyboard test (backspace, tab, shift, caps, enter)
powershell -ExecutionPolicy Bypass -File test_keyboard_extended.ps1
```

## QEMU Configuration (Test Scripts)

- Memory: 128 MiB
- Accelerator: WHPX (Windows Hypervisor Platform)
- Serial: File output (`qemu_serial.log`)
- Display: None (headless)
- Monitor: TCP localhost:5555/5556 for `sendkey` injection
- No reboot on panic (`-no-reboot`)

## Development Roadmap (High-Level)

| Stage | Focus | Status |
|-------|-------|--------|
| 1 | Baseline: boot, interrupts, input, VGA, serial | ✅ Done |
| 2 | Kernel correctness: exception handling, input buffering | ✅ Done |
| 3 | Security: threat model, capability design, user/kernel split | 📋 Planned |
| 4 | Memory: paging, heap, allocator, diagnostics | 📋 Planned |
| 5 | Storage: drivers, filesystem, validation | 📋 Planned |
| 6 | Terminal/Shell: line editor, commands | 📋 Planned |
| 7 | Graphics: framebuffer, compositor, desktop | 📋 Planned |
| 8 | Usability: services, packages, recovery | 📋 Planned |
| 9 | Testing/Hardening: automated tests, stress, fuzzing | 📋 Planned |

## File Index

```
src/
├── main.rs           # Entry point, init sequence, main loop
├── gdt.rs            # GDT, TSS, double-fault IST
├── interrupts.rs     # IDT, PIC, exception/IRQ handlers
├── input.rs          # Lock-free scancode queue, keyboard processor
├── vga_buffer.rs     # VGA text writer, cursor, scrolling, backspace, tab
└── serial.rs         # UART 16550 serial output
```

## References

- [Rust OSDev: Bare Bones](https://os.phil-opp.com/bare-bones/)
- [x86_64 crate docs](https://docs.rs/x86_64/)
- [pc-keyboard crate docs](https://docs.rs/pc-keyboard/)
- [Intel SDM Vol. 3A](https://www.intel.com/content/www/us/en/developer/articles/technical/intel-sdm.html)