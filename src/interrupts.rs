use crate::{gdt, input, serial_println};

use lazy_static::lazy_static;
use pic8259::ChainedPics;
use spin::Mutex;
use x86_64::instructions::port::Port;
use x86_64::structures::idt::{
    InterruptDescriptorTable,
    InterruptStackFrame,
};

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

pub static PICS: Mutex<ChainedPics> =
    Mutex::new(
        unsafe {
            ChainedPics::new(
                PIC_1_OFFSET,
                PIC_2_OFFSET,
            )
        }
    );

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
    Keyboard,
}

impl InterruptIndex {
    fn as_u8(self) -> u8 {
        self as u8
    }

    fn as_usize(self) -> usize {
        usize::from(self.as_u8())
    }
}

// --------------------------------------------------
// IDT
// --------------------------------------------------

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt =
            InterruptDescriptorTable::new();

        idt.breakpoint
            .set_handler_fn(
                breakpoint_handler
            );

        unsafe {
            idt.double_fault
                .set_handler_fn(
                    double_fault_handler
                )
                .set_stack_index(
                    gdt::DOUBLE_FAULT_IST_INDEX
                );
        }

        idt[
            InterruptIndex::Timer.as_usize()
        ]
        .set_handler_fn(
            timer_interrupt_handler
        );

        idt[
            InterruptIndex::Keyboard.as_usize()
        ]
        .set_handler_fn(
            keyboard_interrupt_handler
        );

        idt
    };
}

// --------------------------------------------------
// Initialize IDT
// --------------------------------------------------

pub fn init_idt() {
    IDT.load();

    serial_println!(
        "[IDT] IDT loaded successfully"
    );
}

/// Initialize PICs and unmask IRQ0 (timer) and IRQ1 (keyboard)
pub fn init_pics() {
    unsafe {
        PICS.lock().initialize();
    }
    // Unmask timer (IRQ0) and keyboard (IRQ1) on primary PIC.
    // Bit = 0 means unmasked. 0b1111_1100 unmask IRQ0 and IRQ1.
    unsafe {
        PICS.lock().write_masks(0b1111_1100, 0b1111_1111);
    }
    serial_println!("[ OK ] 8259 PIC initialized, IRQ0/IRQ1 unmasked.");
}

/// Initialize the PS/2 keyboard controller (i8042) so it asserts IRQ1.
pub fn init_keyboard_controller() {
    unsafe {
        // Disable keyboard and mouse while we reconfigure
        kbc_write(0x64, 0xAD); // disable keyboard
        kbc_write(0x64, 0xA7); // disable mouse

        // Flush any pending output
        while kbc_read(0x64) & 1 != 0 {
            let _ = kbc_read(0x60);
        }

        // Read controller configuration byte (command 0x20)
        kbc_write(0x64, 0x20);
        kbc_wait_ibf();
        let config = kbc_read(0x60);

        // Set bit0 (keyboard interrupt enable), clear bit1 (mouse clock),
        // set bit6 (scancode translation to set 1)
        let new_config = (config | 0x01) & !0x02 | 0x40;

        // Write config command then new config byte to data port
        kbc_write(0x64, 0x60);
        kbc_wait_ibf();
        kbc_write(0x60, new_config);

        // Re-enable keyboard
        kbc_write(0x64, 0xAE);

        serial_println!("[ OK ] PS/2 keyboard controller initialized (cfg={:#04x}).", new_config);
    }
}

/// Write a byte to an x86 I/O port (unsafe hardware access).
unsafe fn kbc_write(port: u16, value: u8) {
    Port::<u8>::new(port).write(value);
}

/// Read a byte from an x86 I/O port (unsafe hardware access).
unsafe fn kbc_read(port: u16) -> u8 {
    Port::<u8>::new(port).read()
}

/// Wait for the controller's input buffer to be empty (bit1 of status 0x64).
unsafe fn kbc_wait_ibf() {
    for _ in 0..100_000 {
        if kbc_read(0x64) & 2 == 0 {
            return;
        }
    }
}

// --------------------------------------------------
// Exceptions
// --------------------------------------------------

extern "x86-interrupt"
fn breakpoint_handler(
    _stack_frame: InterruptStackFrame,
) {
    // Diagnostic: trivial output only (no Debug formatting to rule out SSE
    // stack-alignment faults inside core::fmt).
    serial_println!("[EXCEPTION] Breakpoint hit");
}

extern "x86-interrupt"
fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    serial_println!(
        "[PANIC] DOUBLE FAULT: {:#?}",
        stack_frame
    );

    panic!(
        "[ EXCEPTION: DOUBLE FAULT ]\n{:#?}",
        stack_frame
    );
}

// --------------------------------------------------
// Timer
// --------------------------------------------------

extern "x86-interrupt"
fn timer_interrupt_handler(
    _stack_frame: InterruptStackFrame,
) {
    // Call scheduler to switch tasks (if ready)
    unsafe {
        crate::scheduler::on_timer_tick();
    }

    // Send EOI to PIC
    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(
                InterruptIndex::Timer.as_u8()
            );
    }
}

// --------------------------------------------------
// Keyboard
// --------------------------------------------------

extern "x86-interrupt"
fn keyboard_interrupt_handler(
    _stack_frame: InterruptStackFrame,
) {
    let mut port = Port::new(0x60);

    let scancode: u8 = unsafe { port.read() };

    // Enqueue scancode for main-loop processing.
    // If queue is full, drop the scancode (should be rare with 256 capacity).
    let _ = input::SCANCODE_QUEUE.try_push(scancode);

    // Send EOI immediately to avoid masking IRQ1 if processing panics.
    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
}