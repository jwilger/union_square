//! Ring buffer using parking_lot RwLock for safe concurrent access

use crate::proxy::ring_buffer::RingBufferStats;
use crate::proxy::types::*;
use parking_lot::RwLock;
use std::sync::atomic::{AtomicU32, AtomicU64, AtomicU8, AtomicUsize, Ordering};

/// Safe slot using RwLock for data protection
pub struct SafeSlot {
    // Atomic fields for lock-free status checking
    state: AtomicU8,
    size: AtomicU32,

    // Protected data behind RwLock (very fast, low contention)
    data: RwLock<SafeSlotData>,
}

#[derive(Clone, Debug)]
struct SafeSlotData {
    timestamp: TimestampNanos,
    request_id: RequestId,
    data: Vec<u8>,
}

impl SafeSlot {
    fn new(slot_size: usize) -> Self {
        Self {
            state: AtomicU8::new(SlotState::Empty as u8),
            size: AtomicU32::new(0),
            data: RwLock::new(SafeSlotData {
                timestamp: TimestampNanos::from(0),
                request_id: RequestId::new(), // Will be overwritten
                data: vec![0; slot_size],
            }),
        }
    }
}

/// Ring buffer using RwLock for safety with minimal performance impact
pub struct RwLockRingBuffer {
    slots: Vec<SafeSlot>,
    slot_count: usize,
    slot_size: usize,
    write_position: AtomicUsize,
    read_position: AtomicUsize,
    overflow_count: AtomicU64,
    successful_writes: AtomicU64,
    successful_reads: AtomicU64,
}

impl RwLockRingBuffer {
    pub fn new(config: &RingBufferConfig) -> Self {
        let calculated_slot_count = *config.buffer_size.as_ref() / *config.slot_size.as_ref();
        let slot_count = calculated_slot_count.next_power_of_two().max(1);

        let slots: Vec<SafeSlot> = (0..slot_count)
            .map(|_| SafeSlot::new(*config.slot_size.as_ref()))
            .collect();

        Self {
            slots,
            slot_count,
            slot_size: *config.slot_size.as_ref(),
            write_position: AtomicUsize::new(0),
            read_position: AtomicUsize::new(0),
            overflow_count: AtomicU64::new(0),
            successful_writes: AtomicU64::new(0),
            successful_reads: AtomicU64::new(0),
        }
    }

    /// Write data (safe with minimal locking)
    pub fn write(&self, request_id: RequestId, data: &[u8]) -> Result<(), u64> {
        let position = self.write_position.fetch_add(1, Ordering::Relaxed);
        let slot_index = position & (self.slot_count - 1);
        let slot = &self.slots[slot_index];

        // Quick atomic check first (no lock needed)
        if slot.state.load(Ordering::Acquire) != SlotState::Empty as u8 {
            let overflow = self.overflow_count.fetch_add(1, Ordering::Relaxed) + 1;
            return Err(overflow);
        }

        // Try to claim the slot atomically
        if slot
            .state
            .compare_exchange(
                SlotState::Empty as u8,
                SlotState::Writing as u8,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_err()
        {
            let overflow = self.overflow_count.fetch_add(1, Ordering::Relaxed) + 1;
            return Err(overflow);
        }

        // Now we have exclusive write access - use RwLock for safety
        {
            let mut slot_data = slot.data.write(); // Fast write lock
            let copy_size = data.len().min(self.slot_size);

            slot_data.request_id = request_id;
            slot_data.timestamp =
                TimestampNanos::from(chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64);
            slot_data.data[..copy_size].copy_from_slice(&data[..copy_size]);

            slot.size.store(copy_size as u32, Ordering::Release);
        } // Write lock released here

        // Mark as ready
        slot.state.store(SlotState::Ready as u8, Ordering::Release);
        self.successful_writes.fetch_add(1, Ordering::Relaxed);

        Ok(())
    }

    /// Read data (safe with minimal locking)
    pub fn read(&self) -> Option<(RequestId, Vec<u8>)> {
        let position = self.read_position.load(Ordering::Relaxed);
        let slot_index = position & (self.slot_count - 1);
        let slot = &self.slots[slot_index];

        // Quick atomic check
        if slot.state.load(Ordering::Acquire) != SlotState::Ready as u8 {
            return None;
        }

        // Try to claim for reading
        if slot
            .state
            .compare_exchange(
                SlotState::Ready as u8,
                SlotState::Reading as u8,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_err()
        {
            return None;
        }

        // Read the data safely
        let result = {
            let slot_data = slot.data.read(); // Fast read lock
            let size = slot.size.load(Ordering::Acquire) as usize;
            let data = slot_data.data[..size].to_vec();
            (slot_data.request_id, data)
        }; // Read lock released here

        // Mark as empty and advance position
        slot.state.store(SlotState::Empty as u8, Ordering::Release);
        self.read_position.fetch_add(1, Ordering::Relaxed);
        self.successful_reads.fetch_add(1, Ordering::Relaxed);

        Some(result)
    }

    pub fn stats(&self) -> RingBufferStats {
        RingBufferStats {
            total_writes: self.successful_writes.load(Ordering::Relaxed),
            total_reads: self.successful_reads.load(Ordering::Relaxed),
            dropped_events: DroppedEventCount::from(self.overflow_count.load(Ordering::Relaxed)),
        }
    }

    pub fn overflow_count(&self) -> u64 {
        self.overflow_count.load(Ordering::Relaxed)
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlotState {
    Empty = 0,
    Writing = 1,
    Ready = 2,
    Reading = 3,
}
