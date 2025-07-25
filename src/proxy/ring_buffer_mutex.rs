//! Ring buffer using Mutex for simplicity and safety

use crate::proxy::ring_buffer::RingBufferStats;
use crate::proxy::types::*;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

/// Simple safe ring buffer using VecDeque and Mutex
pub struct MutexRingBuffer {
    // Single mutex protecting the queue - simple and correct
    queue: Mutex<VecDeque<RingBufferEntry>>,
    capacity: usize,
    max_data_size: usize,
    overflow_count: AtomicU64,
    successful_writes: AtomicU64,
    successful_reads: AtomicU64,
}

#[derive(Clone, Debug)]
pub struct RingBufferEntry {
    pub request_id: RequestId,
    pub timestamp: TimestampNanos,
    pub data: Vec<u8>,
}

impl MutexRingBuffer {
    pub fn new(config: &RingBufferConfig) -> Self {
        let capacity = (*config.buffer_size.as_ref() / *config.slot_size.as_ref())
            .next_power_of_two()
            .max(1);

        Self {
            queue: Mutex::new(VecDeque::with_capacity(capacity)),
            capacity,
            max_data_size: *config.slot_size.as_ref(),
            overflow_count: AtomicU64::new(0),
            successful_writes: AtomicU64::new(0),
            successful_reads: AtomicU64::new(0),
        }
    }

    /// Write data (safe and simple)
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

        // Single lock acquisition
        let mut queue = self.queue.lock().unwrap();

        if queue.len() >= self.capacity {
            // Queue is full - drop oldest item (FIFO behavior)
            queue.pop_front();
            let overflow = self.overflow_count.fetch_add(1, Ordering::Relaxed) + 1;
            queue.push_back(entry);
            self.successful_writes.fetch_add(1, Ordering::Relaxed);
            Err(overflow)
        } else {
            queue.push_back(entry);
            self.successful_writes.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }
    }

    /// Read data (safe and simple)
    pub fn read(&self) -> Option<(RequestId, Vec<u8>)> {
        let mut queue = self.queue.lock().unwrap();

        match queue.pop_front() {
            Some(entry) => {
                self.successful_reads.fetch_add(1, Ordering::Relaxed);
                Some((entry.request_id, entry.data))
            }
            None => None,
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mutex_ring_buffer_basic() {
        let config = RingBufferConfig {
            buffer_size: BufferSize::try_new(1024).expect("valid size"),
            slot_size: SlotSize::try_new(256).expect("valid size"),
        };

        let buffer = MutexRingBuffer::new(&config);
        let request_id = RequestId::new();
        let data = b"test data";

        assert!(buffer.write(request_id, data).is_ok());
        let (read_id, read_data) = buffer.read().expect("Should have data");
        assert_eq!(read_id, request_id);
        assert_eq!(&read_data[..], data);
    }

    #[test]
    fn test_mutex_ring_buffer_overflow() {
        let config = RingBufferConfig {
            buffer_size: BufferSize::try_new(256).expect("valid size"), // Very small
            slot_size: SlotSize::try_new(64).expect("valid size"),
        };

        let buffer = MutexRingBuffer::new(&config);
        let capacity = buffer.capacity;

        // Fill the buffer
        for i in 0..capacity {
            let id = RequestId::new();
            let data = format!("item {i}");
            assert!(buffer.write(id, data.as_bytes()).is_ok());
        }

        // Next write should cause overflow but still succeed (drops oldest)
        let id = RequestId::new();
        let result = buffer.write(id, b"overflow item");
        assert!(result.is_err()); // Returns overflow count
        assert_eq!(buffer.overflow_count(), 1);
    }
}
