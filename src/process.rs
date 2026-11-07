use crate::{gdt, memory, serial_println};
use alloc::vec::Vec;
use x86_64::{
    registers::control::{Cr3, Cr3Flags},
    structures::paging::{OffsetPageTable, PageTable, PhysFrame, Size4KiB},
    structures::paging::page_table::PageTableEntry,
    PhysAddr, VirtAddr,
};

/// Process ID type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProcessId(pub u64);

impl ProcessId {
    pub fn new() -> Self {
        static NEXT_ID: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(1);
        ProcessId(NEXT_ID.fetch_add(1, core::sync::atomic::Ordering::Relaxed))
    }
}

/// Process state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    Ready,
    Running,
    Blocked,
    Terminated,
}

/// CPU register state for context switching
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Registers {
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
    pub cs: u64,
    pub ss: u64,
}

/// Process address space
pub struct AddressSpace {
    /// Physical frame of the PML4 table
    pml4_frame: PhysFrame<Size4KiB>,
    /// Kernel virtual address of the PML4 table (for manipulation)
    pml4_virt: VirtAddr,
}

impl AddressSpace {
    /// Create a new address space with kernel mappings
    pub fn new_kernel() -> Self {
        let pml4_frame = memory::create_address_space();
        let pml4_virt = VirtAddr::new(memory::PHYSICAL_MEMORY_OFFSET + pml4_frame.start_address().as_u64());
        
        // Map kernel space (identity map first 4 GiB + heap)
        // This is done by copying kernel page table entries
        let kernel_pml4 = unsafe { &*(memory::active_level_4_table() as *const PageTable) };
        let new_pml4 = unsafe { &mut *(pml4_virt.as_mut_ptr::<PageTable>()) };
        
        // Copy kernel mappings (entries 0..256 for first 2 TiB, and entry 511 for recursive mapping)
        for i in 0..256 {
            new_pml4[i] = kernel_pml4[i].clone();
        }
        // Copy recursive mapping entry if present
        new_pml4[511] = kernel_pml4[511].clone();
        
        Self { pml4_frame, pml4_virt }
    }

    /// Create a new user address space (minimal, no user mappings yet)
    pub fn new_user() -> Self {
        let pml4_frame = memory::create_address_space();
        let pml4_virt = VirtAddr::new(memory::PHYSICAL_MEMORY_OFFSET + pml4_frame.start_address().as_u64());
        
        // Map kernel space (same as kernel address space)
        let kernel_pml4 = unsafe { &*(memory::active_level_4_table() as *const PageTable) };
        let new_pml4 = unsafe { &mut *(pml4_virt.as_mut_ptr::<PageTable>()) };
        
        for i in 0..256 {
            new_pml4[i] = kernel_pml4[i].clone();
        }
        new_pml4[511] = kernel_pml4[511].clone();
        
        Self { pml4_frame, pml4_virt }
    }

    /// Get the physical address of the PML4 table (for CR3)
    pub fn pml4_phys(&self) -> PhysAddr {
        self.pml4_frame.start_address()
    }

    /// Get a mutable reference to the PML4 table for mapping
    pub fn pml4_mut(&mut self) -> &mut PageTable {
        unsafe { &mut *self.pml4_virt.as_mut_ptr::<PageTable>() }
    }

    /// Get an OffsetPageTable for this address space
    pub fn offset_page_table(&mut self) -> OffsetPageTable<'_> {
        unsafe { OffsetPageTable::new(self.pml4_mut(), VirtAddr::new(memory::PHYSICAL_MEMORY_OFFSET)) }
    }

    /// Switch to this address space (load CR3)
    pub fn activate(&self) {
        unsafe {
            Cr3::write(self.pml4_frame, Cr3Flags::empty());
        }
    }
}

/// Process control block
pub struct Process {
    pub id: ProcessId,
    pub state: ProcessState,
    pub registers: Registers,
    pub address_space: AddressSpace,
    pub kernel_stack: VirtAddr,
    pub user_stack: Option<VirtAddr>,
}

