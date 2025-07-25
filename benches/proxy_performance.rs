//! Performance benchmarks for the proxy service
//!
//! These benchmarks verify that the hot path maintains <5ms latency
//! as required by ADR-0008 (Dual-path Architecture).

use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;
use std::sync::Arc;
use std::time::Duration;
use union_square::proxy::{ring_buffer::RingBuffer, types::*};

/// Benchmark ring buffer write performance - this is the critical hot path operation
fn bench_ring_buffer_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("ring_buffer_performance");
    group.significance_level(0.05);

    // Test different payload sizes
    for size in &[1024, 10 * 1024, 64 * 1024] {
        group.bench_function(format!("write_{}kb", size / 1024), |b| {
            let config = RingBufferConfig {
                buffer_size: BufferSize::try_new(10 * 1024 * 1024).expect("10MB is valid"), // 10MB
                slot_size: SlotSize::try_new(128 * 1024).expect("128KB is valid"), // 128KB slots
            };
            let ring_buffer = RingBuffer::new(&config);
            let data = vec![b'x'; *size];
            let request_id = RequestId::new();

            b.iter(|| {
                let _ = black_box(ring_buffer.write(request_id, &data));
            });
        });
    }

    // Benchmark concurrent writes to ring buffer
    group.bench_function("concurrent_writes", |b| {
        let config = RingBufferConfig {
            buffer_size: BufferSize::try_new(100 * 1024 * 1024).expect("100MB is valid"), // 100MB
            slot_size: SlotSize::try_new(64 * 1024).expect("64KB is valid"), // 64KB slots
        };
        let ring_buffer = Arc::new(RingBuffer::new(&config));
        let data = vec![b'x'; 1024]; // 1KB payload

        b.iter(|| {
            let handles: Vec<_> = (0..10)
                .map(|_| {
                    let rb = ring_buffer.clone();
                    let data = data.clone();
                    std::thread::spawn(move || {
                        let request_id = RequestId::new();
                        rb.write(request_id, &data)
                    })
                })
                .collect();

            for handle in handles {
                let _ = black_box(handle.join().unwrap());
            }
        });
    });

    group.finish();
}

/// Benchmark audit event serialization overhead
fn bench_audit_event_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("audit_event_serialization");

    // Test serialization of different event types
    group.bench_function("request_received_event", |b| {
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

        b.iter(|| {
            let serialized = serde_json::to_vec(&event).unwrap();
            black_box(serialized);
        });
    });

    group.bench_function("response_received_event", |b| {
        let event = AuditEvent {
            request_id: RequestId::new(),
            session_id: SessionId::new(),
            timestamp: chrono::Utc::now(),
            event_type: AuditEventType::ResponseReceived {
                status: HttpStatusCode::try_new(200).unwrap(),
                headers: Headers::from_vec(vec![(
                    "content-type".to_string(),
                    "application/json".to_string(),
                )])
                .unwrap(),
                body_size: BodySize::from(2048),
                duration_ms: DurationMillis::from(15),
            },
        };

        b.iter(|| {
            let serialized = serde_json::to_vec(&event).unwrap();
            black_box(serialized);
        });
    });

    group.finish();
}

/// Benchmark newtype validation overhead
fn bench_newtype_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("newtype_validation");

    group.bench_function("target_url_validation", |b| {
        let valid_url = "https://api.openai.com/v1/chat/completions";
        b.iter(|| {
            let result = TargetUrl::try_new(valid_url);
            let _ = black_box(result);
        });
    });

    group.bench_function("api_key_validation", |b| {
        let valid_key = "sk-1234567890abcdef";
        b.iter(|| {
            let result = ApiKey::try_new(valid_key);
            let _ = black_box(result);
        });
    });

    group.bench_function("request_id_generation", |b| {
        b.iter(|| {
            let id = RequestId::new();
            black_box(id);
        });
    });

    group.finish();
}

