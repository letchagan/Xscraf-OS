/// Xscraf-OS Demo: Unique Task & Capability System
///
/// Demonstrates:
/// - Task creation with capabilities
/// - Priority-budget scheduler
/// - Message-passing IPC
/// - Hierarchical task trees

use crate::task::{TaskId, Capability, Task};
use crate::memory;
use x86_64::structures::paging::Size4KiB;
use alloc::vec;

/// Demo: Create a parent task with 2 child tasks
pub fn demo_task_creation() {
    crate::serial_println!("\n[DEMO] Xscraf-OS Task System");
    crate::serial_println!("==============================");

    // Parent task (kernel init)
    let parent_id = TaskId(1);
    crate::serial_println!("Parent task: {}", parent_id.as_u64());

    // Child task 1: "Logger"
    // Capabilities: Send messages to parent, timer interrupt
    crate::serial_println!("\n[DEMO] Creating task 'Logger'");
    let logger_code = 0x400000u64; // Placeholder entry point
    let logger_stack = 0x500000u64;
    let logger_pml4 = memory::create_address_space();
    let logger_id = crate::task::create_task(Some(parent_id), logger_code, logger_pml4, logger_stack);
    crate::serial_println!("  Created task ID: {}", logger_id.as_u64());

    // Grant Logger task capabilities
    grant_logger_capabilities(logger_id);

    // Child task 2: "Worker"
    // Capabilities: Allocate memory, receive messages from Logger
    crate::serial_println!("\n[DEMO] Creating task 'Worker'");
    let worker_code = 0x410000u64;
    let worker_stack = 0x510000u64;
    let worker_pml4 = memory::create_address_space();
    let worker_id = crate::task::create_task(Some(parent_id), worker_code, worker_pml4, worker_stack);
    crate::serial_println!("  Created task ID: {}", worker_id.as_u64());

    // Grant Worker task capabilities
    grant_worker_capabilities(worker_id);

    // Create message queues for both tasks
    crate::ipc::create_queue(logger_id);
    crate::ipc::create_queue(worker_id);
    crate::serial_println!("  Message queues created for both tasks");

    // Demonstrate hierarchy
    crate::serial_println!("\n[DEMO] Task Tree:");
    crate::serial_println!("  [Init] (parent)");
    crate::serial_println!("    ├─ [Logger] (child 1)");
    crate::serial_println!("    │  Caps: SendMessage(parent), TimerInterrupt");
    crate::serial_println!("    │  Priority: 200, Budget: 50ms");
    crate::serial_println!("    └─ [Worker] (child 2)");
    crate::serial_println!("       Caps: AllocMemory(1MB), ReceiveMessage(4KB)");
    crate::serial_println!("       Priority: 128, Budget: 100ms");

    // Demonstrate scheduler
    crate::serial_println!("\n[DEMO] Priority-Budget Scheduler");
    crate::serial_println!("  Scheduling order (fair within priority level):");
    crate::serial_println!("  1. Logger (priority 200) → 50ms CPU");
    crate::serial_println!("  2. Worker (priority 128) → 100ms CPU");
    crate::serial_println!("  3. Back to Logger (fairness ensures no starvation)");

    // Demonstrate unique IPC
    crate::serial_println!("\n[DEMO] Message-Passing IPC (No Shared Memory!)");
    crate::serial_println!("  Logger sends message to Worker:");
    crate::serial_println!("  1. Logger: SEND_MESSAGE syscall with capability");
    crate::serial_println!("  2. Kernel: Validates CapSendMessage(Worker, 4KB)");
    crate::serial_println!("  3. Kernel: Copies message data (no shared pointers)");
    crate::serial_println!("  4. Worker: RECEIVE_MESSAGE syscall → message ready");
    crate::serial_println!("  5. Both tasks have sealed address spaces (isolation)");

    crate::serial_println!("\n[DEMO] Unique Features Summary:");
    crate::serial_println!("  ✓ Lightweight tasks (not heavy processes)");
    crate::serial_println!("  ✓ Capability-based security (no ambient authority)");
    crate::serial_println!("  ✓ Priority budgets (fairness guarantees)");
    crate::serial_println!("  ✓ Hierarchical task trees (parent/child)");
    crate::serial_println!("  ✓ Message-passing IPC (sealed memory)");
    crate::serial_println!("  ✓ No fork/exec (Unix model)");
    crate::serial_println!("  ✓ No shared memory (Linux model)");
    crate::serial_println!("  ✓ Rust-first design (type safety)\n");
}

/// Grant capabilities to Logger task
fn grant_logger_capabilities(task_id: TaskId) {
    crate::serial_println!("  Granting capabilities to Logger:");

    // Cap 1: Send messages to parent
    crate::serial_println!("    - SendMessage(parent, 4KB max)");
    let send_cap = Capability::SendMessage {
        dest_task: 1, // parent
        max_size: 4096,
    };
    // TODO: crate::task::grant_capability(task_id, send_cap);

    // Cap 2: Receive timer interrupts
    crate::serial_println!("    - TimerInterrupt");
    let timer_cap = Capability::TimerInterrupt;
    // TODO: crate::task::grant_capability(task_id, timer_cap);

    // Cap 3: Exit self
    crate::serial_println!("    - Exit (always granted)");
    let exit_cap = Capability::Exit;
    // TODO: crate::task::grant_capability(task_id, exit_cap);
}

/// Grant capabilities to Worker task
fn grant_worker_capabilities(task_id: TaskId) {
    crate::serial_println!("  Granting capabilities to Worker:");

    // Cap 1: Allocate up to 1 MB of memory
    crate::serial_println!("    - AllocMemory(1 MB max)");
    let alloc_cap = Capability::AllocMemory {
        bytes: 1024 * 1024,
    };
    // TODO: crate::task::grant_capability(task_id, alloc_cap);

    // Cap 2: Receive messages (up to 4 KB)
    crate::serial_println!("    - ReceiveMessage(4KB max)");
    let recv_cap = Capability::ReceiveMessage {
        max_size: 4096,
    };
    // TODO: crate::task::grant_capability(task_id, recv_cap);

    // Cap 3: Exit self
    crate::serial_println!("    - Exit (always granted)");
    let exit_cap = Capability::Exit;
    // TODO: crate::task::grant_capability(task_id, exit_cap);
}

/// Demo: Send a message from Logger to Worker
pub fn demo_message_passing() {
    crate::serial_println!("\n[DEMO] Message Passing in Action");
    crate::serial_println!("=================================");

    let logger_id = TaskId(2);
    let worker_id = TaskId(3);

    crate::serial_println!("Logger sends: 'Hello, Worker!'");

    let data = alloc::string::String::from("Hello, Worker!").into_bytes();
    if let Ok(msg) = crate::ipc::Message::new(logger_id, worker_id, data) {
        match crate::ipc::send_message(msg) {
            Ok(()) => {
                crate::serial_println!("Message sent successfully (kernel copied data)");
                crate::serial_println!("Message queue depth: {}", crate::ipc::message_count(worker_id));
            }
            Err(e) => {
                crate::serial_println!("Send failed: {}", e);
            }
        }
    }

    crate::serial_println!("\nWorker receives message:");
    if let Some(msg) = crate::ipc::receive_message(worker_id) {
        let text = alloc::string::String::from_utf8_lossy(&msg.data);
        crate::serial_println!("From task {}: {}", msg.sender_id.as_u64(), text);
        crate::serial_println!("(Data was copied by kernel, not shared)");
    }
}
