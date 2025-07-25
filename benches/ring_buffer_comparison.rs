//! Benchmark comparing safe ring buffer implementations

use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;
use std::sync::Arc;
use std::thread;
use union_square::proxy::types::*;

// Import all ring buffer implementations
use union_square::proxy::ring_buffer::RingBuffer as UnsafeRingBuffer;

// For now, just benchmark the original unsafe version vs a simple conceptual safe version
// In practice, you'd import the other implementations too

fn benchmark_unsafe_ring_buffer(c: &mut Criterion) {
    let config = RingBufferConfig {
        buffer_size: BufferSize::try_new(1024 * 1024).expect("valid size"), // 1MB
        slot_size: SlotSize::try_new(1024).expect("valid size"),            // 1KB slots
    };

    let buffer = UnsafeRingBuffer::new(&config);
    let request_id = RequestId::new();
    let data = vec![0u8; 512]; // 512 bytes

    c.bench_function("unsafe_ring_buffer_single_write", |b| {
        b.iter(|| {
            let _ = buffer.write(black_box(request_id), black_box(&data));
        })
    });

    c.bench_function("unsafe_ring_buffer_write_read_cycle", |b| {
        b.iter(|| {
            let _ = buffer.write(black_box(request_id), black_box(&data));
            let _ = buffer.read();
        })
    });
}

fn benchmark_concurrent_unsafe_ring_buffer(c: &mut Criterion) {
    let config = RingBufferConfig {
        buffer_size: BufferSize::try_new(1024 * 1024).expect("valid size"),
        slot_size: SlotSize::try_new(1024).expect("valid size"),
    };

    c.bench_function("unsafe_ring_buffer_concurrent_4_threads", |b| {
        b.iter(|| {
            let buffer = Arc::new(UnsafeRingBuffer::new(&config));
            let handles: Vec<_> = (0..4)
                .map(|_| {
                    let buffer_clone = Arc::clone(&buffer);
                    thread::spawn(move || {
                        let request_id = RequestId::new();
                        let data = vec![0u8; 256];
                        for _ in 0..100 {
                            let _ = buffer_clone.write(request_id, &data);
                        }
                    })
                })
                .collect();

            for handle in handles {
                handle.join().unwrap();
            }
        })
    });
}

criterion_group!(
    benches,
    benchmark_unsafe_ring_buffer,
    benchmark_concurrent_unsafe_ring_buffer
);
criterion_main!(benches);
