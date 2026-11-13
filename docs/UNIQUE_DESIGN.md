# Xscraf-OS: Unique OS Design

## Executive Summary

Xscraf-OS is **not** a clone of Windows or Linux. It uses a distinctive architecture based on:

1. **Task-based execution** (lightweight, not heavy processes)
2. **Capability-based security** (mandatory access control, no ambient authority)
3. **Priority budgets** (fairness-aware scheduling, not traditional priority)
4. **Message-passing IPC** (no shared memory by default)
5. **Hierarchical task trees** (parent/child relationships with delegation)

This document explains what makes each unique.

---

## 1. Task vs Process vs Thread

### Windows Model (Traditional)
```
Process (address space, handles, tokens)
  ├─ Thread (execution context)
  ├─ Thread
  └─ Thread
  
Model: Heavy processes, in-process threads, shared memory default
```

### Linux Model (POSIX)
```
Process (address space, file descriptors, uid/gid)
  ├─ Thread (lightweight, shared memory)
  └─ Thread

Model: fork/exec, COW memory, thread-local storage
```

### Xscraf-OS Model (UNIQUE)
```
Task (sealed address space, capability set, CPU budget)
  ├─ Message Queue (received messages, no shared memory)
  └─ Context (registers, entry point, parent ID)

Model: Lightweight tasks, sealed spaces, message-passing
```

### Key Differences

| Feature | Windows | Linux | Xscraf-OS |
|---------|---------|-------|-----------|
| **Execution unit** | Process + Thread | Process + Thread | Task (single model) |
| **Memory model** | Shared heap + heap handles | Shared heap + CoW | Sealed per-task |
| **IPC** | Pipes, sockets, shared memory | Pipes, sockets, shared memory | Message-passing only |
| **Inheritance** | DLL loading, handle tables | fork/exec + fd inheritance | Parent grants capabilities |
| **Identity** | SID, token, ACL | uid/gid, groups | Capabilities (no identity) |

**Why this is unique**: No process forking. No DLL loading. No uid/gid. Pure capability-based.

---

## 2. Capability-Based Security (NO WINDOWS/LINUX EQUIVALENT)

### Traditional Model (Windows/Linux)
```
uid=1000 → can read /home/alice/
uid=0   → can read /root/
groups  → file access
umask   → default perms

Problem: Ambient authority ("I own this UID, so I can do anything for that UID")
```

### Xscraf-OS Capability Model
```
Task has explicit capabilities:
  - CapRead(0x1000, 0x1000)  -- can only read ONE 4KB page
  - CapWrite(0x2000, 0x1000) -- can only write ONE specific 4KB page
  - CapSendMessage(taskB, 4KB) -- can only send TO taskB, max 4KB
  - CapAllocMemory(1MB)        -- can allocate up to 1MB
  
No task can act without a capability.
Capabilities cannot be forged (kernel-only creation).
Capabilities can be attenuated (restricted) but not amplified.
```

**Example: Creating a Child Task**
```rust
Parent task (caps = [CapCreateTask, CapSendMessage(Child, 4KB)]):
  syscall CreateTask(entry_point) → validates CapCreateTask
  
Kernel:
  child_caps = attenuate(parent_caps)  // child gets SUBSET
  // NOT: child_caps = clone(parent_caps)
  
Child task:
  Can ONLY send messages to parent (inherited CapSendMessage)
  Cannot create other tasks (CapCreateTask not inherited)
```

**Why this is unique**: No setuid. No chmod. No ACLs. Only capabilities.

---

## 3. Priority Budgets (NOT Traditional Priority)

### Linux Priority Model
```
nice level: -20 (high) to +19 (low)
Task with priority 5: Gets CPU when higher-priority tasks block
Task with priority 20: Starves if higher-priority tasks exist

Problem: Low-priority task may never run
```

### Windows Priority Model
```
THREAD_PRIORITY_CRITICAL    (highest)
THREAD_PRIORITY_HIGHEST
...
THREAD_PRIORITY_LOWEST      (lowest)

Problem: Same as Linux (priority inversion)
```

### Xscraf-OS Budget Model (UNIQUE)
```
Each task has:
  priority: 0-255 (higher = more CPU)
  budget_ms: N milliseconds per round

Scheduler algorithm (round-robin within priority):
  1. Group tasks by priority
  2. Within each group, give equal time
  3. After all groups run once, return to top priority
  
Fairness guarantee: Even lowest-priority task WILL run
  (no starvation possible)

Example:
  Logger: priority=200, budget=50ms
  Worker: priority=128, budget=100ms
  
Schedule per round:
  1. Logger runs 50ms
  2. Worker runs 100ms
  3. Loop back
  
Result: Logger gets ~33% CPU, Worker gets ~67%
  (proportional to budget, within priority level)
```

**Why this is unique**: Mandatory fairness. No starvation. Budget-aware.

---

## 4. Message-Passing IPC (NOT Shared Memory)

### Linux IPC
```
Pipe:    P1 → (kernel buffer) → P2  [OK for streaming]
Socket:  P1 → (kernel) → P2          [OK for network]
SysV:    P1 + P2 → shared memory     [Fast but risky]
Mmap:    P1 + P2 → shared memory     [Fast but risky]

Problem: Shared memory allows races, corruption
```

### Windows IPC
```
Pipe:    P1 → (kernel buffer) → P2
Socket:  P1 → (kernel) → P2
Shared Memory: CreateFileMapping + MapViewOfFile
              [Fast but risky]

Problem: Same as Linux
```

