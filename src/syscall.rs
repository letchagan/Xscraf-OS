use crate::{gdt, serial_println};
use alloc::vec;
use x86_64::{
    registers::model_specific::{Star, LStar, SFMask, KernelGsBase},
    registers::rflags::RFlags,
    VirtAddr,
};

/// Syscall numbers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u64)]
pub enum SyscallNumber {
    /// Exit the current task
    Exit = 0,
    /// Write to a file descriptor
    Write = 1,
    /// Read from a file descriptor
    Read = 2,
    /// Allocate memory
    Alloc = 3,
    /// Free memory
    Free = 4,
    /// Get task ID
    GetPid = 5,
    /// Yield CPU
    Yield = 6,
    /// Create a child task (Xscraf-OS unique)
    CreateTask = 7,
    /// Send message to another task (message-passing IPC)
    SendMessage = 8,
    /// Receive message from queue (message-passing IPC)
    ReceiveMessage = 9,
}

/// Syscall return value
#[allow(dead_code)]
pub type SyscallResult = Result<u64, SyscallError>;

/// Syscall errors
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u64)]
pub enum SyscallError {
    InvalidSyscall = 1,
    InvalidArgument = 2,
    PermissionDenied = 3,
    OutOfMemory = 4,
    NotImplemented = 5,
}

/// Initialize syscall MSRs
pub fn init_syscall() {
    // Set up LSTAR (syscall entry point)
    LStar::write(VirtAddr::new(syscall_entry as *const () as u64));

    // Set up STAR (segment selectors)
    // Star::write(cs_sysret, ss_sysret, cs_syscall, ss_syscall)
    let kernel_cs = gdt::kernel_code_selector();
    let kernel_ds = gdt::kernel_data_selector();
    let user_cs = gdt::user_code_selector();
    let user_ds = gdt::user_data_selector();

    Star::write(user_cs, user_ds, kernel_cs, kernel_ds).expect("Failed to write STAR");

    // Set up SFMASK (RFLAGS mask) - clear IF, TF, DF, NT, RF
    let fmask = RFlags::INTERRUPT_FLAG | RFlags::TRAP_FLAG | RFlags::DIRECTION_FLAG | RFlags::NESTED_TASK | RFlags::RESUME_FLAG;
    SFMask::write(fmask);

    // Set up KernelGsBase (kernel GS base for swapgs)
    // We'll set this per-CPU when we have multiple CPUs
    KernelGsBase::write(VirtAddr::new(0));

    serial_println!("[SYSCALL] Syscall MSRs initialized");
}

/// Syscall entry point (called via syscall instruction)
/// This is a naked function that preserves registers and calls the Rust handler
#[unsafe(naked)]
unsafe extern "C" fn syscall_entry() {
    core::arch::naked_asm!(
        // Save user registers
        "swapgs",
        "mov gs:[0x00], rsp",  // Save user RSP
        "mov rsp, gs:[0x08]",  // Load kernel RSP
        "push r11",            // Save RFLAGS (in R11 per syscall ABI)
        "push rcx",            // Save RIP (in RCX per syscall ABI)
        "push rax",            // Syscall number
        "push rdi",            // Arg 1
        "push rsi",            // Arg 2
        "push rdx",            // Arg 3
        "push r10",            // Arg 4 (was RCX, but clobbered)
        "push r8",             // Arg 5
        "push r9",             // Arg 6
        // Call Rust handler
        "call {syscall_handler}",
        // Restore registers
        "pop r9",
        "pop r8",
        "pop r10",
        "pop rdx",
        "pop rsi",
        "pop rdi",
        "pop rax",             // Return value
        "pop rcx",             // Restore RIP
        "pop r11",             // Restore RFLAGS
        "mov gs:[0x08], rsp",  // Save kernel RSP
        "mov rsp, gs:[0x00]",  // Restore user RSP
        "swapgs",
        "sysretq",
        syscall_handler = sym syscall_handler
    );
}