impl Process {
    /// Create a new kernel process
    pub fn new_kernel(entry: extern "C" fn() -> !) -> Self {
        let mut process = Self {
            id: ProcessId::new(),
            state: ProcessState::Ready,
            registers: Registers {
                rax: 0, rbx: 0, rcx: 0, rdx: 0,
                rsi: 0, rdi: 0, rbp: 0, rsp: 0,
                r8: 0, r9: 0, r10: 0, r11: 0,
                r12: 0, r13: 0, r14: 0, r15: 0,
                rip: entry as u64,
                rflags: 0x202, // IF bit set
                cs: gdt::kernel_code_selector().0 as u64,
                ss: gdt::kernel_data_selector().0 as u64,
            },
            address_space: AddressSpace::new_kernel(),
            kernel_stack: VirtAddr::new(0), // Will be allocated
            user_stack: None,
        };
        
        // Allocate kernel stack (16 KiB)
        const STACK_SIZE: usize = 16384;
        let stack_frame = memory::alloc_page_table_frame().expect("Failed to allocate kernel stack");
        process.kernel_stack = VirtAddr::new(memory::PHYSICAL_MEMORY_OFFSET + stack_frame.start_address().as_u64() + STACK_SIZE as u64);
        process.registers.rsp = process.kernel_stack.as_u64();
        
        process
    }

    /// Create a new user process
    pub fn new_user(entry: u64, user_stack_top: u64) -> Self {
        let mut process = Self {
            id: ProcessId::new(),
            state: ProcessState::Ready,
            registers: Registers {
                rax: 0, rbx: 0, rcx: 0, rdx: 0,
                rsi: 0, rdi: 0, rbp: 0, rsp: user_stack_top,
                r8: 0, r9: 0, r10: 0, r11: 0,
                r12: 0, r13: 0, r14: 0, r15: 0,
                rip: entry,
                rflags: 0x202, // IF bit set
                cs: gdt::user_code_selector().0 as u64,
                ss: gdt::user_data_selector().0 as u64,
            },
            address_space: AddressSpace::new_user(),
            kernel_stack: VirtAddr::new(0),
            user_stack: Some(VirtAddr::new(user_stack_top)),
        };
        
        // Allocate kernel stack (16 KiB)
        const STACK_SIZE: usize = 16384;
        let stack_frame = memory::alloc_page_table_frame().expect("Failed to allocate kernel stack");
        process.kernel_stack = VirtAddr::new(memory::PHYSICAL_MEMORY_OFFSET + stack_frame.start_address().as_u64() + STACK_SIZE as u64);
        
        process
    }

    /// Get the kernel stack pointer for context switching
    pub fn kernel_stack_ptr(&self) -> u64 {
        self.kernel_stack.as_u64()
    }
}

/// Process scheduler
pub struct Scheduler {
    processes: Vec<Process>,
    current: Option<usize>,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            processes: Vec::new(),
            current: None,
        }
    }

    pub fn add_process(&mut self, process: Process) -> ProcessId {
        let id = process.id;
        self.processes.push(process);
        id
    }

    pub fn schedule(&mut self) -> Option<&mut Process> {
        if self.processes.is_empty() {
            return None;
        }

        // Simple round-robin: find next ready process
        let start = self.current.map(|c| (c + 1) % self.processes.len()).unwrap_or(0);
        let mut idx = start;
        
        loop {
            if self.processes[idx].state == ProcessState::Ready {
                self.current = Some(idx);
                self.processes[idx].state = ProcessState::Running;
                return Some(&mut self.processes[idx]);
            }
            idx = (idx + 1) % self.processes.len();
            if idx == start {
                break;
            }
        }
        
        None
    }

    pub fn yield_current(&mut self) {
        if let Some(idx) = self.current {
            self.processes[idx].state = ProcessState::Ready;
        }
    }

    pub fn block_current(&mut self) {
        if let Some(idx) = self.current {
            self.processes[idx].state = ProcessState::Blocked;
        }
    }

    pub fn terminate_current(&mut self) {
        if let Some(idx) = self.current {
            self.processes[idx].state = ProcessState::Terminated;
        }
    }
}

/// Global scheduler instance
lazy_static::lazy_static! {
    pub static ref SCHEDULER: spin::Mutex<Scheduler> = spin::Mutex::new(Scheduler::new());
}

/// Initialize the process system
pub fn init() {
    serial_println!("[PROCESS] Process system initialized");
}

/// Create and spawn the init process (first user process)
pub fn spawn_init_process(entry: u64, user_stack_top: u64) -> ProcessId {
    let mut scheduler = SCHEDULER.lock();
    let process = Process::new_user(entry, user_stack_top);
    let id = process.id;
    scheduler.add_process(process);
    serial_println!("[PROCESS] Spawned init process (pid={})", id.0);
    id
}

/// Context switch to the next process
pub fn context_switch() {
    // This is a simplified context switch
    // In a real implementation, this would save/restore registers and switch address spaces
    let mut scheduler = SCHEDULER.lock();
    scheduler.yield_current();
    if let Some(next) = scheduler.schedule() {
        next.address_space.activate();
        serial_println!("[PROCESS] Switched to process {}", next.id.0);
    }
}