/// Kairos-11 Task Manager
///
/// Implements lightweight, sealed, capability-based tasks with:
/// - Priority-budget scheduling (not traditional priority)
/// - Hierarchical task trees (parent/child relationships)
/// - Sealed address spaces (no shared memory by default)
/// - Message-passing IPC (kernel-copied)

use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;
use x86_64::structures::paging::PhysFrame;
use x86_64::PhysAddr;

/// Task ID (unique, never reused)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct TaskId(u64);

impl TaskId {
    pub fn as_u64(self) -> u64 {
        self.0
    }
}

/// Task state in the scheduler queue
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    /// Runnable, not currently executing
    Ready,
    /// Currently executing on CPU
    Running,
    /// Blocked waiting for event (I/O, message, etc.)
    Blocked,
    /// Terminated, waiting for reaping
    Zombie,
    /// Exited and reaped
    Dead,
}

/// Capability types (what tasks can do)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Capability {
    /// Read N bytes at address (sealed: address + size cannot grow)
    Read { addr: u64, size: u64 },
    /// Write N bytes at address
    Write { addr: u64, size: u64 },
    /// Create child task (limited by parent's caps)
    CreateTask,
    /// Send message to specific task (max N bytes)
    SendMessage { dest_task: u64, max_size: u64 },
    /// Receive messages (max N bytes per message)
    ReceiveMessage { max_size: u64 },
    /// Access I/O ports in range [min..max]
    AccessIO { min_port: u16, max_port: u16 },
    /// Allocate N bytes of task memory
    AllocMemory { bytes: u64 },
    /// CPU budget + priority (ms per round, 0-255 priority)
    CPUBudget { milliseconds: u64, priority: u8 },
    /// Receive timer ticks
    TimerInterrupt,
    /// Exit self
    Exit,
}

/// Capability set (what a task can do)
#[derive(Debug, Clone)]
pub struct CapabilitySet {
    caps: Vec<Capability>,
}

impl CapabilitySet {
    pub fn new() -> Self {
        CapabilitySet { caps: Vec::new() }
    }

    pub fn grant(&mut self, cap: Capability) {
        self.caps.push(cap);
    }

    pub fn has_capability(&self, cap: &Capability) -> bool {
        self.caps.iter().any(|c| {
            match (c, cap) {
                (Capability::Read { addr: a1, size: s1 }, Capability::Read { addr: a2, size: s2 }) => {
                    *a1 <= *a2 && *a1 + *s1 >= *a2 + *s2
                }
                (Capability::Write { addr: a1, size: s1 }, Capability::Write { addr: a2, size: s2 }) => {
                    *a1 <= *a2 && *a1 + *s1 >= *a2 + *s2
                }
                (c1, c2) => c1 == c2,
            }
        })
    }

    pub fn attenuate(&self, cap: Capability) -> Option<CapabilitySet> {
        if self.has_capability(&cap) {
            let mut subset = CapabilitySet::new();
            subset.grant(cap);
            Some(subset)
        } else {
            None
        }
    }
}

/// Task context (CPU registers, flags)
#[derive(Debug, Clone, Copy)]
pub struct TaskContext {
    pub rax: u64,
    pub rbx: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub rbp: u64,
    pub rsp: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    pub rip: u64,
    pub rflags: u64,
}

impl TaskContext {
    pub fn new(entry_point: u64, stack_ptr: u64) -> Self {
        TaskContext {
            rax: 0,
            rbx: 0,
            rcx: 0,
            rdx: 0,
            rsi: 0,
            rdi: 0,
            rbp: stack_ptr,
            rsp: stack_ptr,
            r8: 0,
            r9: 0,
            r10: 0,
            r11: 0,
            r12: 0,
            r13: 0,
            r14: 0,
            r15: 0,
            rip: entry_point,
            rflags: 0x202, // IF flag set (interrupts enabled)
        }
    }
}

/// A Kairos-11 task
pub struct Task {
    pub id: TaskId,
    pub parent_id: Option<TaskId>,
    pub state: TaskState,
    pub context: TaskContext,
    pub pml4_frame: PhysFrame<x86_64::structures::paging::Size4KiB>,
    pub capabilities: CapabilitySet,
    /// CPU budget in milliseconds per round
    pub cpu_budget_ms: u64,
    /// Priority (0-255, higher = more CPU time)
    pub priority: u8,
    /// CPU time used this round (ms)
    pub cpu_used_ms: u64,
}

impl Task {
    pub fn new(
        id: TaskId,
        parent_id: Option<TaskId>,
        entry_point: u64,
        pml4_frame: PhysFrame<x86_64::structures::paging::Size4KiB>,
        stack_ptr: u64,
    ) -> Self {
        Task {
            id,
            parent_id,
            state: TaskState::Ready,
            context: TaskContext::new(entry_point, stack_ptr),
            pml4_frame,
            capabilities: CapabilitySet::new(),
            cpu_budget_ms: 10,
            priority: 128,
            cpu_used_ms: 0,
        }
    }
}

/// Global task manager (Mutex-protected)
pub struct TaskManager {
    tasks: Vec<Task>,
    next_task_id: u64,
    current_task_id: Option<TaskId>,
    ready_queue: Vec<TaskId>,
}

