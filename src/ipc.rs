/// Kairos-11 Message-Passing IPC
///
/// No shared memory: all inter-task communication via kernel-copied messages.
/// Each task has a message queue. Sender calls SEND_MESSAGE syscall; receiver calls RECEIVE_MESSAGE.

use alloc::collections::VecDeque;
use alloc::vec::Vec;
use spin::Mutex;
use crate::task::TaskId;

/// Maximum message size (4 KiB)
pub const MAX_MESSAGE_SIZE: usize = 4096;

/// Message envelope
#[derive(Debug, Clone)]
pub struct Message {
    pub sender_id: TaskId,
    pub receiver_id: TaskId,
    pub data: Vec<u8>,
    pub timestamp_ms: u64,
}

impl Message {
    pub fn new(sender_id: TaskId, receiver_id: TaskId, data: Vec<u8>) -> Result<Self, &'static str> {
        if data.len() > MAX_MESSAGE_SIZE {
            return Err("Message too large");
        }
        Ok(Message {
            sender_id,
            receiver_id,
            data,
            timestamp_ms: 0,
        })
    }
}

/// Per-task message queue
#[derive(Debug)]
pub struct MessageQueue {
    task_id: TaskId,
    messages: VecDeque<Message>,
}

impl MessageQueue {
    pub fn new(task_id: TaskId) -> Self {
        MessageQueue {
            task_id,
            messages: VecDeque::new(),
        }
    }

    pub fn enqueue(&mut self, message: Message) -> Result<(), &'static str> {
        if self.messages.len() >= 256 {
            return Err("Message queue full");
        }
        self.messages.push_back(message);
        Ok(())
    }

    pub fn dequeue(&mut self) -> Option<Message> {
        self.messages.pop_front()
    }

    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    pub fn len(&self) -> usize {
        self.messages.len()
    }
}

/// Global IPC manager
pub struct IPCManager {
    queues: alloc::collections::BTreeMap<u64, MessageQueue>,
}

impl IPCManager {
    pub fn new() -> Self {
        IPCManager {
            queues: alloc::collections::BTreeMap::new(),
        }
    }

    pub fn create_queue(&mut self, task_id: TaskId) {
        self.queues.insert(task_id.as_u64(), MessageQueue::new(task_id));
    }

    pub fn send_message(&mut self, message: Message) -> Result<(), &'static str> {
        if let Some(queue) = self.queues.get_mut(&message.receiver_id.as_u64()) {
            queue.enqueue(message)
        } else {
            Err("Destination task has no message queue")
        }
    }

    pub fn receive_message(&mut self, task_id: TaskId) -> Option<Message> {
        if let Some(queue) = self.queues.get_mut(&task_id.as_u64()) {
            queue.dequeue()
        } else {
            None
        }
    }

    pub fn get_message_count(&self, task_id: TaskId) -> usize {
        self.queues
            .get(&task_id.as_u64())
            .map(|q| q.len())
            .unwrap_or(0)
    }
}

/// Global IPC instance
static IPC_MANAGER: Mutex<Option<IPCManager>> = Mutex::new(None);

pub fn init() {
    let mut ipc = IPC_MANAGER.lock();
    *ipc = Some(IPCManager::new());
    crate::serial_println!("[IPC] Message-passing IPC initialized");
}

pub fn create_queue(task_id: TaskId) {
    let mut ipc = IPC_MANAGER.lock();
    if let Some(manager) = ipc.as_mut() {
        manager.create_queue(task_id);
    }
}

pub fn send_message(message: Message) -> Result<(), &'static str> {
    let mut ipc = IPC_MANAGER.lock();
    if let Some(manager) = ipc.as_mut() {
        manager.send_message(message)
    } else {
        Err("IPC not initialized")
    }
}

pub fn receive_message(task_id: TaskId) -> Option<Message> {
    let mut ipc = IPC_MANAGER.lock();
    if let Some(manager) = ipc.as_mut() {
        manager.receive_message(task_id)
    } else {
        None
    }
}

pub fn message_count(task_id: TaskId) -> usize {
    let ipc = IPC_MANAGER.lock();
    ipc.as_ref()
        .map(|m| m.get_message_count(task_id))
        .unwrap_or(0)
}
