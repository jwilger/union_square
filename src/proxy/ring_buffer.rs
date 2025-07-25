//! Ring buffer implementation for audit path handoff

use crate::proxy::types::*;
use std::sync::atomic::{AtomicU32, AtomicU64, AtomicU8, Ordering};
use uuid::Uuid;

/// Slot states for the ring buffer
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlotState {
    Empty = 0,
    Writing = 1,
    Ready = 2,
    Reading = 3,
}

impl From<u8> for SlotState {
    fn from(value: u8) -> Self {
        match value {
            0 => SlotState::Empty,
            1 => SlotState::Writing,
            2 => SlotState::Ready,
            3 => SlotState::Reading,
            _ => SlotState::Empty,
        }
    }
}

/// A single slot in the ring buffer
#[repr(C, align(64))] // Cache line alignment
pub struct Slot {
    state: AtomicU8,
    size: AtomicU32,
    timestamp: u64,
    request_id: [u8; 16], // UUID bytes
    data: Vec<u8>,
}

impl Slot {
    fn new(slot_size: usize) -> Self {
        Self {
            state: AtomicU8::new(SlotState::Empty as u8),
            size: AtomicU32::new(0),
            timestamp: 0,
            request_id: [0; 16],
            data: vec![0; slot_size],
        }
    }
}

/// Lock-free ring buffer for audit event handoff
pub struct RingBuffer {
    slots: Vec<Slot>,
    slot_count: usize,
    slot_size: usize,
    write_position: AtomicU64,
    read_position: AtomicU64,
    overflow_count: AtomicU64,
}

impl RingBuffer {
    /// Create a new ring buffer with the given configuration
    pub fn new(config: &RingBufferConfig) -> Self {
        let slot_count = config.buffer_size / config.slot_size;
        // Ensure power of 2 for efficient modulo
        let slot_count = slot_count.next_power_of_two();

        let slots: Vec<Slot> = (0..slot_count)
            .map(|_| Slot::new(config.slot_size))
            .collect();

        Self {
            slots,
            slot_count,
            slot_size: config.slot_size,
            write_position: AtomicU64::new(0),
            read_position: AtomicU64::new(0),
            overflow_count: AtomicU64::new(0),
        }
    }

    /// Write data to the ring buffer (hot path operation)
    pub fn write(&self, request_id: RequestId, data: &[u8]) -> Result<(), u64> {
        // Get next write position
        let position = self.write_position.fetch_add(1, Ordering::Relaxed);
        let slot_index = (position & (self.slot_count as u64 - 1)) as usize;
        let slot = &self.slots[slot_index];

        // Try to claim the slot
        let current_state = slot.state.load(Ordering::Acquire);
        if current_state != SlotState::Empty as u8 {
            // Slot not available, increment overflow counter
            let overflow = self.overflow_count.fetch_add(1, Ordering::Relaxed) + 1;
            return Err(overflow);
        }

        // Try to transition to Writing state
        match slot.state.compare_exchange(
            SlotState::Empty as u8,
            SlotState::Writing as u8,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => {
                // We have exclusive access to the slot
                // Copy data (with truncation if needed)
                let copy_size = data.len().min(self.slot_size);
                unsafe {
                    // Safe because we have exclusive access
                    let slot_ptr = slot as *const Slot as *mut Slot;
                    (&mut (*slot_ptr).data)[..copy_size].copy_from_slice(&data[..copy_size]);
                    (*slot_ptr).timestamp =
                        chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64;
                    (*slot_ptr)
                        .request_id
                        .copy_from_slice(request_id.as_ref().as_bytes());
                }

                // Store actual size
                slot.size.store(copy_size as u32, Ordering::Release);

                // Mark as ready for reading
                slot.state.store(SlotState::Ready as u8, Ordering::Release);

                Ok(())
            }
            Err(_) => {
                // Someone else got the slot, count as overflow
                let overflow = self.overflow_count.fetch_add(1, Ordering::Relaxed) + 1;
                Err(overflow)
            }
        }
    }

    /// Read the next available slot (audit path operation)
    pub fn read(&self) -> Option<(RequestId, Vec<u8>)> {
        let position = self.read_position.load(Ordering::Relaxed);
        let slot_index = (position & (self.slot_count as u64 - 1)) as usize;
        let slot = &self.slots[slot_index];

        // Check if slot is ready
        let current_state = slot.state.load(Ordering::Acquire);
        if current_state != SlotState::Ready as u8 {
            return None;
        }

        // Try to transition to Reading state
        match slot.state.compare_exchange(
            SlotState::Ready as u8,
            SlotState::Reading as u8,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => {
                // We have exclusive access to read
                let size = slot.size.load(Ordering::Acquire) as usize;
                let data = slot.data[..size].to_vec();
                let request_id_bytes = slot.request_id;
                let request_id =
                    unsafe { RequestId::new_unchecked(Uuid::from_bytes(request_id_bytes)) };

                // Mark as empty for reuse
                slot.state.store(SlotState::Empty as u8, Ordering::Release);

                // Advance read position
                self.read_position.fetch_add(1, Ordering::Relaxed);

                Some((request_id, data))
            }
            Err(_) => {
                // Someone else is reading this slot
                None
            }
        }
    }

    /// Get the current overflow count
    pub fn overflow_count(&self) -> u64 {
        self.overflow_count.load(Ordering::Relaxed)
    }
}