### Xscraf-OS IPC (UNIQUE)
```
ONLY method: Message passing
  Task A: SEND_MESSAGE syscall(dest=B, data, size)
  ↓ (kernel validates CapSendMessage(B, size))
  ↓ (kernel copies data from A's address space)
  ↓ (kernel enqueues to B's message queue)
  
  Task B: RECEIVE_MESSAGE syscall()
  ← (kernel copies message to B's address space)
  ← (B has exclusive access to data)

Guarantees:
  - No shared pointers
  - No race conditions (kernel copies atomic)
  - No memory corruption
  - Message size bounded by capability
```

**Why this is unique**: No shared memory. Pure isolation. Slower but safer.

---

## 5. Hierarchical Task Trees (NOT LIKE UNIX)

### Unix Model
```
init (PID 1)
  └─ systemd (PID 2)
      ├─ sshd (PID X)
      ├─ bash (PID Y)
      └─ ...

Relationship: Parent (init) → Child (any daemon)
Problem: Child does NOT inherit parent's privileges
  (if parent drops UID, child keeps its own UID)
```

### Xscraf-OS Model (UNIQUE)
```
Init Task
  ├─ Logger Task
  │   Capabilities: SendMessage(parent), TimerInterrupt, Exit
  │   (inherited SUBSET of parent's caps)
  │
  └─ Worker Task
      Capabilities: AllocMemory(1MB), ReceiveMessage(4KB), Exit
      (inherited DIFFERENT SUBSET)

Key difference:
  - Parent CONTROLS what capabilities child gets
  - Child CANNOT escalate (can only use granted caps)
  - Parent can REVOKE capabilities at runtime
  - Task tree is explicit (not implicit from PID)
```

**Why this is unique**: Explicit capability delegation. No escalation path.

---

## Summary: What Makes Xscraf-OS Unique

| Aspect | Windows | Linux | Xscraf-OS |
|--------|---------|-------|-----------|
| **Execution model** | Process/Thread | Process/Thread | Task (single) |
| **Security** | ACLs, SIDs | uid/gid, capabilities (kernel) | Pure capabilities (userspace visible) |
| **IPC default** | Shared memory | Shared memory | Message passing |
| **Task creation** | CreateProcess API | fork/exec | Capability-based CreateTask |
| **Inheritance** | DLL loading, handles | fork duplication | Capability attenuation |
| **Scheduling** | Dynamic priorities | CFS (Linux 2.6.23+) | Priority budgets (fairness) |
| **Memory model** | Virtual addressing | Virtual addressing | Virtual + sealed spaces |
| **API philosophy** | Win32 (broad surface) | POSIX (narrow surface) | Capability syscalls (minimal) |

---

## Example: Complete Workflow

### 1. Init Task Starts
```
Kernel initializes Init task with:
  - Full capabilities (special boot cap)
  - Entry point at kernel load address
  - 1 MB stack
```

### 2. Init Creates Logger Task
```
Init: syscall CreateTask(logger_code)
  → Validates: CapCreateTask ✓
  
Kernel:
  1. Allocate new address space (PML4 table)
  2. Map logger code at 0x400000
  3. Map logger stack at 0x500000
  4. Create capset: [CapSendMessage(init), CapExit]
  5. Add to ready queue
  → Return logger_task_id = 2
```

### 3. Init Creates Worker Task
```
Init: syscall CreateTask(worker_code)
  → Validates: CapCreateTask ✓
  
Kernel:
  1. Allocate new address space (separate PML4)
  2. Map worker code at 0x410000
  3. Map worker stack at 0x510000
  4. Create capset: [CapAllocMemory(1MB), CapReceiveMessage(4KB), CapExit]
  5. Add to ready queue
  → Return worker_task_id = 3
```

### 4. Logger Sends Message
```
Logger: syscall SendMessage(dest=3, data="hello", size=5)
  → Validates: CapSendMessage(3, 4KB) ✓ (5 < 4096)
  
Kernel:
  1. Copy 5 bytes from logger's address space
  2. Create Message(from=2, to=3, data=[h,e,l,l,o])
  3. Enqueue to worker's message queue
  → Return success
```

### 5. Worker Receives Message
```
Worker: syscall ReceiveMessage(buf, 4KB)
  → Validates: CapReceiveMessage(4KB) ✓
  
Kernel:
  1. Dequeue from worker's message queue
  2. Copy message data to buf
  3. Return message length
  
Worker: buf = "hello" (isolated copy, no shared memory)
```

### 6. Scheduler Runs Both Tasks
```
Round 1:
  - Logger: runs for 50ms (priority 200, budget 50ms)
  - Kernel: timer tick, saves context, loads Worker
  - Worker: runs for 100ms (priority 128, budget 100ms)
  - Kernel: timer tick, saves context, loads Logger
  
Fairness: Logger gets 50/150 ≈ 33% CPU
          Worker gets 100/150 ≈ 67% CPU
          Both tasks ALWAYS get time (no starvation)
```

---

## Performance vs Security Trade-off

Xscraf-OS prioritizes **security** over **performance**:

- **No shared memory** → Message copying overhead (slower)
- **Capability checks** → Syscall validation overhead (slower)
- **Budget-based scheduling** → More context switches (slower)

But gains:
- **Memory isolation** → One task cannot corrupt another
- **No privilege escalation** → Only capabilities grant access
- **Deadlock prevention** → Hierarchical task trees + message bounds
- **Predictable latency** → Budget-based scheduling

---

## Next: What Would Break This Design?

These would violate Xscraf-OS's principles:

1. **Shared memory between tasks** (defeats isolation)
2. **Task with uid/gid** (defeats capabilities)
3. **fork/exec** (defeats explicit creation)
4. **Ambient authority** (defeats least privilege)
5. **Unbounded message sizes** (defeats cap attenuation)
6. **Priority without budgets** (defeats fairness)

Xscraf-OS is designed to REJECT all of these.

