//! Performance test to compare safe ring buffer implementations

#[cfg(test)]
mod tests {
    use crate::proxy::ring_buffer::RingBuffer as UnsafeRingBuffer;
    use crate::proxy::ring_buffer_mutex::MutexRingBuffer;
    use crate::proxy::ring_buffer_safe::SafeRingBuffer;
    use crate::proxy::types::*;
    use std::sync::Arc;
    use std::thread;
    use std::time::Instant;

    #[test]
    fn performance_comparison_single_threaded() {
        let config = RingBufferConfig {
            buffer_size: BufferSize::try_new(1024 * 1024).expect("valid size"), // 1MB
            slot_size: SlotSize::try_new(1024).expect("valid size"),            // 1KB
        };

        let request_id = RequestId::new();
        let data = vec![0u8; 512]; // 512 bytes
        let iterations = 10000;

        // Test unsafe ring buffer
        let unsafe_buffer = UnsafeRingBuffer::new(&config);
        let start = Instant::now();
        for _ in 0..iterations {
            let _ = unsafe_buffer.write(request_id, &data);
        }
        let unsafe_duration = start.elapsed();

        // Test crossbeam safe ring buffer
        let safe_buffer = SafeRingBuffer::new(&config);
        let start = Instant::now();
        for _ in 0..iterations {
            let _ = safe_buffer.write(request_id, &data);
        }
        let crossbeam_duration = start.elapsed();

        // Test mutex ring buffer
        let mutex_buffer = MutexRingBuffer::new(&config);
        let start = Instant::now();
        for _ in 0..iterations {
            let _ = mutex_buffer.write(request_id, &data);
        }
        let mutex_duration = start.elapsed();

        println!("Performance comparison ({iterations} writes):");
        println!(
            "  Unsafe ring buffer: {:?} ({:.2}ns per write)",
            unsafe_duration,
            unsafe_duration.as_nanos() as f64 / iterations as f64
        );
        println!(
            "  Crossbeam safe:     {:?} ({:.2}ns per write)",
            crossbeam_duration,
            crossbeam_duration.as_nanos() as f64 / iterations as f64
        );
        println!(
            "  Mutex safe:         {:?} ({:.2}ns per write)",
            mutex_duration,
            mutex_duration.as_nanos() as f64 / iterations as f64
        );

        // Performance ratios
        let crossbeam_ratio =
            crossbeam_duration.as_nanos() as f64 / unsafe_duration.as_nanos() as f64;
        let mutex_ratio = mutex_duration.as_nanos() as f64 / unsafe_duration.as_nanos() as f64;

        println!("  Crossbeam overhead: {crossbeam_ratio:.2}x slower than unsafe");
        println!("  Mutex overhead:     {mutex_ratio:.2}x slower than unsafe");

        // Both safe alternatives should still be very fast
        // Even if they're 2-10x slower, we're talking nanoseconds vs microseconds
        let crossbeam_ns_per_write = crossbeam_duration.as_nanos() as f64 / iterations as f64;
        let mutex_ns_per_write = mutex_duration.as_nanos() as f64 / iterations as f64;

        println!("  Crossbeam absolute: {crossbeam_ns_per_write:.2}ns per write");
        println!("  Mutex absolute:     {mutex_ns_per_write:.2}ns per write");

        // Even if these are 10x slower than unsafe, they should still be well under 1μs
        assert!(
            crossbeam_ns_per_write < 1000.0,
            "Crossbeam should be under 1μs per write"
        );
        assert!(
            mutex_ns_per_write < 10000.0,
            "Mutex should be under 10μs per write"
        );
    }

