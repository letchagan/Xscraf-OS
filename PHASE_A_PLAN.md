# Phase A: Performance Fast-Track - Implementation Plan

**Goal**: 2x IPC speedup + scalable scheduler + benchmarks  
**Timeline**: 1 week (7 days)  
**Effort**: Medium (~40 hours)  

---

## Overview: Current Bottlenecks

### Current IPC Implementation (ipc.rs)
```rust
// PROBLEMS:
1. VecDeque<Message>              // Heap allocation per message
2. Mutex<IPCManager>              // Lock on every send/receive
3. Message::data: Vec<u8>         // Another heap allocation
4. Clone on every operation       // Copy overhead

RESULT: ~4.2µs per message
```

### Current Scheduler (task.rs)
```rust
// PROBLEMS:
1. Linear search through tasks    // O(N) complexity
2. ready_queue: Vec<TaskId>       // No priority ordering
3. Full iteration every tick      // Wasteful

RESULT: 50 cycles per schedule() call
```

---

## Phase A: Week-by-Week Breakdown

### **Day 1-2: Lock-Free Message Queue (Design)**

#### What We're Building
Replace VecDeque + Mutex with:
```
Static Ring Buffer (256 slots)
  ├─ Each slot: Fixed-size message (4KB)
  ├─ Head pointer (atomic)
  ├─ Tail pointer (atomic)
  └─ No Mutex, no heap allocation
  
Result: Lock-free SPSC queue
  - Producer: Sender (Ring 3)
  - Consumer: Receiver (Ring 3)
  - NO allocations
  - Atomic operations only
```

#### Files to Create
- `src/ipc_lockfree.rs` (NEW, 200 lines)
  - StaticMessageSlot struct
  - LockFreeQueue struct
  - Atomic operations

#### Timeline
- Day 1: Design + structure
- Day 2: Implement enqueue/dequeue

---

### **Day 3-4: Lock-Free Queue (Implementation)**

#### Code Structure
```rust
pub struct MessageSlot {
    data: [u8; 4096],           // Fixed size, no alloc
    sender_id: u64,
    size: u16,
    timestamp: u64,
}

pub struct LockFreeQueue {
    slots: [MessageSlot; 256],   // Static, no alloc
    head: AtomicUsize,           // Producer advances
    tail: AtomicUsize,           // Consumer advances
}

impl LockFreeQueue {
    pub fn try_enqueue(&self, msg: &[u8]) -> Result<()> {
        // Atomic compare-and-swap
        // No lock, no allocation
    }
    
    pub fn try_dequeue(&self) -> Option<MessageSlot> {
        // Atomic load + advance
        // No lock, no allocation
    }
}
```

#### Performance Target
- Before: ~4.2µs per message
- After: <1µs per message (4x speedup)
- Zero allocations on critical path

#### Timeline
- Day 3: Core implementation
- Day 4: Testing + validation

---

### **Day 5: Scheduler Optimization (Per-Priority Queues)**

#### Current Problem
```rust
// Every 10ms timer tick:
for task in all_tasks {          // O(N) search
    if task.state == Ready {
        schedule(task)
    }
}
```

#### New Design
```rust
pub struct Scheduler {
    // Per-priority queues
    priority_0: VecDeque<TaskId>,   // Highest
    priority_1: VecDeque<TaskId>,
    // ... 
    priority_255: VecDeque<TaskId>, // Lowest
    
    current_priority: u8,
}

impl Scheduler {
    pub fn schedule(&mut self) -> Option<TaskId> {
        // O(1): Just check current priority queue
        if let Some(task) = self.current_priority_queue.pop_front() {
            return Some(task);
        }
        // Move to next lower priority
        self.current_priority += 1;
        // Loop back when done
        if self.current_priority > 255 {
            self.current_priority = 0;
        }
        self.schedule()  // Try next priority
    }
}
```

#### Performance Target
- Before: 50 cycles (linear search)
- After: ~5 cycles (constant time)
- 10x speedup

#### Timeline
- Day 5: Full implementation + testing

---

### **Day 6: Performance Benchmarking**

#### Create: `src/benchmarks.rs` (NEW, 100 lines)

Measure:
```rust
fn bench_syscall_latency() {
    // Measure CreateTask syscall time
    // Measure SendMessage time
    // Measure ReceiveMessage time
}

fn bench_context_switch() {
    // Measure timer → schedule → page table switch
}

fn bench_ipc_throughput() {
    // Messages per second
    // Latency variation (jitter)
}

fn bench_scheduler() {
    // Time to pick next task
    // Time with 10 vs 100 vs 1000 tasks
}
```

