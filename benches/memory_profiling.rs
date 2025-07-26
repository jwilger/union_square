//! Memory profiling benchmark using dhat
//!
//! Run with: cargo bench --bench memory_profiling
//!
//! Note: This benchmark uses a global allocator which is required for dhat
//! memory profiling. This benchmark should be run in isolation to avoid
//! interference with other benchmarks.

use union_square::proxy::storage::RingBuffer;
use union_square::proxy::types::*;

// Global allocator is required for dhat heap profiling to track all allocations
#[global_allocator]
static ALLOCATOR: dhat::Alloc = dhat::Alloc;

fn main() {
    let _profiler = dhat::Profiler::new_heap();

    println!("Starting memory profiling...");

    // Profile ring buffer allocations
    profile_ring_buffer();

    // Profile audit event allocations
    profile_audit_events();

    // Profile concurrent operations
    profile_concurrent_allocations();

    println!("\nMemory profiling complete!");
}

fn profile_ring_buffer() {
    println!("\n=== Ring Buffer Memory Profile ===");

    let config = RingBufferConfig {
        buffer_size: BufferSize::try_new(16 * 1024 * 1024).expect("16MB is valid"),
        slot_size: SlotSize::try_new(128 * 1024).expect("128KB is valid"),
    };

    let ring_buffer = RingBuffer::new(&config);

    // Write various sized payloads
    for size in &[1024, 10 * 1024, 64 * 1024] {
        let data = vec![b'x'; *size];
        let request_id = RequestId::new();

        for _ in 0..100 {
            let _ = ring_buffer.write(request_id, &data);
        }

        println!("Completed 100 writes of {}KB", size / 1024);
    }
}

fn profile_audit_events() {
    println!("\n=== Audit Event Memory Profile ===");

    let mut events = Vec::new();

    // Create and serialize various audit events
    for i in 0..1000 {
        let event = AuditEvent {
            request_id: RequestId::new(),
            session_id: SessionId::new(),
            timestamp: chrono::Utc::now(),
            event_type: if i % 2 == 0 {
                AuditEventType::RequestReceived {
                    method: HttpMethod::try_new("POST".to_string()).unwrap(),
                    uri: RequestUri::try_new("/v1/chat/completions".to_string()).unwrap(),
                    headers: Headers::from_vec(vec![
                        ("content-type".to_string(), "application/json".to_string()),
                        ("authorization".to_string(), format!("Bearer sk-test{i}")),
                        ("user-agent".to_string(), "union-square/1.0".to_string()),
                    ])
                    .unwrap(),
                    body_size: BodySize::from(1024 * (i % 10 + 1) as usize),
                }
            } else {
                AuditEventType::ResponseReceived {
                    status: HttpStatusCode::try_new(200).unwrap(),
                    headers: Headers::from_vec(vec![
                        ("content-type".to_string(), "application/json".to_string()),
                        (
                            "content-length".to_string(),
                            format!("{}", 2048 * (i % 5 + 1)),
                        ),
                    ])
                    .unwrap(),
                    body_size: BodySize::from(2048 * (i % 5 + 1) as usize),
                    duration_ms: DurationMillis::from(15 + (i % 100) as u64),
                }
            },
        };

        let serialized = serde_json::to_vec(&event).unwrap();
        events.push(serialized);
    }

    println!("Created and serialized 1000 audit events");
    println!(
        "Total serialized size: {} bytes",
        events.iter().map(|e| e.len()).sum::<usize>()
    );
}

fn profile_concurrent_allocations() {
    println!("\n=== Concurrent Operations Memory Profile ===");

    use std::sync::Arc;
    use std::thread;

    let config = RingBufferConfig {
        buffer_size: BufferSize::try_new(128 * 1024 * 1024).expect("128MB is valid"),
        slot_size: SlotSize::try_new(64 * 1024).expect("64KB is valid"),
    };
    let ring_buffer = Arc::new(RingBuffer::new(&config));

    let handles: Vec<_> = (0..10)
        .map(|thread_id| {
            let rb = ring_buffer.clone();
            thread::spawn(move || {
                for i in 0..1000 {
                    let request_id = RequestId::new();
                    let data = vec![b'x'; 1024 + (i % 10) * 1024]; // Variable sizes
                    let _ = rb.write(request_id, &data);
                }
                println!("Thread {thread_id} completed 1000 writes");
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    println!("Completed concurrent allocation test");
}