impl TaskManager {
    pub fn new() -> Self {
        TaskManager {
            tasks: Vec::new(),
            next_task_id: 1,
            current_task_id: None,
            ready_queue: Vec::new(),
        }
    }

    pub fn create_task(
        &mut self,
        parent_id: Option<TaskId>,
        entry_point: u64,
        pml4_frame: PhysFrame<x86_64::structures::paging::Size4KiB>,
        stack_ptr: u64,
    ) -> TaskId {
        let id = TaskId(self.next_task_id);
        self.next_task_id += 1;

        let task = Task::new(id, parent_id, entry_point, pml4_frame, stack_ptr);
        self.tasks.push(task);
        self.ready_queue.push(id);

        id
    }

    pub fn get_task(&self, id: TaskId) -> Option<&Task> {
        self.tasks.iter().find(|t| t.id == id)
    }

    pub fn get_task_mut(&mut self, id: TaskId) -> Option<&mut Task> {
        self.tasks.iter_mut().find(|t| t.id == id)
    }

    pub fn set_task_state(&mut self, id: TaskId, state: TaskState) {
        if let Some(task) = self.get_task_mut(id) {
            task.state = state;
            if state == TaskState::Ready {
                self.ready_queue.push(id);
            }
        }
    }

    /// Priority-budget scheduler: round-robin within priority levels
    pub fn schedule(&mut self) -> Option<TaskId> {
        // Remove finished/dead tasks
        self.ready_queue.retain(|id| {
            if let Some(task) = self.get_task(*id) {
                task.state == TaskState::Ready
            } else {
                false
            }
        });

        if self.ready_queue.is_empty() {
            return None;
        }

        // Group by priority, round-robin within each priority
        self.ready_queue.sort_by_key(|id| {
            self.get_task(*id).map(|t| (core::u8::MAX - t.priority, *id)).unwrap_or((255, *id))
        });

        // Pick first ready task (highest priority, oldest in queue)
        self.ready_queue.remove(0).into()
    }

    pub fn grant_capability(&mut self, task_id: TaskId, cap: Capability) {
        if let Some(task) = self.get_task_mut(task_id) {
            task.capabilities.grant(cap);
        }
    }
}

/// Global task manager instance
static TASK_MANAGER: Mutex<Option<TaskManager>> = Mutex::new(None);

pub fn init() {
    let mut manager = TASK_MANAGER.lock();
    *manager = Some(TaskManager::new());
    crate::serial_println!("[TASK] Task manager initialized");
}

pub fn create_task(
    parent_id: Option<TaskId>,
    entry_point: u64,
    pml4_frame: PhysFrame<x86_64::structures::paging::Size4KiB>,
    stack_ptr: u64,
) -> TaskId {
    let mut manager = TASK_MANAGER.lock();
    if let Some(mgr) = manager.as_mut() {
        mgr.create_task(parent_id, entry_point, pml4_frame, stack_ptr)
    } else {
        panic!("Task manager not initialized");
    }
}

pub fn current_task() -> Option<TaskId> {
    let manager = TASK_MANAGER.lock();
    manager.as_ref().and_then(|m| m.current_task_id)
}

pub fn schedule() -> Option<TaskId> {
    let mut manager = TASK_MANAGER.lock();
    if let Some(mgr) = manager.as_mut() {
        let next = mgr.schedule();
        if let Some(id) = next {
            mgr.current_task_id = Some(id);
        }
        next
    } else {
        None
    }
}

pub fn get_task_snapshot(id: TaskId) -> Option<TaskSnapshot> {
    let manager = TASK_MANAGER.lock();
    if let Some(mgr) = manager.as_ref() {
        mgr.get_task(id).map(|t| TaskSnapshot {
            priority: t.priority,
            cpu_budget_ms: t.cpu_budget_ms,
            pml4_frame: t.pml4_frame,
            state: t.state,
        })
    } else {
        None
    }
}

pub fn set_task_state(id: TaskId, state: TaskState) {
    let mut manager = TASK_MANAGER.lock();
    if let Some(mgr) = manager.as_mut() {
        mgr.set_task_state(id, state);
    }
}

pub fn grant_capability(id: TaskId, cap: Capability) {
    let mut manager = TASK_MANAGER.lock();
    if let Some(mgr) = manager.as_mut() {
        mgr.grant_capability(id, cap);
    }
}

pub fn has_capability(id: TaskId, cap: &Capability) -> bool {
    let manager = TASK_MANAGER.lock();
    if let Some(mgr) = manager.as_ref() {
        if let Some(task) = mgr.get_task(id) {
            task.capabilities.has_capability(cap)
        } else {
            false
        }
    } else {
        false
    }
}

/// Lightweight snapshot of task info for scheduler
#[derive(Debug, Clone, Copy)]
pub struct TaskSnapshot {
    pub priority: u8,
    pub cpu_budget_ms: u64,
    pub pml4_frame: x86_64::structures::paging::PhysFrame<x86_64::structures::paging::Size4KiB>,
    pub state: TaskState,
}
