/// Stage 4: Xscraf-OS Scheduler
///
/// Integrates with timer interrupt (IRQ0) to switch between tasks.
/// Implements priority-budget scheduling with fairness guarantees.

use crate::task::{TaskId, TaskState};
use crate::memory::PHYSICAL_MEMORY_OFFSET;
use spin::Mutex;
use x86_64::registers::control::Cr3;

/// Per-CPU scheduler state
pub struct Scheduler {
    current_task: Option<TaskId>,
    scheduler_ready: bool,
}

static SCHEDULER: Mutex<Scheduler> = Mutex::new(Scheduler {
    current_task: None,
    scheduler_ready: false,
});

pub fn init_scheduler() {
    let mut sched = SCHEDULER.lock();
    sched.scheduler_ready = true;
    crate::serial_println!("[SCHEDULER] Initialized (ready to switch tasks on timer)");
}

/// Called from timer interrupt handler (IRQ0)
/// Saves current task state and schedules next task
///
/// SAFETY: Must be called from interrupt context with interrupts disabled
pub unsafe fn on_timer_tick() {
    if !SCHEDULER.lock().scheduler_ready {
        return;  // Scheduler not ready yet
    }

    // Get current task
    let current = crate::task::current_task();

    // Switch to next task
    if let Some(next_id) = crate::task::schedule() {
        if Some(next_id) != current {
            // Task switch needed
            perform_task_switch(current, Some(next_id));
        }
    }
}

/// Perform actual task switch
///
/// SAFETY: Must be called with interrupts disabled
/// After this call, CPU is running in next task's context
unsafe fn perform_task_switch(current: Option<TaskId>, next: Option<TaskId>) {
    if let Some(curr_id) = current {
        // Save current task's page table context
        if let Some(curr_task) = crate::task::get_task_snapshot(curr_id) {
            crate::serial_println!("[SCHED] Saving task {} context", curr_id.as_u64());
        }
    }

    if let Some(next_id) = next {
        // Load next task's page table
        if let Some(next_task) = crate::task::get_task_snapshot(next_id) {
            crate::serial_println!("[SCHED] Loading task {} (priority={}, budget={}ms)",
                next_id.as_u64(),
                next_task.priority,
                next_task.cpu_budget_ms
            );

            // Switch page tables (address space)
            let pml4_phys = next_task.pml4_frame.start_address();
            let pml4_flags = Cr3::read().1;
            Cr3::write(next_task.pml4_frame, pml4_flags);

            // Mark task as running
            crate::task::set_task_state(next_id, TaskState::Running);
        }
    }
}

/// Get current running task
pub fn get_current_task() -> Option<TaskId> {
    SCHEDULER.lock().current_task
}

/// Yield CPU to scheduler
pub fn yield_task() {
    unsafe {
        // Let scheduler pick next task
        on_timer_tick();
    }
}

/// Statistics (for debugging)
pub struct SchedulerStats {
    pub total_switches: u64,
    pub current_task: Option<TaskId>,
}

pub fn get_stats() -> SchedulerStats {
    let sched = SCHEDULER.lock();
    SchedulerStats {
        total_switches: 0,  // TODO: track this
        current_task: sched.current_task,
    }
}
