//! Performance benchmarks for the proxy service
//!
//! These benchmarks verify that the hot path maintains <5ms latency
//! as required by ADR-0008 (Dual-path Architecture).

use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;
use std::sync::Arc;
use std::time::Duration;
use union_square::proxy::{
    hot_path::StreamingHotPathService, middleware::AuthConfig, ring_buffer::RingBuffer, types::*,
};

/// Benchmark ring buffer write performance - this is the critical hot path operation
fn bench_ring_buffer_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("ring_buffer_performance");
    group.significance_level(0.05);

    // Test different payload sizes
    for size in &[1024, 10 * 1024, 64 * 1024] {
        group.bench_function(format!("write_{}kb", size / 1024), |b| {
            let config = RingBufferConfig {
                buffer_size: BufferSize::try_new(16 * 1024 * 1024).expect("16MB is valid"), // 16MB (power of 2)
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
            buffer_size: BufferSize::try_new(128 * 1024 * 1024).expect("128MB is valid"), // 128MB (power of 2)
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
            buffer_size: BufferSize::try_new(128 * 1024 * 1024).expect("128MB is valid"), // 128MB
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
            buffer_size: BufferSize::try_new(128 * 1024 * 1024).expect("128MB is valid"),
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

/// Benchmark streaming service performance
fn bench_streaming_service_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("streaming_service_performance");
    group.significance_level(0.05);
    group.measurement_time(Duration::from_secs(10));

    // Benchmark URI construction logic
    group.bench_function("uri_construction", |b| {
        let config = ProxyConfig::default();
        let ring_buffer = Arc::new(RingBuffer::new(&config.ring_buffer));
        let _service = StreamingHotPathService::new(config, ring_buffer);

        b.iter(|| {
            // Simulate URI construction for different scenarios
            let base_url = "https://api.openai.com";
            let endpoint_url = "https://api.openai.com/v1/chat/completions";

            // Test base URL + path scenario
            let target1 = TargetUrl::try_new(base_url.to_string()).unwrap();
            let path1 = "/v1/chat/completions";

            // Test full endpoint URL scenario
            let target2 = TargetUrl::try_new(endpoint_url.to_string()).unwrap();

            black_box((target1, path1, target2));
        });
    });

    // Benchmark request/response metadata extraction
    group.bench_function("metadata_extraction", |b| {
        b.iter(|| {
            // Simulate extracting headers and metadata from HTTP request/response
            let headers: Vec<(String, String)> = vec![
                ("content-type".to_string(), "application/json".to_string()),
                ("authorization".to_string(), "Bearer sk-test123".to_string()),
                ("user-agent".to_string(), "union-square/1.0".to_string()),
                ("accept".to_string(), "*/*".to_string()),
                ("content-length".to_string(), "1024".to_string()),
            ];

            let method = HttpMethod::try_new("POST".to_string()).unwrap();
            let uri = RequestUri::try_new("/v1/chat/completions".to_string()).unwrap();
            let status = HttpStatusCode::try_new(200).unwrap();

            black_box((headers, method, uri, status));
        });
    });

    group.finish();
}

/// Benchmark middleware stack overhead
fn bench_middleware_stack_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("middleware_stack_performance");
    group.significance_level(0.05);
    group.measurement_time(Duration::from_secs(10));

    // Benchmark individual middleware components
    group.bench_function("request_id_generation", |b| {
        b.iter(|| {
            // Simulate request ID middleware overhead
            let request_id = uuid::Uuid::now_v7();
            let header_value = axum::http::HeaderValue::from_str(&request_id.to_string()).unwrap();
            black_box((request_id, header_value));
        });
    });

    group.bench_function("auth_validation", |b| {
        let auth_config = AuthConfig::default();
        let _api_key = ApiKey::try_new("sk-test123456789".to_string()).unwrap();

        b.iter(|| {
            // Simulate authentication middleware overhead
            let auth_header = "Bearer sk-test123456789";
            let extracted_key = auth_header.trim_start_matches("Bearer ").trim();
            let parsed_key = ApiKey::try_new(extracted_key.to_string()).unwrap();
            let is_valid = auth_config.api_keys.contains(&parsed_key);

            black_box((extracted_key, parsed_key, is_valid));
        });
    });

    group.bench_function("logging_metadata", |b| {
        b.iter(|| {
            // Simulate logging middleware overhead
            let method = "POST";
            let path = "/v1/chat/completions";
            let request_id = "req_123456789";
            let status = 200u16;
            let duration_ms = 150u64;

            // Simulate structured logging field extraction
            let log_entry = format!(
                "request_id={request_id}, method={method}, path={path}, status={status}, duration_ms={duration_ms}"
            );

            black_box(log_entry);
        });
    });

    group.finish();
}

/// Benchmark complete proxy request flow (without network I/O)
fn bench_complete_proxy_flow_simulation(c: &mut Criterion) {
    let mut group = c.benchmark_group("complete_proxy_flow_simulation");
    group.significance_level(0.05);
    group.measurement_time(Duration::from_secs(15));

    // This benchmark simulates the complete proxy flow without actual networking
    // to measure pure computational overhead and validate <5ms requirement
    group.bench_function("simulated_proxy_request", |b| {
        let config = ProxyConfig::default();
        let ring_buffer = Arc::new(RingBuffer::new(&config.ring_buffer));

        b.iter(|| {
            let start_time = std::time::Instant::now();

            // 1. Request ID generation (middleware)
            let request_id = RequestId::new();

            // 2. Authentication (middleware)
            let _api_key = ApiKey::try_new("sk-test123456789".to_string()).unwrap();

            // 3. Target URL validation
            let _target_url =
                TargetUrl::try_new("https://api.openai.com/v1/chat/completions".to_string())
                    .unwrap();

            // 4. Request metadata extraction
            let method = HttpMethod::try_new("POST".to_string()).unwrap();
            let uri = RequestUri::try_new("/v1/chat/completions".to_string()).unwrap();
            let headers = Headers::from_vec(vec![
                ("content-type".to_string(), "application/json".to_string()),
                (
                    "authorization".to_string(),
                    "Bearer sk-test123456789".to_string(),
                ),
            ])
            .unwrap();

            // 5. Request audit event creation and serialization
            let request_event = AuditEvent {
                request_id,
                session_id: SessionId::new(),
                timestamp: chrono::Utc::now(),
                event_type: AuditEventType::RequestReceived {
                    method,
                    uri,
                    headers,
                    body_size: BodySize::from(1024),
                },
            };
            let request_serialized = serde_json::to_vec(&request_event).unwrap();

            // 6. Ring buffer write (fire-and-forget)
            let _ = ring_buffer.write(request_id, &request_serialized);

            // 7. Response simulation (what would come back from target)
            let response_status = HttpStatusCode::try_new(200).unwrap();
            let response_headers = Headers::from_vec(vec![(
                "content-type".to_string(),
                "application/json".to_string(),
            )])
            .unwrap();

            // 8. Response audit event creation and serialization
            let duration_ms = DurationMillis::from(start_time.elapsed().as_millis() as u64);
            let response_event = AuditEvent {
                request_id,
                session_id: SessionId::new(),
                timestamp: chrono::Utc::now(),
                event_type: AuditEventType::ResponseReceived {
                    status: response_status,
                    headers: response_headers,
                    body_size: BodySize::from(2048),
                    duration_ms,
                },
            };
            let response_serialized = serde_json::to_vec(&response_event).unwrap();

            // 9. Ring buffer write for response
            let _ = ring_buffer.write(request_id, &response_serialized);

            black_box((
                request_serialized,
                response_serialized,
                start_time.elapsed(),
            ));
        });
    });

    // Benchmark to specifically validate <5ms requirement
    group.bench_function("latency_requirement_validation", |b| {
        let config = ProxyConfig::default();
        let ring_buffer = Arc::new(RingBuffer::new(&config.ring_buffer));

        b.iter_custom(|iters| {
            let mut total_duration = Duration::ZERO;
            let mut max_duration = Duration::ZERO;
            let latency_target = Duration::from_millis(5);

            for _ in 0..iters {
                let start = std::time::Instant::now();

                // Critical path operations only
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

                let elapsed = start.elapsed();
                total_duration += elapsed;
                max_duration = max_duration.max(elapsed);

                // Validate that we're meeting the <5ms requirement
                // This will show up in benchmark results if we exceed it
                if elapsed > latency_target {
                    eprintln!("⚠️  Latency target exceeded: {elapsed:?} > {latency_target:?}");
                }
            }

            // Report max latency for percentile analysis
            if max_duration > latency_target {
                eprintln!("⚠️  Max latency in batch: {max_duration:?}");
            }

            total_duration
        });
    });

    group.finish();
}

/// Benchmark error handling performance
fn bench_error_handling_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("error_handling_performance");

    // Benchmark error creation and conversion overhead
    group.bench_function("error_creation_and_conversion", |b| {
        b.iter(|| {
            // Test different error types that occur in hot path
            let timeout_error = ProxyError::RequestTimeout(Duration::from_secs(30));
            let invalid_url_error = ProxyError::InvalidTargetUrl("not-a-url".to_string());
            let size_error = ProxyError::RequestTooLarge {
                size: BodySize::from(10_000_000),
                max_size: RequestSizeLimit::try_new(5_000_000).unwrap(),
            };

            // Convert to HTTP responses (what happens in middleware)
            use axum::response::IntoResponse;
            let timeout_response = timeout_error.into_response();
            let url_response = invalid_url_error.into_response();
            let size_response = size_error.into_response();

            black_box((timeout_response, url_response, size_response));
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
    bench_memory_allocation,
    bench_streaming_service_performance,
    bench_middleware_stack_performance,
    bench_complete_proxy_flow_simulation,
    bench_error_handling_performance
);

criterion_main!(benches);
