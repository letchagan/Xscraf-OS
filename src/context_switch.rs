/// Context Switching for Xscraf-OS Tasks
///
/// Saves and restores CPU state when switching between tasks.
/// Not used on first task creation (bootloader sets initial state).

use crate::task::{Task, TaskContext};
use x86_64::registers::control::Cr3;
use x86_64::PhysAddr;

/// Save current CPU context (called before switching away from a task)
/// SAFETY: Must be called from kernel mode with interrupts disabled
#[inline(never)]
pub unsafe fn save_context(task: &mut Task) {
    // This would normally be done in an interrupt handler (timer tick)
    // For now, we save minimal state. A real implementation would use
    // a naked function to capture all registers.

    // Note: RIP, RSP, RFLAGS are typically saved by the exception handler
    // Here we just document the process.

    core::arch::asm!(
        // Save general-purpose registers to task context
        // This is a pseudo-code representation; actual asm! macro call
        // would be in the timer interrupt handler
    );
}

/// Load task's page table (switch address space)
pub fn load_page_table(task: &Task) {
    let pml4_phys = task.pml4_frame.start_address();
    unsafe {
        Cr3::write(task.pml4_frame, Cr3::read().1);
    }
}

/// Restore CPU context when resuming a task
/// SAFETY: Must be called from kernel mode; does not preserve caller-saved registers
#[inline(never)]
pub unsafe fn restore_context(task: &Task) -> ! {
    // Load task's page table
    load_page_table(task);

    // Jump to task's code (RIP) with task's stack (RSP)
    // This is a simplified version; real implementation uses task's saved context

    // Set up registers from task context
    let ctx = &task.context;

    core::arch::asm!(
        "mov rsp, {rsp}",
        "mov rbp, {rbp}",
        "mov rax, {rax}",
        "mov rbx, {rbx}",
        "mov rcx, {rcx}",
        "mov rdx, {rdx}",
        "mov rsi, {rsi}",
        "mov rdi, {rdi}",
        "mov r8, {r8}",
        "mov r9, {r9}",
        "mov r10, {r10}",
        "mov r11, {r11}",
        "mov r12, {r12}",
        "mov r13, {r13}",
        "mov r14, {r14}",
        "mov r15, {r15}",
        "push {rflags}",
        "popfq",
        "jmp {rip}",
        rsp = in(reg) ctx.rsp,
        rbp = in(reg) ctx.rbp,
        rax = in(reg) ctx.rax,
        rbx = in(reg) ctx.rbx,
        rcx = in(reg) ctx.rcx,
        rdx = in(reg) ctx.rdx,
        rsi = in(reg) ctx.rsi,
        rdi = in(reg) ctx.rdi,
        r8 = in(reg) ctx.r8,
        r9 = in(reg) ctx.r9,
        r10 = in(reg) ctx.r10,
        r11 = in(reg) ctx.r11,
        r12 = in(reg) ctx.r12,
        r13 = in(reg) ctx.r13,
        r14 = in(reg) ctx.r14,
        r15 = in(reg) ctx.r15,
        rflags = in(reg) ctx.rflags,
        rip = in(reg) ctx.rip,
        options(noreturn)
    );
}

/// Helper: Update task context with new values
pub fn update_context(task: &mut Task, rip: u64, rsp: u64) {
    task.context.rip = rip;
    task.context.rsp = rsp;
}
