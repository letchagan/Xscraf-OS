#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(alloc_error_handler)]

extern crate alloc;

use bootloader::bootinfo::BootInfo;
use bootloader::entry_point;

mod gdt;
mod input;
mod interrupts;
mod memory;
mod serial;
mod syscall;
mod vga_buffer;

use core::alloc::Layout;
use core::panic::PanicInfo;
use pc_keyboard::KeyCode;

entry_point!(kernel_main);

fn is_modifier_key(key: KeyCode) -> bool {
    matches!(key,
        KeyCode::LControl | KeyCode::RControl |
        KeyCode::LShift | KeyCode::RShift |
        KeyCode::LAlt |
        KeyCode::LWin | KeyCode::RWin |
        KeyCode::CapsLock
    )
}

fn enable_sse() {
    use x86_64::registers::control::{Cr0, Cr0Flags, Cr4, Cr4Flags};
    unsafe {
        // Clear EM (bit 2) and set MP (bit 1) in CR0
        let mut cr0 = Cr0::read();
        cr0.remove(Cr0Flags::EMULATE_COPROCESSOR);
        cr0.insert(Cr0Flags::MONITOR_COPROCESSOR);
        Cr0::write(cr0);

        // Set OSFXSR (bit 9) and OSXMMEXCPT_ENABLE (bit 10) in CR4
        let mut cr4 = Cr4::read();
        cr4.insert(Cr4Flags::OSFXSR);
        cr4.insert(Cr4Flags::OSXMMEXCPT_ENABLE);
        Cr4::write(cr4);
    }
}

#[global_allocator]
static ALLOCATOR: linked_list_allocator::LockedHeap = linked_list_allocator::LockedHeap::empty();

#[alloc_error_handler]
fn alloc_error_handler(layout: Layout) -> ! {
    serial_println!("[ALLOC ERROR] layout={:?}", layout);
    panic!("Allocation failed: {:?}", layout);
}

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    serial_println!("========================================");
    serial_println!("       Welcome to AetherOS Kernel       ");
    serial_println!("========================================");

    println!("========================================");
    println!("       Welcome to AetherOS Kernel       ");
    println!("========================================");
    println!();

    enable_sse();
    serial_println!("[ OK ] SSE enabled in CR0/CR4.");

    gdt::init();
    serial_println!("[ OK ] GDT initialized.");
    println!("[ OK ] GDT initialized.");

    syscall::init_syscall();
    serial_println!("[ OK ] Syscall initialized.");
    println!("[ OK ] Syscall initialized.");

    // Initialize paging and heap BEFORE IDT (needed for double-fault stack guard)
    memory::init(boot_info);
    serial_println!("[ OK ] Paging & heap initialized.");
    println!("[ OK ] Paging & heap initialized.");

    interrupts::init_idt();
    serial_println!("[ OK ] IDT initialized.");
    println!("[ OK ] IDT initialized.");

    interrupts::init_pics();
    println!("[ OK ] 8259 PIC initialized.");

    interrupts::init_keyboard_controller();
    println!("[ OK ] PS/2 Keyboard initialized.");

    serial_println!("[ OK ] Enabling CPU interrupts.");
    x86_64::instructions::interrupts::enable();

    println!();
    println!("========================================");
    println!("       AetherOS Keyboard Ready          ");
    println!("========================================");
    println!("Type something (interrupt-driven):");
    println!();

    serial_println!("[READY] AetherOS awaiting keyboard interrupts");

    loop {
        // Process any pending scancodes from the interrupt queue
        while let Some(scancode) = input::SCANCODE_QUEUE.try_pop() {
            if let Some(key) = input::KEYBOARD_PROCESSOR.process_scancode(scancode) {
                match key {
                    pc_keyboard::DecodedKey::Unicode(character) => {
                        serial_println!("[KEYBOARD] Character: {}", character);
                        print!("{}", character);
                    }
                    pc_keyboard::DecodedKey::RawKey(key) => {
                        // Log raw keys for debugging, but only print non-modifier keys to VGA
                        serial_println!("[KEYBOARD] Raw key: {:?}", key);
                        // Skip modifier keys (Ctrl, Shift, Alt, etc.) - they don't produce visible output
                        let is_modifier = is_modifier_key(key);
                        if !is_modifier {
                            print!("{:?}", key);
                        }
                    }
                }
            }
        }

        x86_64::instructions::hlt();
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    serial_println!("[PANIC] {}", info);
    println!();
    println!("KERNEL PANIC!");
    println!("{}", info);

    loop {
        x86_64::instructions::hlt();
    }
}
