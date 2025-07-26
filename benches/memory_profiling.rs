//! Memory profiling benchmark using dhat
//!
//! Run with: cargo bench --bench memory_profiling
//!
//! Note: This benchmark uses a global allocator which is required for dhat
//! memory profiling. This benchmark should be run in isolation to avoid
//! interference with other benchmarks.

use union_square::benchmark_types::*;
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
    let payload_sizes = [
        PayloadSize::one_kb(),
        PayloadSize::ten_kb(),
        PayloadSize::sixty_four_kb(),
    ];
    for size in &payload_sizes {
        let data = vec![b'x'; size.into_inner()];
        let request_id = RequestId::new();

        let write_iterations = BenchmarkIterations::try_new(100).expect("100 is valid iterations");
        for _ in 0..write_iterations.into_inner() {
            let _ = ring_buffer.write(request_id, &data);
        }

        println!(
            "Completed {} writes of {}KB",
            write_iterations.into_inner(),
            size.into_inner() / 1024
        );
    }
}

fn profile_audit_events() {
    println!("\n=== Audit Event Memory Profile ===");

    let mut events = Vec::new();

    // Create and serialize various audit events
    let event_iterations = BenchmarkIterations::try_new(1000).expect("1000 is valid iterations");
    for i in 0..event_iterations.into_inner() {
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
                    body_size: BodySize::from(
                        PayloadSize::one_kb().into_inner() * (i % 10 + 1) as usize,
                    ),
                }
            } else {
                AuditEventType::ResponseReceived {
                    status: HttpStatusCode::try_new(200).unwrap(),
                    headers: Headers::from_vec(vec![
                        ("content-type".to_string(), "application/json".to_string()),
                        (
                            "content-length".to_string(),
                            format!(
                                "{}",
                                PayloadSize::one_kb().into_inner() * 2 * (i % 5 + 1) as usize
                            ),
                        ),
                    ])
                    .unwrap(),
                    body_size: BodySize::from(
                        PayloadSize::one_kb().into_inner() * 2 * (i % 5 + 1) as usize,
                    ),
                    duration_ms: DurationMillis::from(15 + (i % 100) as u64),
                }
            },
        };

        let serialized = serde_json::to_vec(&event).unwrap();
        events.push(serialized);
    }

    println!(
        "Created and serialized {} audit events",
        event_iterations.into_inner()
    );
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

    let thread_count = ThreadCount::try_new(10).expect("10 is valid thread count");
    let handles: Vec<_> = (0..thread_count.into_inner())
        .map(|thread_id| {
            let rb = ring_buffer.clone();
            thread::spawn(move || {
                let ops_per_thread = OperationsPerThread::try_new(1000).expect("1000 is valid ops");
                for i in 0..ops_per_thread.into_inner() {
                    let request_id = RequestId::new();
                    let base_size = PayloadSize::one_kb().into_inner();
                    let data = vec![b'x'; base_size + (i % 10) as usize * base_size]; // Variable sizes
                    let _ = rb.write(request_id, &data);
                }
                println!(
                    "Thread {thread_id} completed {} writes",
                    ops_per_thread.into_inner()
                );
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    println!("Completed concurrent allocation test");
}
