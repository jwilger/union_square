//! Tests to validate that our benchmark framework is configured correctly
//! and that performance requirements are being met.

use std::time::{Duration, Instant};
use union_square::proxy::types::*;

/// Test that we can measure sub-millisecond operations accurately
#[test]
fn test_timing_precision() {
    let start = Instant::now();

    // Do a simple operation that should take nanoseconds
    let _id = RequestId::new();

    let elapsed = start.elapsed();

    // Verify we can measure sub-millisecond durations
    assert!(elapsed < Duration::from_millis(1));
    assert!(elapsed > Duration::from_nanos(0));
}

/// Test that critical path operations complete within performance budget
#[test]
fn test_critical_path_performance() {
    use union_square::proxy::storage::RingBuffer;

    let config = RingBufferConfig {
        buffer_size: BufferSize::try_new(16 * 1024 * 1024).expect("16MB is valid"),
        slot_size: SlotSize::try_new(128 * 1024).expect("128KB is valid"),
    };
    let ring_buffer = RingBuffer::new(&config);

    // Run multiple iterations to get stable measurements
    let mut durations = Vec::new();
    for _ in 0..100 {
        let start = Instant::now();

        // Critical path operations
        let request_id = RequestId::new();
        let data = vec![b'x'; 1024]; // 1KB payload
        let _ = ring_buffer.write(request_id, &data);

        durations.push(start.elapsed());
    }

    // Calculate average and max
    let total: Duration = durations.iter().sum();
    let avg = total / durations.len() as u32;
    let max = durations.iter().max().unwrap();

    // Verify performance requirements
    // The whole critical path should be well under 5ms
    assert!(
        avg < Duration::from_millis(1),
        "Average duration {avg:?} exceeds 1ms"
    );
    assert!(
        *max < Duration::from_millis(5),
        "Max duration {max:?} exceeds 5ms budget"
    );
}

/// Test that the system can handle concurrent operations
#[test]
fn test_concurrent_performance() {
    use std::sync::Arc;
    use std::thread;
    use union_square::proxy::storage::RingBuffer;

    let config = RingBufferConfig {
        buffer_size: BufferSize::try_new(128 * 1024 * 1024).expect("128MB is valid"),
        slot_size: SlotSize::try_new(64 * 1024).expect("64KB is valid"),
    };
    let ring_buffer = Arc::new(RingBuffer::new(&config));

    let start = Instant::now();

    // Spawn 10 threads each doing 100 operations
    let handles: Vec<_> = (0..10)
        .map(|_| {
            let rb = ring_buffer.clone();
            thread::spawn(move || {
                for _ in 0..100 {
                    let request_id = RequestId::new();
                    let data = vec![b'x'; 1024];
                    let _ = rb.write(request_id, &data);
                }
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    let total_elapsed = start.elapsed();

    // 1000 total operations should complete quickly even with contention
    assert!(
        total_elapsed < Duration::from_secs(1),
        "Concurrent operations took {total_elapsed:?}, exceeding 1 second"
    );
}

/// Test memory allocation patterns don't cause performance issues
#[test]
fn test_allocation_performance() {
    use union_square::proxy::types::{AuditEvent, AuditEventType};

    let iterations = 1000;
    let start = Instant::now();

    for _ in 0..iterations {
        // Create events that would be serialized
        let event = AuditEvent {
            request_id: RequestId::new(),
            session_id: SessionId::new(),
            timestamp: chrono::Utc::now(),
            event_type: AuditEventType::RequestReceived {
                method: HttpMethod::try_new("POST".to_string()).unwrap(),
                uri: RequestUri::try_new("/v1/chat/completions".to_string()).unwrap(),
                headers: Headers::from_vec(vec![
                    ("content-type".to_string(), "application/json".to_string()),
                    ("authorization".to_string(), "Bearer test-key".to_string()),
                ])
                .unwrap(),
                body_size: BodySize::from(1024),
            },
        };

        // Serialize to simulate real usage
        let _ = serde_json::to_vec(&event).unwrap();
    }

    let elapsed = start.elapsed();
    let per_iteration = elapsed / iterations;

    // Each event creation and serialization should be sub-millisecond
    assert!(
        per_iteration < Duration::from_millis(1),
        "Per-iteration time {per_iteration:?} exceeds 1ms"
    );
}
