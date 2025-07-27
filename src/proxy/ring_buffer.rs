//! Ring buffer implementation for audit path handoff
//!
//! This implementation provides a lock-free Multi-Producer Single-Consumer (MPSC) ring buffer
//! designed for Union Square's dual-path architecture. It maintains strict safety invariants
//! through atomic state coordination while providing <1μs write latency.

use crate::proxy::types::*;
use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicU32, AtomicU64, AtomicU8, Ordering};
use uuid::Uuid;

/// Slot states for the ring buffer state machine
///
/// State transitions: Empty → Writing → Ready → Reading → Empty
/// This state machine ensures memory safety for concurrent access.
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
///
/// # Safety Invariants
///
/// - Only the thread that successfully transitions state from Empty to Writing may access
///   the non-atomic fields (timestamp, request_id, data)
/// - Non-atomic fields must only be accessed when holding the Writing state
/// - State transitions must follow: Empty → Writing → Ready → Reading → Empty
/// - Cache line alignment prevents false sharing between slots
#[repr(C, align(64))] // Cache line alignment
pub struct Slot {
    state: AtomicU8,
    size: AtomicU32,
    timestamp: UnsafeCell<TimestampNanos>,
    request_id: UnsafeCell<[u8; UUID_SIZE_BYTES]>, // UUID bytes
    data: UnsafeCell<Vec<u8>>,
}

// SAFETY: Slot is safe to send between threads because:
// - All access to non-atomic fields is coordinated via the atomic state field
// - The state machine guarantees exclusive access during the Writing state
// - Proper memory ordering ensures visibility of non-atomic writes
unsafe impl Send for Slot {}
unsafe impl Sync for Slot {}

impl Slot {
    fn new(slot_size: SlotSize) -> Self {
        Self {
            state: AtomicU8::new(SlotState::Empty as u8),
            size: AtomicU32::new(0),
            timestamp: UnsafeCell::new(TimestampNanos::from(0)),
            request_id: UnsafeCell::new([0; UUID_SIZE_BYTES]),
            data: UnsafeCell::new(vec![0; *slot_size.as_ref()]),
        }
    }
}

/// Statistics about ring buffer usage
pub struct RingBufferStats {
    pub total_writes: u64,
    pub total_reads: u64,
    pub dropped_events: DroppedEventCount,
}

/// Lock-free ring buffer for audit event handoff
///
/// This implementation follows the design specified in ADR-0009 and supports
/// the chunked payload system described in ADR-0017. It maintains the fail-when-busy
/// semantics required for proper backpressure and data integrity.
///
/// # Performance Characteristics
///
/// - <1μs write latency (when slots are available)
/// - Zero heap allocations after initialization
/// - Lock-free concurrent writes from multiple producers
/// - Sequential reads from single consumer
///
/// # Safety
///
/// Uses unsafe code for performance-critical path, but maintains strict safety invariants
/// through atomic state coordination. All unsafe operations are documented and bounded.
pub struct RingBuffer {
    slots: Vec<Slot>,
    slot_count: SlotCount,
    slot_size: SlotSize,
    write_position: AtomicU64,
    read_position: AtomicU64,
    overflow_count: AtomicU64,
    successful_writes: AtomicU64,
    successful_reads: AtomicU64,
}

impl RingBuffer {
    /// Statistics about ring buffer usage
    pub fn stats(&self) -> RingBufferStats {
        RingBufferStats {
            total_writes: self.successful_writes.load(Ordering::Relaxed),
            total_reads: self.successful_reads.load(Ordering::Relaxed),
            dropped_events: DroppedEventCount::from(self.overflow_count.load(Ordering::Relaxed)),
        }
    }

    /// Create a new ring buffer with the given configuration
    pub fn new(config: &RingBufferConfig) -> Self {
        let calculated_slot_count = *config.buffer_size.as_ref() / *config.slot_size.as_ref();
        // Ensure power of 2 for efficient modulo, but don't exceed calculated count
        let mut slot_count_value = calculated_slot_count.next_power_of_two();

        // If rounding up to power of 2 would exceed buffer capacity, round down
        if slot_count_value > calculated_slot_count {
            slot_count_value /= 2;
        }

        // Ensure at least 1 slot
        slot_count_value = slot_count_value.max(1);

        let slot_count =
            SlotCount::try_new(slot_count_value).expect("calculated slot count should be valid");

        let slots = (0..*slot_count.as_ref())
            .map(|_| Slot::new(config.slot_size))
            .collect();

        Self {
            slots,
            slot_count,
            slot_size: config.slot_size,
            write_position: AtomicU64::new(0),
            read_position: AtomicU64::new(0),
            overflow_count: AtomicU64::new(0),
            successful_writes: AtomicU64::new(0),
            successful_reads: AtomicU64::new(0),
        }
    }

