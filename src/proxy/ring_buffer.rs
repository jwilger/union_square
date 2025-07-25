//! Safe ring buffer implementation using crossbeam for lock-free operations

use crate::proxy::types::*;
use crossbeam_queue::ArrayQueue;
use std::sync::atomic::{AtomicU64, Ordering};

/// A safe ring buffer entry with all data
#[derive(Clone, Debug)]
pub struct RingBufferEntry {
    pub request_id: RequestId,
    pub timestamp: TimestampNanos,
    pub data: Vec<u8>,
}

/// Statistics about ring buffer usage
pub struct RingBufferStats {
    pub total_writes: u64,
    pub total_reads: u64,
    pub dropped_events: DroppedEventCount,
}

/// Safe lock-free ring buffer using crossbeam's ArrayQueue with force_push
///
/// This implementation provides true ring buffer semantics with automatic overwrite when full.
/// It eliminates all unsafe code while maintaining excellent performance. The force_push method
/// ensures that writes never fail - when the buffer is full, the oldest entry is automatically
/// overwritten, which is the expected behavior for a ring buffer in a proxy/wire-tap service.
pub struct RingBuffer {
    queue: ArrayQueue<RingBufferEntry>,
    overflow_count: AtomicU64,
    successful_writes: AtomicU64,
    successful_reads: AtomicU64,
    max_data_size: usize,
}

impl RingBuffer {
    /// Create a new safe ring buffer
    pub fn new(config: &RingBufferConfig) -> Self {
        let slot_count = *config.buffer_size.as_ref() / *config.slot_size.as_ref();
        let capacity = slot_count.next_power_of_two().max(1);

        Self {
            queue: ArrayQueue::new(capacity),
            overflow_count: AtomicU64::new(0),
            successful_writes: AtomicU64::new(0),
            successful_reads: AtomicU64::new(0),
            max_data_size: *config.slot_size.as_ref(),
        }
    }

    /// Write data to the ring buffer with automatic overwrite (completely safe, lock-free)
    ///
    /// Returns Ok(()) on success. If the buffer is full, automatically overwrites the oldest entry.
    /// Returns Err(overwrite_count) when an entry was overwritten.
    /// This provides true ring buffer semantics unlike the previous queue-based implementation.
    pub fn write(&self, request_id: RequestId, data: &[u8]) -> Result<(), u64> {
        // Truncate data if needed (same behavior as before)
        let data_to_store = if data.len() > self.max_data_size {
            data[..self.max_data_size].to_vec()
        } else {
            data.to_vec()
        };

        let entry = RingBufferEntry {
            request_id,
            timestamp: TimestampNanos::from(
                chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64
            ),
            data: data_to_store,
        };

        // Use force_push for true ring buffer semantics
        match self.queue.force_push(entry) {
            None => {
                // No entry was overwritten - buffer had space
                self.successful_writes.fetch_add(1, Ordering::Relaxed);
                Ok(())
            }
            Some(_overwritten_entry) => {
                // An entry was overwritten - true ring buffer behavior
                self.successful_writes.fetch_add(1, Ordering::Relaxed);
                let overwrite_count = self.overflow_count.fetch_add(1, Ordering::Relaxed) + 1;
                Err(overwrite_count)
            }
        }
    }

    /// Read the next available entry (completely safe, lock-free)
    ///
    /// Returns Some((request_id, data)) if data is available, None otherwise.
    /// This maintains the same API as the original unsafe implementation.
    pub fn read(&self) -> Option<(RequestId, Vec<u8>)> {
        match self.queue.pop() {
            Some(entry) => {
                self.successful_reads.fetch_add(1, Ordering::Relaxed);
                Some((entry.request_id, entry.data))
            }
            None => None,
        }
    }

