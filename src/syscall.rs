use crate::{gdt, serial_println};
use x86_64::{
    registers::model_specific::{Star, LStar, SFMask, KernelGsBase},
    registers::rflags::RFlags,
    VirtAddr,
};

/// Syscall numbers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u64)]
pub enum SyscallNumber {
    /// Exit the current process
    Exit = 0,
    /// Write to a file descriptor
    Write = 1,
    /// Read from a file descriptor
    Read = 2,
    /// Allocate memory
    Alloc = 3,
    /// Free memory
    Free = 4,
    /// Get process ID
    GetPid = 5,
    /// Yield CPU
    Yield = 6,
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
            _ => Err(()),
        }
    }
}