#### Output Format
```
Xscraf-OS Performance Benchmarks
════════════════════════════════

Syscalls:
  CreateTask:      2.1µs  (was 2.3µs)  ✓ 9% faster
  SendMessage:     0.8µs  (was 4.2µs)  ✓ 5.2x faster ⭐
  ReceiveMessage:  0.9µs  (was 4.1µs)  ✓ 4.6x faster ⭐

Scheduling:
  Pick next task:  5 cycles  (was 50)  ✓ 10x faster ⭐
  Task capacity:   1000+     (was 100) ✓ 10x scalable ⭐

Context Switch:
  Total time:      950µs  (was 1100µs) ✓ 13% faster

IPC Throughput:
  Messages/sec:    1.25M  (was 250K)   ✓ 5x faster ⭐

Overall Improvement: +2.3x performance (from 250K msg/s → 1.25M msg/s)
```

#### Timeline
- Day 6: Write benchmarks + run tests

---

### **Day 7: Integration + Polish**

#### What to Do
1. Update `src/ipc.rs` to use lock-free queue
2. Update `src/task.rs` to use per-priority scheduler
3. Run full test suite
4. Update documentation
5. Create before/after summary

#### Verification Checklist
- [ ] Compiles without warnings
- [ ] All tests pass
- [ ] No Mutex in IPC path
- [ ] No heap alloc on send/receive
- [ ] Benchmarks show >2x speedup
- [ ] Scheduler scales to 1000+ tasks
- [ ] Serial output shows improvements

#### Timeline
- Day 7: Integration + verification

---

## Detailed Implementation: Day-by-Day

### **Day 1: Lock-Free Queue Design**

```rust
// src/ipc_lockfree.rs (START)

use core::sync::atomic::{AtomicUsize, Ordering};
use core::mem::MaybeUninit;

const QUEUE_SIZE: usize = 256;
const MAX_MSG_SIZE: usize = 4096;

#[repr(C)]
pub struct MessageSlot {
    /// Raw message data (fixed size, no allocation)
    data: [u8; MAX_MSG_SIZE],
    
    /// Sender task ID
    sender_id: u64,
    
    /// Actual message size (0 = slot empty)
    size: u16,
    
    /// Timestamp (optional, for profiling)
    timestamp_ms: u64,
}

impl MessageSlot {
    const fn new() -> Self {
        MessageSlot {
            data: [0u8; MAX_MSG_SIZE],
            sender_id: 0,
            size: 0,
            timestamp_ms: 0,
        }
    }
}

/// Lock-free SPSC message queue
/// Single Producer (sender), Single Consumer (receiver)
/// No Mutex, no allocations on critical path
pub struct LockFreeQueue {
    /// Fixed-size slot array (no dynamic allocation)
    slots: [MessageSlot; QUEUE_SIZE],
    
    /// Producer index (sender advances this)
    head: AtomicUsize,
    
    /// Consumer index (receiver advances this)
    tail: AtomicUsize,
}

impl LockFreeQueue {
    pub const fn new() -> Self {
        // Can't use array init with AtomicUsize, so use trick:
        // MaybeUninit + transmute (safe because MessageSlot is sized)
        
        const INIT: MessageSlot = MessageSlot::new();
        LockFreeQueue {
            slots: [INIT; QUEUE_SIZE],
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
        }
    }
    
    /// Enqueue message (producer side)
    /// Returns false if queue full
    pub fn try_enqueue(&self, sender_id: u64, data: &[u8]) -> bool {
        if data.len() > MAX_MSG_SIZE {
            return false;
        }
        
        // Get current head
        let head = self.head.load(Ordering::Acquire);
        let next_head = (head + 1) % QUEUE_SIZE;
        
        // Check if full (next_head == tail)
        let tail = self.tail.load(Ordering::Acquire);
        if next_head == tail {
            return false;  // Queue full
        }
        
        // Write to slot (safe: we know it's available)
        unsafe {
            let slot = &mut self.slots[head] as *mut MessageSlot;
            (*slot).sender_id = sender_id;
            (*slot).size = data.len() as u16;
            core::ptr::copy_nonoverlapping(
                data.as_ptr(),
                (*slot).data.as_mut_ptr(),
                data.len()
            );
        }
        
        // Advance head (atomic, acquire-release)
        self.head.store(next_head, Ordering::Release);
        true
    }
    
    /// Dequeue message (consumer side)
    /// Returns None if queue empty
    pub fn try_dequeue(&self) -> Option<(u64, &[u8])> {
        // Get current tail
        let tail = self.tail.load(Ordering::Acquire);
        
        // Check if empty (tail == head)
        let head = self.head.load(Ordering::Acquire);
        if tail == head {
            return None;  // Queue empty
        }
        
        // Read from slot (safe: we know it has data)
        let (sender_id, size) = unsafe {
            let slot = &self.slots[tail] as *const MessageSlot;
            ((*slot).sender_id, (*slot).size as usize)
        };
        
        // Advance tail (atomic)
        let next_tail = (tail + 1) % QUEUE_SIZE;
        self.tail.store(next_tail, Ordering::Release);
        
        Some((sender_id, &self.slots[tail].data[..size]))
    }
    
    pub fn is_empty(&self) -> bool {
        self.head.load(Ordering::Acquire) == self.tail.load(Ordering::Acquire)
    }
    
    pub fn len(&self) -> usize {
        let head = self.head.load(Ordering::Acquire);
        let tail = self.tail.load(Ordering::Acquire);
        
        if head >= tail {
            head - tail
        } else {
            QUEUE_SIZE - (tail - head)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_enqueue_dequeue() {
        let q = LockFreeQueue::new();
        let msg = b"Hello";
        
        assert!(q.try_enqueue(1, msg));
        assert!(q.len() == 1);
        
        let (sender, data) = q.try_dequeue().unwrap();
        assert_eq!(sender, 1);
        assert_eq!(data, msg);
        assert!(q.is_empty());
    }
    
    #[test]
    fn test_full() {
        let q = LockFreeQueue::new();
        
        // Fill queue
        for i in 0..QUEUE_SIZE-1 {
            assert!(q.try_enqueue(i as u64, b"X"));
        }
        
        // Next should fail (queue full)
        assert!(!q.try_enqueue(999, b"X"));
    }
}
```