    #[test]
    fn performance_comparison_concurrent() {
        let config = RingBufferConfig {
            buffer_size: BufferSize::try_new(4 * 1024 * 1024).expect("valid size"), // 4MB
            slot_size: SlotSize::try_new(1024).expect("valid size"),                // 1KB
        };

        let thread_count = 4;
        let writes_per_thread = 1000;

        // Test concurrent performance with different implementations
        println!(
            "Concurrent performance test ({thread_count} threads, {writes_per_thread} writes each):"
        );

        // Unsafe ring buffer
        let unsafe_buffer = Arc::new(UnsafeRingBuffer::new(&config));
        let start = Instant::now();
        let handles: Vec<_> = (0..thread_count)
            .map(|_| {
                let buffer = Arc::clone(&unsafe_buffer);
                thread::spawn(move || {
                    let request_id = RequestId::new();
                    let data = vec![0u8; 256];
                    for _ in 0..writes_per_thread {
                        let _ = buffer.write(request_id, &data);
                    }
                })
            })
            .collect();
        for handle in handles {
            handle.join().unwrap();
        }
        let unsafe_concurrent_duration = start.elapsed();

        // Safe crossbeam ring buffer
        let safe_buffer = Arc::new(SafeRingBuffer::new(&config));
        let start = Instant::now();
        let handles: Vec<_> = (0..thread_count)
            .map(|_| {
                let buffer = Arc::clone(&safe_buffer);
                thread::spawn(move || {
                    let request_id = RequestId::new();
                    let data = vec![0u8; 256];
                    for _ in 0..writes_per_thread {
                        let _ = buffer.write(request_id, &data);
                    }
                })
            })
            .collect();
        for handle in handles {
            handle.join().unwrap();
        }
        let crossbeam_concurrent_duration = start.elapsed();

        // Mutex ring buffer
        let mutex_buffer = Arc::new(MutexRingBuffer::new(&config));
        let start = Instant::now();
        let handles: Vec<_> = (0..thread_count)
            .map(|_| {
                let buffer = Arc::clone(&mutex_buffer);
                thread::spawn(move || {
                    let request_id = RequestId::new();
                    let data = vec![0u8; 256];
                    for _ in 0..writes_per_thread {
                        let _ = buffer.write(request_id, &data);
                    }
                })
            })
            .collect();
        for handle in handles {
            handle.join().unwrap();
        }
        let mutex_concurrent_duration = start.elapsed();

        println!("  Unsafe concurrent:    {unsafe_concurrent_duration:?}");
        println!("  Crossbeam concurrent: {crossbeam_concurrent_duration:?}");
        println!("  Mutex concurrent:     {mutex_concurrent_duration:?}");

        let total_operations = thread_count * writes_per_thread;
        println!("  Operations per second:");
        println!(
            "    Unsafe:    {:.0} ops/sec",
            total_operations as f64 / unsafe_concurrent_duration.as_secs_f64()
        );
        println!(
            "    Crossbeam: {:.0} ops/sec",
            total_operations as f64 / crossbeam_concurrent_duration.as_secs_f64()
        );
        println!(
            "    Mutex:     {:.0} ops/sec",
            total_operations as f64 / mutex_concurrent_duration.as_secs_f64()
        );
    }

    #[test]
    fn test_safe_alternatives_correctness() {
        let config = RingBufferConfig {
            buffer_size: BufferSize::try_new(1024).expect("valid size"),
            slot_size: SlotSize::try_new(256).expect("valid size"),
        };

        // Test that all implementations work correctly
        let request_id = RequestId::new();
        let data = b"test data for correctness";

        // Crossbeam version
        let safe_buffer = SafeRingBuffer::new(&config);
        assert!(safe_buffer.write(request_id, data).is_ok());
        let (read_id, read_data) = safe_buffer.read().expect("Should read data");
        assert_eq!(read_id, request_id);
        assert_eq!(&read_data[..], data);

        // Mutex version
        let mutex_buffer = MutexRingBuffer::new(&config);
        assert!(mutex_buffer.write(request_id, data).is_ok());
        let (read_id, read_data) = mutex_buffer.read().expect("Should read data");
        assert_eq!(read_id, request_id);
        assert_eq!(&read_data[..], data);

        println!("✅ All safe implementations work correctly!");
    }
}
