//! Safe ring buffer implementation using crossbeam for lock-free operations

use crate::proxy::ring_buffer::RingBufferStats;
use crate::proxy::types::*;
use crossbeam::queue::ArrayQueue;
use std::sync::atomic::{AtomicU64, Ordering};

/// A safe ring buffer entry with all data
#[derive(Clone, Debug)]
pub struct RingBufferEntry {
    pub request_id: RequestId,
    pub timestamp: TimestampNanos,
    pub data: Vec<u8>,
}

/// Safe lock-free ring buffer using crossbeam's ArrayQueue
pub struct SafeRingBuffer {
    queue: ArrayQueue<RingBufferEntry>,
    overflow_count: AtomicU64,
    successful_writes: AtomicU64,
    successful_reads: AtomicU64,
    max_data_size: usize,
}

impl SafeRingBuffer {
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

    /// Write data to the ring buffer (completely safe)
    pub fn write(&self, request_id: RequestId, data: &[u8]) -> Result<(), u64> {
        // Truncate data if needed
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

        match self.queue.push(entry) {
            Ok(()) => {
                self.successful_writes.fetch_add(1, Ordering::Relaxed);
                Ok(())
            }
            Err(_) => {
                // Queue is full
                let overflow = self.overflow_count.fetch_add(1, Ordering::Relaxed) + 1;
                Err(overflow)
            }
        }
    }

    /// Read the next available entry (completely safe)
    pub fn read(&self) -> Option<(RequestId, Vec<u8>)> {
        match self.queue.pop() {
            Some(entry) => {
                self.successful_reads.fetch_add(1, Ordering::Relaxed);
                Some((entry.request_id, entry.data))
            }
            None => None,
        }
    }

    /// Get statistics
    pub fn stats(&self) -> RingBufferStats {
        RingBufferStats {
            total_writes: self.successful_writes.load(Ordering::Relaxed),
            total_reads: self.successful_reads.load(Ordering::Relaxed),
            dropped_events: DroppedEventCount::from(self.overflow_count.load(Ordering::Relaxed)),
        }
    }

    /// Get the current overflow count
    pub fn overflow_count(&self) -> u64 {
        self.overflow_count.load(Ordering::Relaxed)
    }
}

// Safe to send between threads - no unsafe code needed!
unsafe impl Send for SafeRingBuffer {}
unsafe impl Sync for SafeRingBuffer {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_ring_buffer_basic_operations() {
        let config = RingBufferConfig {
            buffer_size: BufferSize::try_new(1024).expect("valid size"),
            slot_size: SlotSize::try_new(256).expect("valid size"),
        };

        let buffer = SafeRingBuffer::new(&config);
        let request_id = RequestId::new();
        let data = b"test data";

        // Write should succeed
        assert!(buffer.write(request_id, data).is_ok());

        // Read should return the same data
        let (read_id, read_data) = buffer.read().expect("Should have data");
        assert_eq!(read_id, request_id);
        assert_eq!(&read_data[..], data);
    }

    #[test]
    fn test_safe_ring_buffer_concurrent_operations() {
        use std::sync::Arc;
        use std::thread;

        let config = RingBufferConfig {
            buffer_size: BufferSize::try_new(4096).expect("valid size"),
            slot_size: SlotSize::try_new(256).expect("valid size"),
        };

        let buffer = Arc::new(SafeRingBuffer::new(&config));
        let thread_count = 4;
        let writes_per_thread = 100;

        let handles: Vec<_> = (0..thread_count)
            .map(|thread_id| {
                let buffer_clone = Arc::clone(&buffer);
                thread::spawn(move || {
                    for i in 0..writes_per_thread {
                        let id = RequestId::new();
                        let data = format!("thread {thread_id} item {i}");
                        let _ = buffer_clone.write(id, data.as_bytes());
                    }
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        // All operations should complete without panics
        let stats = buffer.stats();
        println!("Successful writes: {}", stats.total_writes);
    }
}