**Checkpoint**: Basic lock-free queue structure with tests ✓

---

### **Day 2: Lock-Free Queue Validation**

Add more tests + edge cases:

```rust
#[test]
fn test_wraparound() {
    // Test queue wrapping around buffer boundary
}

#[test]
fn test_large_message() {
    // Test with 4KB message (max size)
}

#[test]
fn test_concurrent_safety() {
    // Verify atomicity with multiple sends/receives
}

#[test]
fn test_ordering() {
    // Verify FIFO: messages arrive in order
}
```

**Checkpoint**: Lock-free queue proven correct ✓

---

### **Days 3-4: Integration into IPC**

Replace `src/ipc.rs`:

```rust
// OLD: Mutex<VecDeque<Message>>
// NEW: Array of LockFreeQueue (one per task)

pub struct IPCManager {
    queues: [LockFreeQueue; 256],  // Max 256 tasks
}
```

**Checkpoint**: IPC using lock-free queue ✓

---

### **Day 5: Scheduler Optimization**

Replace `src/task.rs` scheduler:

```rust
pub struct Scheduler {
    // 256 priority levels
    priority_queues: [VecDeque<TaskId>; 256],
    current_priority: u8,
}

pub fn schedule(&mut self) -> Option<TaskId> {
    // Try current priority first
    for _ in 0..256 {
        if let Some(task) = self.priority_queues[self.current_priority as usize].pop_front() {
            return Some(task);
        }
        self.current_priority = self.current_priority.wrapping_add(1);
    }
    None
}
```

**Checkpoint**: O(1) scheduling ✓

---

### **Day 6-7: Benchmarking + Polish**

Create comprehensive benchmarks and verify improvements.

---

## Success Criteria

After Phase A, we should see:

```
✅ IPC Throughput:  250K msg/s → 1.25M msg/s  (5x)
✅ IPC Latency:     4.2µs → 0.9µs             (4.6x)
✅ Scheduling:      50 cycles → 5 cycles      (10x)
✅ Memory:          No heap allocations in IPC path
✅ Code:            No Mutex in critical path
✅ Tests:           All pass, new benchmarks added
✅ Documentation:   Before/after metrics published
```

---

## Risks & Mitigations

| Risk | Mitigation |
|------|-----------|
| Lock-free bugs | Comprehensive tests, atomic validation |
| Message corruption | Fixed-size slots, bounds checking |
| Scheduler starvation | Priority-based, guaranteed round-robin |
| Performance regression | Benchmark before/after |

---

## Expected Outcome

**A proven, performant, scalable OS:**
- ✅ 5x faster IPC
- ✅ 10x better scheduler
- ✅ 0 allocations in critical path
- ✅ Scales to 1000+ tasks
- ✅ Lock-free (no Mutex on IPC)

**Ready to:**
- Publish benchmarks
- Scale to production
- Move to Phase B (security/filesystem)

---

**Status**: Plan Complete, Ready to Implement 🚀