/// Benchmark hot path request processing simulation
fn bench_hot_path_simulation(c: &mut Criterion) {
    let mut group = c.benchmark_group("hot_path_simulation");
    group.significance_level(0.05);
    group.measurement_time(Duration::from_secs(10));

    // Simulate the hot path processing steps without actual networking
    group.bench_function("complete_hot_path_flow", |b| {
        let config = RingBufferConfig {
            buffer_size: BufferSize::try_new(100 * 1024 * 1024).expect("100MB is valid"), // 100MB
            slot_size: SlotSize::try_new(64 * 1024).expect("64KB is valid"),              // 64KB
        };
        let ring_buffer = RingBuffer::new(&config);

        b.iter(|| {
            // 1. Generate request ID (microseconds)
            let request_id = RequestId::new();

            // 2. Validate target URL (nanoseconds for valid URL)
            let _target_url = TargetUrl::try_new("https://api.openai.com/v1/chat/completions")
                .expect("Valid URL");

            // 3. Create audit event
            let audit_event = AuditEvent {
                request_id,
                session_id: SessionId::new(),
                timestamp: chrono::Utc::now(),
                event_type: AuditEventType::RequestReceived {
                    method: HttpMethod::try_new("POST".to_string()).unwrap(),
                    uri: RequestUri::try_new("/v1/chat/completions".to_string()).unwrap(),
                    headers: Headers::from_vec(vec![(
                        "content-type".to_string(),
                        "application/json".to_string(),
                    )])
                    .unwrap(),
                    body_size: BodySize::from(1024),
                },
            };

            // 4. Serialize event (this is the main overhead)
            let serialized = serde_json::to_vec(&audit_event).unwrap();

            // 5. Write to ring buffer (sub-microsecond)
            let _ = ring_buffer.write(request_id, &serialized);

            black_box(serialized);
        });
    });

    // Measure latency percentiles
    group.bench_function("hot_path_latency_distribution", |b| {
        let config = RingBufferConfig {
            buffer_size: BufferSize::try_new(100 * 1024 * 1024).expect("100MB is valid"),
            slot_size: SlotSize::try_new(64 * 1024).expect("64KB is valid"),
        };
        let ring_buffer = RingBuffer::new(&config);

        b.iter_custom(|iters| {
            let mut total_duration = Duration::ZERO;

            for _ in 0..iters {
                let start = std::time::Instant::now();

                // Simulate hot path operations
                let request_id = RequestId::new();
                let audit_event = AuditEvent {
                    request_id,
                    session_id: SessionId::new(),
                    timestamp: chrono::Utc::now(),
                    event_type: AuditEventType::RequestReceived {
                        method: HttpMethod::try_new("POST".to_string()).unwrap(),
                        uri: RequestUri::try_new("/v1/chat/completions".to_string()).unwrap(),
                        headers: Headers::new(),
                        body_size: BodySize::from(1024),
                    },
                };
                let serialized = serde_json::to_vec(&audit_event).unwrap();
                let _ = ring_buffer.write(request_id, &serialized);

                total_duration += start.elapsed();
            }

            total_duration
        });
    });

    group.finish();
}

/// Benchmark memory allocation patterns
fn bench_memory_allocation(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_allocation");

    // Benchmark allocation patterns for different sizes
    group.bench_function("audit_event_allocation", |b| {
        b.iter(|| {
            let headers: Vec<(String, String)> = (0..10)
                .map(|i| (format!("header-{i}"), format!("value-{i}")))
                .collect();

            let event = AuditEvent {
                request_id: RequestId::new(),
                session_id: SessionId::new(),
                timestamp: chrono::Utc::now(),
                event_type: AuditEventType::RequestReceived {
                    method: HttpMethod::try_new("POST".to_string()).unwrap(),
                    uri: RequestUri::try_new("/v1/chat/completions".to_string()).unwrap(),
                    headers: Headers::from_vec(headers).unwrap(),
                    body_size: BodySize::from(4096),
                },
            };
            black_box(event);
        });
    });

    group.finish();
}

// Criterion benchmark groups
criterion_group!(
    benches,
    bench_ring_buffer_performance,
    bench_audit_event_serialization,
    bench_newtype_validation,
    bench_hot_path_simulation,
    bench_memory_allocation
);

criterion_main!(benches);
