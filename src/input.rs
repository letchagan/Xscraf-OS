use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicUsize, Ordering};
use spin::Mutex;

/// Maximum number of scancodes that can be queued.
/// Must be a power of 2 for efficient modulo via bitmask.
const QUEUE_CAPACITY: usize = 256;
const QUEUE_MASK: usize = QUEUE_CAPACITY - 1;

/// A lock-free single-producer, single-consumer ring buffer for scancodes.
/// Producer: keyboard interrupt handler (single IRQ, so single producer).
/// Consumer: main loop (single thread).
pub struct ScancodeQueue {
    buffer: UnsafeCell<[u8; QUEUE_CAPACITY]>,
    head: AtomicUsize, // Producer index (where to write next)
    tail: AtomicUsize, // Consumer index (where to read next)
}

// SAFETY: ScancodeQueue is safe to share between threads (interrupt context and main loop)
// because:
// - head is only written by producer (interrupt handler), read by consumer with Acquire ordering
// - tail is only written by consumer (main loop), read by producer with Acquire ordering
// - buffer slots are only accessed by one thread at a time due to head/tail coordination
// - All synchronization is done via atomic operations with proper memory ordering
unsafe impl Sync for ScancodeQueue {}

impl ScancodeQueue {
    pub const fn new() -> Self {
        Self {
            buffer: UnsafeCell::new([0; QUEUE_CAPACITY]),
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
        }
    }

    /// Try to enqueue a scancode. Returns `true` on success, `false` if full.
    /// Safe to call from interrupt context (no locks, no allocation).
    pub fn try_push(&self, scancode: u8) -> bool {
        let head = self.head.load(Ordering::Relaxed);
        let next_head = (head + 1) & QUEUE_MASK;
        let tail = self.tail.load(Ordering::Acquire);

        if next_head == tail {
            // Queue full
            return false;
        }

        // SAFETY: Single producer (interrupt handler), no concurrent writes to same slot.
        // The head index is only modified by this method.
        unsafe {
            (*self.buffer.get())[head] = scancode;
        }
        self.head.store(next_head, Ordering::Release);
        true
    }

    /// Try to dequeue a scancode. Returns `Some(scancode)` on success, `None` if empty.
    /// Safe to call from main loop.
    pub fn try_pop(&self) -> Option<u8> {
        let tail = self.tail.load(Ordering::Relaxed);
        let head = self.head.load(Ordering::Acquire);

        if tail == head {
            // Queue empty
            return None;
        }

        // SAFETY: Single consumer (main loop), no concurrent reads from same slot.
        // The tail index is only modified by this method.
        let scancode = unsafe { (*self.buffer.get())[tail] };
        let next_tail = (tail + 1) & QUEUE_MASK;
        self.tail.store(next_tail, Ordering::Release);
        Some(scancode)
    }

    /// Returns true if the queue is empty.
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        let head = self.head.load(Ordering::Acquire);
        let tail = self.tail.load(Ordering::Acquire);
        head == tail
    }

    /// Returns true if the queue is full.
    #[allow(dead_code)]
    pub fn is_full(&self) -> bool {
        let head = self.head.load(Ordering::Acquire);
        let next_head = (head + 1) & QUEUE_MASK;
        let tail = self.tail.load(Ordering::Acquire);
        next_head == tail
    }
}

/// Global scancode queue instance.
pub static SCANCODE_QUEUE: ScancodeQueue = ScancodeQueue::new();

/// Keyboard state machine for processing scancodes into key events.
/// This runs in the main loop, not in interrupt context.
pub struct KeyboardProcessor {
    keyboard: Mutex<pc_keyboard::Keyboard<pc_keyboard::layouts::Us104Key, pc_keyboard::ScancodeSet1>>,
}

impl KeyboardProcessor {
    pub fn new() -> Self {
        Self {
            keyboard: Mutex::new(pc_keyboard::Keyboard::new(
                pc_keyboard::ScancodeSet1::new(),
                pc_keyboard::layouts::Us104Key,
                pc_keyboard::HandleControl::Ignore,
            )),
        }
    }

    /// Process a single scancode, returning a decoded key if available.
    pub fn process_scancode(&self, scancode: u8) -> Option<pc_keyboard::DecodedKey> {
        let mut keyboard = self.keyboard.lock();
        if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
            keyboard.process_keyevent(key_event)
        } else {
            None
        }
    }
}

lazy_static::lazy_static! {
    pub static ref KEYBOARD_PROCESSOR: KeyboardProcessor = KeyboardProcessor::new();
}