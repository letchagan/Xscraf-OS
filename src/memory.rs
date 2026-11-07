use x86_64::{
    structures::paging::{
        FrameAllocator, Mapper, OffsetPageTable, Page, PageTable, PageTableFlags, PhysFrame, Size4KiB,
        mapper::MapToError,
    },
    PhysAddr, VirtAddr,
};

use crate::{serial_println, ALLOCATOR};
use bootloader::bootinfo::{BootInfo, MemoryRegionType};
use x86_64::instructions::tlb;

/// Physical memory offset for direct mapping (bootloader provides this via `bootinfo`).
pub const PHYSICAL_MEMORY_OFFSET: u64 = 0xFFFF_8000_0000_0000;

/// Kernel heap size: 1 MiB (256 pages)
pub const HEAP_SIZE: usize = 1024 * 1024;
/// Heap start: 1 GiB (chosen to avoid low memory and kernel image)
pub const HEAP_START: usize = 0x4000_0000;

/// A frame allocator that uses the bootloader's memory map to find usable frames.
struct BootInfoFrameAllocator {
    memory_map: &'static bootloader::bootinfo::MemoryMap,
    next_region: usize,
    next_frame: u64,
}

impl BootInfoFrameAllocator {
    fn new(memory_map: &'static bootloader::bootinfo::MemoryMap) -> Self {
        let mut allocator = Self {
            memory_map,
            next_region: 0,
            next_frame: 0,
        };
        allocator.advance_to_usable();
        allocator
    }

    fn advance_to_usable(&mut self) {
        while self.next_region < self.memory_map.len() {
            let region = &self.memory_map[self.next_region];
            if region.region_type == MemoryRegionType::Usable {
                self.next_frame = region.range.start_frame_number;
                return;
            }
            self.next_region += 1;
        }
        // No more usable regions
        self.next_frame = 0;
    }
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        loop {
            if self.next_region >= self.memory_map.len() {
                return None;
            }
            let region = &self.memory_map[self.next_region];
            if self.next_frame < region.range.end_frame_number {
                let frame = PhysFrame::containing_address(PhysAddr::new(self.next_frame * 4096));
                self.next_frame += 1;
                return Some(frame);
            }
            // Move to next region
            self.next_region += 1;
            self.advance_to_usable();
        }
    }
}

/// Initialize paging: identity map first 4 GiB (kernel + low memory), map heap region, enable NXE.
pub fn init(boot_info: &'static BootInfo) {
    use x86_64::registers::control::{Cr0, Cr0Flags, Cr4, Cr4Flags, Efer, EferFlags};

    serial_println!("[MEM] Starting paging init...");

    // Get the bootloader-provided physical memory offset.
    let physical_memory_offset = VirtAddr::new(boot_info.physical_memory_offset);
    serial_println!("[MEM] Physical memory offset: {:#x}", physical_memory_offset.as_u64());

    // SAFETY: We're in early boot, single-threaded, and the page tables are valid.
    let mut mapper = unsafe { OffsetPageTable::new(active_level_4_table(), physical_memory_offset) };
    serial_println!("[MEM] Mapper created");

    // Create a frame allocator that uses the bootloader's memory map.
    let mut frame_allocator = BootInfoFrameAllocator::new(&boot_info.memory_map);
    serial_println!("[MEM] Frame allocator created");

    // Identity map the first 4 GiB (kernel + low memory).
    // This covers kernel code, data, VGA buffer (0xB8000), and early structures.
    // The bootloader already maps most of this, so we skip already-mapped pages.
    serial_println!("[MEM] Identity mapping first 4 GiB...");
    let mut mapped_count = 0;
    for frame_idx in 0..(4 * 1024 * 1024 / 4096) {
        let frame: PhysFrame<Size4KiB> = PhysFrame::containing_address(PhysAddr::new((frame_idx * 4096) as u64));
        let page = Page::containing_address(VirtAddr::new((frame_idx * 4096) as u64));
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_EXECUTE;
        unsafe {
            match mapper.map_to(page, frame, flags, &mut frame_allocator) {
                Ok(_) => {
                    tlb::flush(page.start_address());
                    mapped_count += 1;
                }
                Err(MapToError::PageAlreadyMapped(_)) => {
                    // Page already mapped, skip
                }
                Err(e) => {
                    panic!("identity map failed: {:?}", e);
                }
            }
        }
    }
    serial_println!("[MEM] Identity mapped {} new pages", mapped_count);

    // Map the heap region (1 MiB at HEAP_START) with WRITABLE but NO_EXECUTE.
    // Allocate frames from our frame allocator.
    serial_println!("[MEM] Mapping heap region at {:#x}...", HEAP_START);
    let heap_start_page = Page::containing_address(VirtAddr::new(HEAP_START as u64));
    let heap_pages = HEAP_SIZE / 4096;
    serial_println!("[MEM] Heap pages to map: {}", heap_pages);
    for i in 0..heap_pages {
        if i % 64 == 0 {
            serial_println!("[MEM] Mapping heap page {}/{}", i, heap_pages);
        }
        let page = heap_start_page + u64::try_from(i).unwrap();
        let frame = frame_allocator.allocate_frame().expect("out of frames for heap");
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_EXECUTE;
        unsafe {
            mapper.map_to(page, frame, flags, &mut frame_allocator)
                .expect("heap map failed")
                .flush();
        }
    }
    serial_println!("[MEM] Heap mapped ({} pages)", heap_pages);

    // Enable NXE (No-Execute Enable) in EFER
    serial_println!("[MEM] Enabling NXE...");
    unsafe {
        let mut efer = Efer::read();
        efer.insert(EferFlags::NO_EXECUTE_ENABLE);
        Efer::write(efer);
    }

    // Enable WP (Write Protect) in CR0 to enforce read-only for supervisor
    serial_println!("[MEM] Enabling WP...");
    unsafe {
        let mut cr0 = Cr0::read();
        cr0.insert(Cr0Flags::WRITE_PROTECT);
        Cr0::write(cr0);
    }

    // Enable SMEP (Supervisor Mode Execution Protection) and SMAP in CR4
    serial_println!("[MEM] Enabling SMEP/SMAP...");
    unsafe {
        let mut cr4 = Cr4::read();
        cr4.insert(Cr4Flags::SUPERVISOR_MODE_EXECUTION_PROTECTION);
        cr4.insert(Cr4Flags::SUPERVISOR_MODE_ACCESS_PREVENTION);
        Cr4::write(cr4);
    }

    // Initialize the global heap allocator.
    // SAFETY: Heap region is mapped and exclusive to kernel.
    serial_println!("[MEM] Initializing heap allocator...");
    unsafe {
        ALLOCATOR.lock().init(HEAP_START as *mut u8, HEAP_SIZE);
    }

    serial_println!("[MEM] Paging enabled: identity map 0..4GiB, heap at {:#x}..{:#x}", HEAP_START, HEAP_START + HEAP_SIZE);
}