/// Rust syscall handler
extern "C" fn syscall_handler(
    syscall_num: u64,
    arg1: u64,
    arg2: u64,
    arg3: u64,
    _arg4: u64,
    _arg5: u64,
    _arg6: u64,
) -> u64 {
    let syscall = match SyscallNumber::try_from(syscall_num) {
        Ok(s) => s,
        Err(_) => return SyscallError::InvalidSyscall as u64,
    };

    let result: Result<u64, SyscallError> = match syscall {
        SyscallNumber::Exit => {
            // TODO: Implement process exit
            serial_println!("[SYSCALL] Exit called");
            Ok(0)
        }
        SyscallNumber::Write => {
            // arg1 = fd, arg2 = buf, arg3 = len
            // TODO: Implement write
            serial_println!("[SYSCALL] Write(fd={}, buf={:#x}, len={})", arg1, arg2, arg3);
            Ok(0)
        }
        SyscallNumber::Read => {
            // arg1 = fd, arg2 = buf, arg3 = len
            // TODO: Implement read
            serial_println!("[SYSCALL] Read(fd={}, buf={:#x}, len={})", arg1, arg2, arg3);
            Ok(0)
        }
        SyscallNumber::Alloc => {
            // arg1 = size, arg2 = align
            // TODO: Implement alloc
            serial_println!("[SYSCALL] Alloc(size={}, align={})", arg1, arg2);
            Ok(0)
        }
        SyscallNumber::Free => {
            // arg1 = ptr, arg2 = size
            // TODO: Implement free
            serial_println!("[SYSCALL] Free(ptr={:#x}, size={})", arg1, arg2);
            Ok(0)
        }
        SyscallNumber::GetPid => {
            // TODO: Return current process ID
            serial_println!("[SYSCALL] GetPid");
            Ok(1)
        }
        SyscallNumber::Yield => {
            // TODO: Yield CPU
            serial_println!("[SYSCALL] Yield");
            Ok(0)
        }
        SyscallNumber::CreateTask => {
            // arg1 = entry_point, arg2 = stack_size, arg3 = priority
            serial_println!("[SYSCALL] CreateTask(entry={:#x}, stack_size={}, priority={})", arg1, arg2, arg3);

            let caller_id = crate::task::current_task()
                .unwrap_or(crate::task::TaskId(1));

            // SECURITY CHECK 1: Verify caller has CapCreateTask
            let create_cap = crate::task::Capability::CreateTask;
            if !crate::task::has_capability(caller_id, &create_cap) {
                serial_println!("[SYSCALL] DENIED: Task {} lacks CapCreateTask", caller_id.as_u64());
                return SyscallError::PermissionDenied as u64;
            }

            // VALIDATION: Priority range
            if arg3 > 255 {
                serial_println!("[SYSCALL] ERROR: priority {} > 255", arg3);
                return SyscallError::InvalidArgument as u64;
            }

            // VALIDATION: Stack size range
            if arg2 == 0 || arg2 > 1024 * 1024 {
                serial_println!("[SYSCALL] ERROR: invalid stack size {}", arg2);
                return SyscallError::InvalidArgument as u64;
            }

            // VALIDATION: Entry point in user-space
            if arg1 >= 0x8000_0000_0000 {
                serial_println!("[SYSCALL] ERROR: entry point {:#x} outside user-space", arg1);
                return SyscallError::InvalidArgument as u64;
            }

            // Create child task
            let pml4_frame = crate::memory::create_address_space();
            let stack_ptr = 0x7000_0000u64;

            let child_id = crate::task::create_task(
                Some(caller_id),
                arg1,
                pml4_frame,
                stack_ptr
            );

            // Grant child capabilities (attenuated from parent)
            let send_cap = crate::task::Capability::SendMessage {
                dest_task: caller_id.as_u64(),
                max_size: 4096,
            };
            let exit_cap = crate::task::Capability::Exit;

            crate::task::grant_capability(child_id, send_cap);
            crate::task::grant_capability(child_id, exit_cap);

            serial_println!("[SYSCALL] CreateTask SUCCESS: created task {}", child_id.as_u64());
            Ok(child_id.as_u64())
        }
        SyscallNumber::SendMessage => {
            // arg1 = dest_task_id, arg2 = msg_ptr, arg3 = msg_size
            serial_println!("[SYSCALL] SendMessage(dest={}, msg_ptr={:#x}, size={})", arg1, arg2, arg3);

            let sender_id = crate::task::current_task()
                .unwrap_or(crate::task::TaskId(1));

            // VALIDATION: Message size
            if arg3 == 0 || arg3 > 4096 {
                serial_println!("[SYSCALL] ERROR: message size {} invalid", arg3);
                return SyscallError::InvalidArgument as u64;
            }

            // SECURITY CHECK 1: Verify sender has CapSendMessage to dest
            let send_cap = crate::task::Capability::SendMessage {
                dest_task: arg1,
                max_size: arg3,
            };
            if !crate::task::has_capability(sender_id, &send_cap) {
                serial_println!("[SYSCALL] DENIED: Task {} lacks CapSendMessage({}, {})",
                    sender_id.as_u64(), arg1, arg3);
                return SyscallError::PermissionDenied as u64;
            }

            // SECURITY CHECK 2: Validate user pointer with bounds
            if let Err(e) = crate::userspace::validate_pointer_for_task(arg2, arg3 as usize) {
                serial_println!("[SYSCALL] ERROR: Invalid pointer: {}", e);
                return SyscallError::InvalidArgument as u64;
            }

            // SAFETY: Copy message data from user-space
            let data = match crate::userspace::copy_from_user(arg2, arg3 as usize) {
                Ok(d) => d,
                Err(e) => {
                    serial_println!("[SYSCALL] ERROR: Failed to copy message: {}", e);
                    return SyscallError::InvalidArgument as u64;
                }
            };

            // Create and send message
            if let Ok(msg) = crate::ipc::Message::new(
                sender_id,
                crate::task::TaskId(arg1),
                data
            ) {
                match crate::ipc::send_message(msg) {
                    Ok(()) => {
                        serial_println!("[SYSCALL] SendMessage SUCCESS: {} → {}",
                            sender_id.as_u64(), arg1);
                        Ok(0)
                    }
                    Err(e) => {
                        serial_println!("[SYSCALL] ERROR: Send failed: {}", e);
                        SyscallError::PermissionDenied as u64
                    }
                }
            } else {
                SyscallError::InvalidArgument as u64
            }
        }
        SyscallNumber::ReceiveMessage => {
            // arg1 = buf_ptr, arg2 = buf_size
            serial_println!("[SYSCALL] ReceiveMessage(buf_ptr={:#x}, size={})", arg1, arg2);

            let receiver_id = crate::task::current_task()
                .unwrap_or(crate::task::TaskId(1));

            // VALIDATION: Buffer size
            if arg2 == 0 || arg2 > 4096 {
                serial_println!("[SYSCALL] ERROR: buffer size {} invalid", arg2);
                return SyscallError::InvalidArgument as u64;
            }

            // SECURITY CHECK: Verify receiver has CapReceiveMessage
            let recv_cap = crate::task::Capability::ReceiveMessage {
                max_size: arg2,
            };
            if !crate::task::has_capability(receiver_id, &recv_cap) {
                serial_println!("[SYSCALL] DENIED: Task {} lacks CapReceiveMessage", receiver_id.as_u64());
                return SyscallError::PermissionDenied as u64;
            }

            // SECURITY CHECK: Validate user pointer
            if let Err(e) = crate::userspace::validate_pointer_for_task(arg1, arg2 as usize) {
                serial_println!("[SYSCALL] ERROR: Invalid buffer pointer: {}", e);
                return SyscallError::InvalidArgument as u64;
            }

            // Try to receive message
            if let Some(msg) = crate::ipc::receive_message(receiver_id) {
                let msg_size = msg.data.len();

                // Check if message fits in buffer
                if msg_size > arg2 as usize {
                    serial_println!("[SYSCALL] ERROR: Message {} bytes > buffer {} bytes",
                        msg_size, arg2);
                    return SyscallError::InvalidArgument as u64;
                }

                // SAFETY: Copy message to user-space
                if let Err(e) = crate::userspace::copy_to_user(&msg.data, arg1, msg_size) {
                    serial_println!("[SYSCALL] ERROR: Failed to copy message to user: {}", e);
                    return SyscallError::InvalidArgument as u64;
                }

                serial_println!("[SYSCALL] ReceiveMessage SUCCESS: from task {}, size={}",
                    msg.sender_id.as_u64(), msg_size);
                Ok(msg_size as u64)
            } else {
                // No message waiting (not an error)
                serial_println!("[SYSCALL] No message waiting");
                Ok(0)
            }
        }
    };

    match result {
        Ok(val) => val,
        Err(e) => e as u64,
    }
}

impl TryFrom<u64> for SyscallNumber {
    type Error = ();

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(SyscallNumber::Exit),
            1 => Ok(SyscallNumber::Write),
            2 => Ok(SyscallNumber::Read),
            3 => Ok(SyscallNumber::Alloc),
            4 => Ok(SyscallNumber::Free),
            5 => Ok(SyscallNumber::GetPid),
            6 => Ok(SyscallNumber::Yield),
            7 => Ok(SyscallNumber::CreateTask),
            8 => Ok(SyscallNumber::SendMessage),
            9 => Ok(SyscallNumber::ReceiveMessage),
            _ => Err(()),
        }
    }
}