    /// Write data to the ring buffer
    ///
    /// Returns Ok(()) on success, or Err(overflow_count) if the buffer is full.
    /// This maintains the fail-when-busy semantics required by ADR-0009 for proper
    /// backpressure and prevents corruption of multi-slot chunked payloads (ADR-0017).
    ///
    /// # Performance
    ///
    /// - <1μs when slot is available
    /// - Zero heap allocations
    /// - Lock-free operation
    ///
    /// # Data Handling
    ///
    /// - Data larger than slot size is truncated
    /// - Timestamps are captured at write time
    /// - Request IDs are stored for correlation
    pub fn write(&self, request_id: RequestId, data: &[u8]) -> Result<(), u64> {
        // Get next write position
        let position = self.write_position.fetch_add(1, Ordering::Relaxed);
        let slot_index = (position & (*self.slot_count.as_ref() as u64 - 1)) as usize;
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

                // Safety check in debug mode
                debug_assert_eq!(
                    slot.state.load(Ordering::Acquire),
                    SlotState::Writing as u8,
                    "State should be Writing after successful CAS"
                );

                // Copy data (with truncation if needed)
                let copy_size = data.len().min(*self.slot_size.as_ref());

                // SAFETY: Exclusive access guaranteed by successful CAS transition from Empty to Writing.
                // - We successfully transitioned the state from Empty to Writing using a CAS operation
                // - Only one thread can win the compare_exchange operation, ensuring exclusive access
                // - The state machine guarantees no other thread accesses these fields until Ready state
                // - The data Vec is pre-allocated, so no reallocation or memory race occurs
                unsafe {
                    let data_ref = &mut *slot.data.get();
                    data_ref[..copy_size].copy_from_slice(&data[..copy_size]);

                    let timestamp_value =
                        chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64;
                    *slot.timestamp.get() = TimestampNanos::from(timestamp_value);

                    (*slot.request_id.get()).copy_from_slice(request_id.as_ref().as_bytes());
                }

                // Store actual size
                slot.size.store(copy_size as u32, Ordering::Release);

                // Mark as ready for reading
                slot.state.store(SlotState::Ready as u8, Ordering::Release);

                // Increment successful writes counter
                self.successful_writes.fetch_add(1, Ordering::Relaxed);

                Ok(())
            }
            Err(_) => {
                // Someone else got the slot, count as overflow
                let overflow = self.overflow_count.fetch_add(1, Ordering::Relaxed) + 1;
                Err(overflow)
            }
        }
    }

    /// Read the next available entry from the ring buffer
    ///
    /// Returns Some((request_id, data)) if data is available, None otherwise.
    /// This follows the single-consumer pattern - only one thread should call read().
    ///
    /// # Safety
    ///
    /// This method is designed for single-consumer use. Multiple concurrent readers
    /// would violate safety invariants and could cause data corruption.
    pub fn read(&self) -> Option<(RequestId, Vec<u8>)> {
        let read_pos = self.read_position.load(Ordering::Relaxed);
        let slot_index = (read_pos & (*self.slot_count.as_ref() as u64 - 1)) as usize;
        let slot = &self.slots[slot_index];

        // Check if slot is ready for reading
        if slot.state.load(Ordering::Acquire) != SlotState::Ready as u8 {
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
                // We have exclusive access to read the slot

                // Safety check in debug mode
                debug_assert_eq!(
                    slot.state.load(Ordering::Acquire),
                    SlotState::Reading as u8,
                    "State should be Reading after successful CAS"
                );

                let size = slot.size.load(Ordering::Acquire) as usize;

                // SAFETY: Exclusive access guaranteed by successful CAS transition from Ready to Reading.
                // - The CAS operation successfully transitioned from Ready to Reading, ensuring only
                //   this thread can proceed with reading. Other threads would fail the CAS
                // - The writer completed and marked the slot Ready, all writes are finalized
                // - The state machine enforces no other thread accesses during Reading state
                // - Memory ordering with Acquire ensures we observe all writes from Writing phase
                let (request_id, data) = unsafe {
                    let request_id_bytes = *slot.request_id.get();
                    let uuid = Uuid::from_bytes(request_id_bytes);
                    let request_id = RequestId::try_new(uuid)
                        .expect("UUID from ring buffer should always be valid v7");

                    let data_ref = &*slot.data.get();
                    let data = data_ref[..size].to_vec();

                    (request_id, data)
                };

                // Mark slot as empty for reuse
                slot.state.store(SlotState::Empty as u8, Ordering::Release);

                // Advance read position
                self.read_position.store(read_pos + 1, Ordering::Relaxed);

                // Increment successful reads counter
                self.successful_reads.fetch_add(1, Ordering::Relaxed);

                Some((request_id, data))
            }
            Err(_) => {
                // Someone else is reading or slot became unavailable
                None
            }
        }
    }

    /// Get the current overflow count
    ///
    /// Returns the number of write attempts that failed due to unavailable slots.
    /// This provides visibility into backpressure and helps with capacity planning.
    pub fn overflow_count(&self) -> u64 {
        self.overflow_count.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proxy::types::{RequestId, RingBufferConfig};

    #[test]
    fn test_ring_buffer_creation() {
        let config = RingBufferConfig {
            buffer_size: BufferSize::try_new(1024 * 1024).expect("valid size"), // 1MB
            slot_size: SlotSize::try_new(1024).expect("valid size"),            // 1KB
        };

        let buffer = RingBuffer::new(&config);

        // Should start with zero overflow
        assert_eq!(buffer.overflow_count(), 0);

        let stats = buffer.stats();
        assert_eq!(stats.total_writes, 0);
        assert_eq!(stats.total_reads, 0);
    }

    #[test]
    fn test_write_and_read_single_event() {
        let config = RingBufferConfig {
            buffer_size: BufferSize::try_new(1024 * 1024).expect("valid size"),
            slot_size: SlotSize::try_new(1024).expect("valid size"),
        };

        let buffer = RingBuffer::new(&config);
        let request_id = RequestId::new();
        let data = b"test event data";

        // Write should succeed
        let write_result = buffer.write(request_id, data);
        assert!(write_result.is_ok());

        // Read should return the same data
        let read_result = buffer.read();
        assert!(read_result.is_some());

        let (read_id, read_data) = read_result.unwrap();
        assert_eq!(read_id.as_ref(), request_id.as_ref());
        assert_eq!(&read_data[..], data);
    }

    #[test]
    fn test_overflow_when_buffer_full() {
        let config = RingBufferConfig {
            buffer_size: BufferSize::try_new(256).expect("valid size"), // Very small buffer
            slot_size: SlotSize::try_new(64).expect("valid size"),
        };

        let buffer = RingBuffer::new(&config);
        let capacity = 256 / 64; // 4 slots

        // Fill the buffer
        for i in 0..capacity {
            let id = RequestId::new();
            let data = format!("event {i}");
            assert!(buffer.write(id, data.as_bytes()).is_ok());
        }

        // Next write should fail with overflow
        let id = RequestId::new();
        let result = buffer.write(id, b"overflow event");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), 1);
        assert_eq!(buffer.overflow_count(), 1);
    }

    #[test]
    fn test_data_truncation() {
        let config = RingBufferConfig {
            buffer_size: BufferSize::try_new(1024).expect("valid size"),
            slot_size: SlotSize::try_new(64).expect("valid size"), // Small slots for testing
        };

        let buffer = RingBuffer::new(&config);
        let request_id = RequestId::new();
        let large_data = vec![42u8; 128]; // Larger than slot size

        // Write should succeed (data will be truncated)
        assert!(buffer.write(request_id, &large_data).is_ok());

        // Read should return truncated data
        let (read_id, read_data) = buffer.read().expect("Should read event");
        assert_eq!(read_id.as_ref(), request_id.as_ref());
        assert_eq!(read_data.len(), 64); // Truncated to slot size
        assert_eq!(&read_data[..], &large_data[..64]);
    }

    #[test]
    fn test_concurrent_writes() {
        use std::sync::Arc;
        use std::thread;

        let config = RingBufferConfig {
            buffer_size: BufferSize::try_new(1024 * 1024).expect("valid size"),
            slot_size: SlotSize::try_new(1024).expect("valid size"),
        };

        let buffer = Arc::new(RingBuffer::new(&config));
        let thread_count = 10;
        let writes_per_thread = 100;

        let handles: Vec<_> = (0..thread_count)
            .map(|thread_id| {
                let buffer_clone = Arc::clone(&buffer);
                thread::spawn(move || {
                    let mut successful_writes = 0;
                    for i in 0..writes_per_thread {
                        let id = RequestId::new();
                        let data = format!("thread {thread_id} event {i}");
                        if buffer_clone.write(id, data.as_bytes()).is_ok() {
                            successful_writes += 1;
                        }
                    }
                    successful_writes
                })
            })
            .collect();

        let total_successful: usize = handles.into_iter().map(|h| h.join().unwrap()).sum();

        // Should have high success rate with large buffer
        assert!(total_successful > (thread_count * writes_per_thread) / 2);
    }

    #[test]
    fn test_empty_read() {
        let config = RingBufferConfig::default();
        let buffer = RingBuffer::new(&config);

        // Reading from empty buffer should return None
        assert!(buffer.read().is_none());
    }

    #[test]
    fn test_stats_accuracy() {
        let config = RingBufferConfig {
            buffer_size: BufferSize::try_new(1024 * 1024).expect("valid size"), // Larger buffer
            slot_size: SlotSize::try_new(256).expect("valid size"),
        };

        let buffer = RingBuffer::new(&config);

        // Write some events
        for i in 0..5 {
            let id = RequestId::new();
            let data = format!("event {i}");
            let result = buffer.write(id, data.as_bytes());
            assert!(result.is_ok(), "Write {i} failed: {result:?}");
        }

        // Read some events
        for i in 0..3 {
            let result = buffer.read();
            assert!(result.is_some(), "Read {i} failed: should have data");
        }

        let stats = buffer.stats();
        assert_eq!(stats.total_writes, 5);
        assert_eq!(stats.total_reads, 3);
        assert_eq!(*stats.dropped_events.as_ref(), 0);
    }
}

#[cfg(test)]
#[path = "ring_buffer_tests.rs"]
mod ring_buffer_tests;