/// Returns a mutable reference to the active level 4 page table.
/// SAFETY: Caller must ensure the physical memory offset is correct and the page table is valid.
pub unsafe fn active_level_4_table() -> &'static mut PageTable {
    use x86_64::registers::control::Cr3;

    let (level_4_table_frame, _) = Cr3::read();
    let phys = level_4_table_frame.start_address();
    let virt = PHYSICAL_MEMORY_OFFSET + phys.as_u64();
    &mut *VirtAddr::new(virt).as_mut_ptr()
}

/// Allocate a new page table frame from the global allocator (post-init).
#[allow(dead_code)]
pub fn alloc_page_table_frame() -> Option<PhysFrame<Size4KiB>> {
    use alloc::alloc::{alloc_zeroed, Layout};
    use core::ptr::NonNull;

    let layout = Layout::new::<PageTable>();
    let ptr = unsafe { alloc_zeroed(layout) };
    NonNull::new(ptr).map(|ptr| {
        let phys = PhysAddr::new(ptr.as_ptr() as u64);
        PhysFrame::containing_address(phys)
    })
}

/// A frame allocator that uses the global heap allocator.
#[allow(dead_code)]
pub struct GlobalFrameAllocator;

unsafe impl FrameAllocator<Size4KiB> for GlobalFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        alloc_page_table_frame()
    }
}

/// Create a new address space for a user process.
/// Returns the physical frame of the new PML4 table.
#[allow(dead_code)]
pub fn create_address_space() -> PhysFrame<Size4KiB> {
    let frame = alloc_page_table_frame().expect("failed to allocate PML4 frame");
    // Zero the page table
    let phys_addr = frame.start_address();
    let virt_addr = VirtAddr::new(PHYSICAL_MEMORY_OFFSET + phys_addr.as_u64());
    unsafe {
        core::ptr::write_bytes(virt_addr.as_mut_ptr::<u8>(), 0, 512 * 8);
    }
    frame
}

/// Map a page in a given address space (for process memory management).
#[allow(dead_code)]
pub fn map_page(
    mapper: &mut OffsetPageTable,
    page: Page<Size4KiB>,
    frame: PhysFrame<Size4KiB>,
    flags: PageTableFlags,
) {
    let mut frame_allocator = GlobalFrameAllocator;
    unsafe {
        mapper.map_to(page, frame, flags, &mut frame_allocator)
            .expect("map_page failed")
            .flush();
    }
}

/// Unmap a page in a given address space.
#[allow(dead_code)]
pub fn unmap_page(mapper: &mut OffsetPageTable, page: Page<Size4KiB>) {
    mapper.unmap(page).expect("unmap_page failed").1.flush();
}