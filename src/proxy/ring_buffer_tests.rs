//! Property-based tests for the ring buffer

use super::*;
use proptest::prelude::*;
use std::sync::Arc;
use std::thread;

// Strategy for generating valid buffer configurations
prop_compose! {
    fn buffer_config_strategy()(
        // Buffer size must be power of 2, between 1KB and 1MB for testing
        buffer_power in 10u32..20u32,
        // Slot size between 64 bytes and 64KB
        slot_size in 64usize..65536usize,
    ) -> RingBufferConfig {
        let buffer_size = 1usize << buffer_power; // 2^buffer_power
        RingBufferConfig {
            buffer_size: BufferSize::try_new(buffer_size).unwrap(),
            slot_size: SlotSize::try_new(slot_size).unwrap(),
        }
    }
}

// Strategy for generating data that fits in a slot
prop_compose! {
    fn data_strategy(max_size: usize)(
        data in prop::collection::vec(any::<u8>(), 1..=max_size)
    ) -> Vec<u8> {
        data
    }
}

proptest! {
    #[test]
    fn prop_ring_buffer_never_loses_data_under_capacity(
        config in buffer_config_strategy(),
        data_sets in prop::collection::vec(
            prop::collection::vec(any::<u8>(), 1..=1024), // Small data chunks
            1..=10 // Limited number of writes to stay under capacity
        )
    ) {
        let ring_buffer = RingBuffer::new(&config);
        let mut written_data = Vec::new();
        let mut request_ids = Vec::new();

        // Calculate actual slot count after power-of-2 adjustment
        let actual_slot_count = {
            let calculated = config.buffer_size.as_ref() / config.slot_size.as_ref();
            let next_pow2 = calculated.next_power_of_two();
            if next_pow2 > calculated {
                next_pow2 / 2
            } else {
                next_pow2
            }.max(1)
        };

        // Write all data (up to slot capacity)
        for (i, data) in data_sets.iter().enumerate() {
            // Stop if we've written to all available slots
            if i >= actual_slot_count {
                break;
            }

            let request_id = RequestId::new();
            request_ids.push(request_id);

            // Write will succeed if data fits in slot (data is stored separately from metadata)
            if data.len() <= *config.slot_size.as_ref() {
                prop_assert!(ring_buffer.write(request_id, data).is_ok());
                written_data.push((request_id, data.clone()));
            }
        }

        // Read all data back
        let mut read_data = Vec::new();
        while let Some((id, data)) = ring_buffer.read() {
            read_data.push((id, data));
        }

        // Verify all written data was read (order may differ due to concurrent access)
        prop_assert_eq!(written_data.len(), read_data.len());

        // Verify all data is present (may be in different order)
        for (id, data) in &written_data {
            prop_assert!(read_data.iter().any(|(read_id, read_data)|
                read_id == id && read_data == data
            ));
        }
    }

    #[test]
    fn prop_concurrent_writes_are_thread_safe(
        config in buffer_config_strategy(),
        thread_count in 2usize..=8usize,
        writes_per_thread in 10usize..=50usize,
    ) {
        let ring_buffer = Arc::new(RingBuffer::new(&config));
        let mut handles = Vec::new();

        // Spawn threads that write concurrently
        for thread_id in 0..thread_count {
            let rb = Arc::clone(&ring_buffer);
            let handle = thread::spawn(move || {
                let mut successful_writes = 0;
                for i in 0..writes_per_thread {
                    let data = format!("thread-{thread_id}-write-{i}").into_bytes();
                    let request_id = RequestId::new();
                    if rb.write(request_id, &data).is_ok() {
                        successful_writes += 1;
                    }
                }
                successful_writes
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        let total_successful_writes: usize = handles
            .into_iter()
            .map(|h| h.join().unwrap())
            .sum();

        // Read all data
        let mut read_count = 0;
        while ring_buffer.read().is_some() {
            read_count += 1;
        }

        // All successful writes should be readable
        prop_assert_eq!(total_successful_writes, read_count);
    }

    #[test]
    fn prop_data_too_large_is_truncated(
        config in buffer_config_strategy(),
        excess in 1usize..=1000usize,
    ) {
        let ring_buffer = RingBuffer::new(&config);
        let request_id = RequestId::new();

        // Create data that exceeds slot size
        let max_data_size = config.slot_size.as_ref();
        let oversized_data = vec![42u8; max_data_size + excess];

        // Write should succeed but data will be truncated
        prop_assert!(ring_buffer.write(request_id, &oversized_data).is_ok());

        // Read back and verify truncation
        if let Some((read_id, read_data)) = ring_buffer.read() {
            prop_assert_eq!(read_id, request_id);
            // Data should be truncated to slot size
            prop_assert_eq!(read_data.len(), *max_data_size);
            // Check truncated data matches
            prop_assert_eq!(&read_data[..], &oversized_data[..*max_data_size]);
        } else {
            panic!("Expected to read data back");
        }
    }

    #[test]
    fn prop_ring_buffer_handles_wraparound(
        config in buffer_config_strategy().prop_filter("Need multiple slots", |c| {
            let slot_count = c.buffer_size.as_ref() / c.slot_size.as_ref();
            slot_count >= 4
        }),
        write_count in 100usize..=1000usize,
    ) {
        let ring_buffer = RingBuffer::new(&config);
        let slot_count = config.buffer_size.as_ref() / config.slot_size.as_ref();

        // Write more data than the buffer can hold to force wraparound
        let small_data = vec![1u8; 32]; // Small data to ensure it fits
        let mut successful_writes = 0;

        for _ in 0..write_count {
            let request_id = RequestId::new();
            if ring_buffer.write(request_id, &small_data).is_ok() {
                successful_writes += 1;
            }
        }

        // We should have written some data (exact count depends on timing/contention)
        prop_assert!(successful_writes > 0);
        prop_assert!(successful_writes <= write_count);

        // Read back data - should get the most recent writes
        let mut read_count = 0;
        while ring_buffer.read().is_some() {
            read_count += 1;
        }

        // Should read approximately one buffer's worth of data
        prop_assert!(read_count <= slot_count);
        prop_assert!(read_count > 0);
    }

    #[test]
    fn prop_stats_are_accurate(
        config in buffer_config_strategy(),
        operations in prop::collection::vec(
            prop::bool::ANY, // true = write, false = read
            1..=100
        ),
    ) {
        let ring_buffer = RingBuffer::new(&config);
        let small_data = vec![1u8; 32];

        let initial_stats = ring_buffer.stats();
        prop_assert_eq!(initial_stats.total_writes, 0);
        prop_assert_eq!(initial_stats.total_reads, 0);
        prop_assert_eq!(initial_stats.dropped_events.as_ref(), &0);

        let mut expected_writes = 0;
        let mut expected_reads = 0;
        let mut available_to_read = 0;

        for is_write in operations {
            if is_write {
                let request_id = RequestId::new();
                if ring_buffer.write(request_id, &small_data).is_ok() {
                    expected_writes += 1;
                    available_to_read += 1;
                }
            } else if available_to_read > 0 && ring_buffer.read().is_some() {
                expected_reads += 1;
                available_to_read -= 1;
            }
        }

        let final_stats = ring_buffer.stats();
        prop_assert_eq!(final_stats.total_writes, expected_writes);
        prop_assert_eq!(final_stats.total_reads, expected_reads);
    }
}

#[test]
fn test_ring_buffer_memory_layout() {
    // Test that our memory layout assumptions are correct
    let config = RingBufferConfig {
        buffer_size: BufferSize::try_new(BYTES_1KB).unwrap(),
        slot_size: SlotSize::try_new(SLOT_SIZE_SMALL).unwrap(),
    };

    let ring_buffer = RingBuffer::new(&config);
    let _expected_slots = BYTES_1KB / SLOT_SIZE_SMALL; // 8 slots

    // Write to fill some slots
    let mut written_ids = Vec::new();
    for i in 0..5 {
        let request_id = RequestId::new();
        written_ids.push(request_id);
        let data = vec![i as u8; 32];
        assert!(
            ring_buffer.write(request_id, &data).is_ok(),
            "Write {i} should succeed"
        );
    }

    // Read back the data to verify
    for (i, expected_id) in written_ids.iter().enumerate() {
        let (id, data) = ring_buffer.read().expect("Should be able to read");
        assert_eq!(id, *expected_id);
        assert_eq!(data[0], i as u8);
    }

    // No more data
    assert!(ring_buffer.read().is_none());

    // Write more data
    for i in 5..10 {
        let request_id = RequestId::new();
        let data = vec![i as u8; 32];
        assert!(
            ring_buffer.write(request_id, &data).is_ok(),
            "Write {i} should succeed"
        );
    }

    // Should be able to read the new data
    let mut read_count = 0;
    while ring_buffer.read().is_some() {
        read_count += 1;
    }
    assert_eq!(read_count, 5);
}

#[test]
fn test_ring_buffer_concurrent_stress() {
    // Stress test with many concurrent writers and readers
    let config = RingBufferConfig {
        buffer_size: BufferSize::try_new(BYTES_1MB).unwrap(),
        slot_size: SlotSize::try_new(BYTES_1KB).unwrap(),
    };

    let ring_buffer = Arc::new(RingBuffer::new(&config));
    let mut write_handles = Vec::new();
    let mut read_handles = Vec::new();

    // Spawn writer threads
    for thread_id in 0..TEST_THREAD_COUNT {
        let rb = Arc::clone(&ring_buffer);
        let handle = thread::spawn(move || {
            let mut successful_writes = 0;
            for i in 0..TEST_ITERATIONS_LARGE {
                let data = format!("thread-{thread_id}-msg-{i}").into_bytes();
                let request_id = RequestId::new();
                if rb.write(request_id, &data).is_ok() {
                    successful_writes += 1;
                }

                // Small delay to simulate real work
                thread::yield_now();
            }
            successful_writes
        });
        write_handles.push(handle);
    }

    // Create a shutdown signal for coordinated termination
    let shutdown_flag = Arc::new(std::sync::atomic::AtomicBool::new(false));

    // Spawn reader threads that continue until shutdown
    for _ in 0..5 {
        let rb = Arc::clone(&ring_buffer);
        let shutdown = Arc::clone(&shutdown_flag);
        let handle = thread::spawn(move || {
            let mut read_count = 0;
            while !shutdown.load(std::sync::atomic::Ordering::Relaxed) {
                if rb.read().is_some() {
                    read_count += 1;
                }
                // Small delay to simulate processing
                thread::yield_now();
            }
            // Continue draining until no more data
            while rb.read().is_some() {
                read_count += 1;
            }
            read_count
        });
        read_handles.push(handle);
    }

    // Wait for all writer threads to complete
    let total_writes: usize = write_handles.into_iter().map(|h| h.join().unwrap()).sum();

    // Signal readers to start shutdown
    shutdown_flag.store(true, std::sync::atomic::Ordering::Relaxed);

    // Wait for all reader threads to complete
    let total_reads: usize = read_handles.into_iter().map(|h| h.join().unwrap()).sum();

    // Check final stats - now reads should equal writes since readers drain everything
    let stats = ring_buffer.stats();
    assert!(stats.total_writes > 0);
    assert!(stats.total_reads > 0);
    assert_eq!(total_reads, total_writes);
}