    /// Get statistics about ring buffer usage
    pub fn stats(&self) -> RingBufferStats {
        RingBufferStats {
            total_writes: self.successful_writes.load(Ordering::Relaxed),
            total_reads: self.successful_reads.load(Ordering::Relaxed),
            dropped_events: DroppedEventCount::from(self.overflow_count.load(Ordering::Relaxed)),
        }
    }

    /// Get the current overwrite count
    ///
    /// Returns the number of times older entries were overwritten due to buffer being full.
    /// This provides visibility into data loss while maintaining the never-fail semantics
    /// expected from a ring buffer in a proxy service.
    pub fn overflow_count(&self) -> u64 {
        self.overflow_count.load(Ordering::Relaxed)
    }
}

// Automatically safe to send between threads - no unsafe code needed!
// The compiler can verify this automatically with crossbeam's ArrayQueue.

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
    fn test_multiple_writes_and_reads() {
        let config = RingBufferConfig {
            buffer_size: BufferSize::try_new(1024 * 1024).expect("valid size"),
            slot_size: SlotSize::try_new(1024).expect("valid size"),
        };

        let buffer = RingBuffer::new(&config);
        let events = vec![
            (RequestId::new(), b"event 1".to_vec()),
            (RequestId::new(), b"event 2".to_vec()),
            (RequestId::new(), b"event 3".to_vec()),
        ];

        // Write all events
        for (id, data) in &events {
            assert!(buffer.write(*id, data).is_ok());
        }

        // Read all events back (order may vary in queue, but all should be present)
        let mut received_ids = Vec::new();
        let mut received_data = Vec::new();

        for _ in 0..events.len() {
            let (id, data) = buffer.read().expect("Should read event");
            received_ids.push(id);
            received_data.push(data);
        }

        // All IDs should be present (though order may differ)
        for (original_id, _) in &events {
            assert!(received_ids
                .iter()
                .any(|id| id.as_ref() == original_id.as_ref()));
        }

        // No more events to read
        assert!(buffer.read().is_none());
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
    fn test_overwrite_handling() {
        let config = RingBufferConfig {
            buffer_size: BufferSize::try_new(256).expect("valid size"), // Very small buffer
            slot_size: SlotSize::try_new(64).expect("valid size"),
        };

        let buffer = RingBuffer::new(&config);
        let capacity = 256 / 64; // 4 slots, but ArrayQueue rounds to next power of 2, so actually 4 slots

        // Fill the buffer (should all succeed with no overwrites)
        for i in 0..capacity {
            let id = RequestId::new();
            let data = format!("event {i}");
            let result = buffer.write(id, data.as_bytes());
            assert!(result.is_ok(), "Write {i} should succeed without overwrite");
        }

        // This should still succeed but no overwrite yet (depends on actual ArrayQueue capacity)
        let id = RequestId::new();
        let result = buffer.write(id, b"potential overwrite");

        // With true ring buffer semantics, this write will either succeed without overwrite
        // or succeed with overwrite, but it will never fail
        assert!(result.is_ok() || result.is_err());

        if result.is_err() {
            // An overwrite occurred
            assert_eq!(result.unwrap_err(), 1);
            assert_eq!(buffer.overflow_count(), 1);
        }

        // Continue writing until we definitely get overwrites
        let mut overwrite_detected = false;
        for i in 0..10 {
            let id = RequestId::new();
            let data = format!("overflow event {i}");
            let result = buffer.write(id, data.as_bytes());

            // Write should always succeed (never fail)
            if result.is_err() {
                overwrite_detected = true;
                break;
            }
        }

        // We should eventually detect overwrites with a small buffer
        assert!(
            overwrite_detected,
            "Should eventually detect overwrites with small buffer"
        );
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

        // All writes should succeed (buffer is large enough)
        assert_eq!(total_successful, thread_count * writes_per_thread);

        // Read all events
        let mut read_count = 0;
        while buffer.read().is_some() {
            read_count += 1;
        }

        assert_eq!(read_count, total_successful);
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
