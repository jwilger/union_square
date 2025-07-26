//! Load testing scenarios for MVP performance targets
//!
//! Run with: cargo test --test load_testing --release -- --nocapture --test-threads=1
//!
//! Note: Load tests are marked as #[ignore] because they require significant resources
//! and proper configuration to run successfully. Run them explicitly with:
//! cargo test --test load_testing -- --ignored --nocapture --test-threads=1

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::task::JoinSet;
use union_square::proxy::storage::RingBuffer;
use union_square::proxy::types::*;

/// MVP Target: 500 RPS sustained load test
#[tokio::test]
#[ignore] // Run explicitly with: cargo test --test load_testing test_500_rps_sustained_load -- --ignored
async fn test_500_rps_sustained_load() {
    println!("\n=== Testing 500 RPS Sustained Load (30 seconds) ===");

    let config = RingBufferConfig {
        buffer_size: BufferSize::try_new(256 * 1024 * 1024).expect("256MB is valid"),
        slot_size: SlotSize::try_new(128 * 1024).expect("128KB is valid"),
    };
    let ring_buffer = Arc::new(RingBuffer::new(&config));

    let target_rps = 500;
    let duration = Duration::from_secs(30);
    let operations = Arc::new(AtomicU64::new(0));
    let errors = Arc::new(AtomicU64::new(0));

    let start = Instant::now();
    let mut tasks = JoinSet::new();

    // Calculate operations per millisecond to maintain steady rate
    let ops_per_ms = target_rps as f64 / 1000.0;
    let mut next_op_time = start;

    while start.elapsed() < duration {
        let rb = ring_buffer.clone();
        let ops = operations.clone();
        let errs = errors.clone();

        tasks.spawn(async move {
            let request_id = RequestId::new();
            let data = vec![b'x'; 1024]; // 1KB payload

            match rb.write(request_id, &data) {
                Ok(_) => {
                    ops.fetch_add(1, Ordering::Relaxed);
                }
                Err(_) => {
                    errs.fetch_add(1, Ordering::Relaxed);
                }
            }
        });

        // Maintain steady rate
        next_op_time += Duration::from_micros((1000.0 / ops_per_ms) as u64);
        if let Some(sleep_duration) = next_op_time.checked_duration_since(Instant::now()) {
            tokio::time::sleep(sleep_duration).await;
        }
    }

    // Wait for all tasks to complete
    while tasks.join_next().await.is_some() {}

    let total_duration = start.elapsed();
    let total_ops = operations.load(Ordering::Relaxed);
    let total_errors = errors.load(Ordering::Relaxed);
    let actual_rps = total_ops as f64 / total_duration.as_secs_f64();

    println!("Duration: {total_duration:?}");
    println!("Total operations: {total_ops}");
    println!("Total errors: {total_errors}");
    println!("Actual RPS: {actual_rps:.2}");

    // Verify we achieved target RPS (within 5% tolerance)
    assert!(
        actual_rps >= target_rps as f64 * 0.95,
        "Failed to achieve target RPS: {:.2} < {}",
        actual_rps,
        target_rps as f64 * 0.95
    );
    assert_eq!(
        total_errors, 0,
        "Errors occurred during sustained load test"
    );
}

/// MVP Target: 2000 RPS burst load test
#[tokio::test]
#[ignore] // Run explicitly with: cargo test --test load_testing test_2000_rps_burst_load -- --ignored
async fn test_2000_rps_burst_load() {
    println!("\n=== Testing 2000 RPS Burst Load (10 seconds) ===");

    let config = RingBufferConfig {
        buffer_size: BufferSize::try_new(512 * 1024 * 1024).expect("512MB is valid"),
        slot_size: SlotSize::try_new(128 * 1024).expect("128KB is valid"),
    };
    let ring_buffer = Arc::new(RingBuffer::new(&config));

    let target_rps = 2000;
    let duration = Duration::from_secs(10);
    let operations = Arc::new(AtomicU64::new(0));
    let errors = Arc::new(AtomicU64::new(0));
    let mut latencies = Vec::new();

    let start = Instant::now();
    let mut tasks = JoinSet::new();

    // Burst pattern: generate all operations quickly
    let total_operations = target_rps * duration.as_secs();

    for _ in 0..total_operations {
        let rb = ring_buffer.clone();
        let ops = operations.clone();
        let errs = errors.clone();

        let op_start = Instant::now();
        tasks.spawn(async move {
            let request_id = RequestId::new();
            let data = vec![b'x'; 1024]; // 1KB payload

            match rb.write(request_id, &data) {
                Ok(_) => {
                    ops.fetch_add(1, Ordering::Relaxed);
                }
                Err(_) => {
                    errs.fetch_add(1, Ordering::Relaxed);
                }
            }

            op_start.elapsed()
        });

        // Small delay to prevent overwhelming the system
        if tasks.len() >= 1000 {
            while let Some(result) = tasks.join_next().await {
                if let Ok(latency) = result {
                    latencies.push(latency);
                }
            }
        }
    }

    // Wait for remaining tasks
    while let Some(result) = tasks.join_next().await {
        if let Ok(latency) = result {
            latencies.push(latency);
        }
    }

    let total_duration = start.elapsed();
    let total_ops = operations.load(Ordering::Relaxed);
    let total_errors = errors.load(Ordering::Relaxed);
    let actual_rps = total_ops as f64 / total_duration.as_secs_f64();

    // Calculate latency percentiles
    latencies.sort();
    let p50 = latencies[latencies.len() / 2];
    let p95 = latencies[latencies.len() * 95 / 100];
    let p99 = latencies[latencies.len() * 99 / 100];

    println!("Duration: {total_duration:?}");
    println!("Total operations: {total_ops}");
    println!("Total errors: {total_errors}");
    println!("Actual RPS: {actual_rps:.2}");
    println!("Latency P50: {p50:?}");
    println!("Latency P95: {p95:?}");
    println!("Latency P99: {p99:?}");

    // Verify performance
    assert!(
        actual_rps >= target_rps as f64 * 0.9,
        "Failed to achieve burst RPS: {:.2} < {}",
        actual_rps,
        target_rps as f64 * 0.9
    );
    assert!(
        p99 < Duration::from_millis(5),
        "P99 latency {p99:?} exceeds 5ms requirement"
    );
    assert_eq!(total_errors, 0, "Errors occurred during burst load test");
}

/// MVP Target: 1000 concurrent users test
#[tokio::test]
#[ignore] // Run explicitly with: cargo test --test load_testing test_1000_concurrent_users -- --ignored
async fn test_1000_concurrent_users() {
    println!("\n=== Testing 1000 Concurrent Users (20 seconds) ===");

    let config = RingBufferConfig {
        buffer_size: BufferSize::try_new(512 * 1024 * 1024).expect("512MB is valid"),
        slot_size: SlotSize::try_new(128 * 1024).expect("128KB is valid"),
    };
    let ring_buffer = Arc::new(RingBuffer::new(&config));

    let num_users = 1000;
    let duration = Duration::from_secs(20);
    let operations = Arc::new(AtomicU64::new(0));
    let errors = Arc::new(AtomicU64::new(0));

    let start = Instant::now();
    let mut tasks = JoinSet::new();

    // Spawn concurrent users
    for user_id in 0..num_users {
        let rb = ring_buffer.clone();
        let ops = operations.clone();
        let errs = errors.clone();
        let user_duration = duration;

        tasks.spawn(async move {
            let user_start = Instant::now();
            let mut user_ops = 0u64;

            // Each user performs operations for the duration
            while user_start.elapsed() < user_duration {
                let request_id = RequestId::new();
                let _session_id = SessionId::new(); // Each user has own session
                let data = format!("User {user_id} request {user_ops}").into_bytes();

                match rb.write(request_id, &data) {
                    Ok(_) => {
                        ops.fetch_add(1, Ordering::Relaxed);
                        user_ops += 1;
                    }
                    Err(_) => {
                        errs.fetch_add(1, Ordering::Relaxed);
                    }
                }

                // Simulate realistic user behavior with small delays
                tokio::time::sleep(Duration::from_millis(10 + (user_id % 10) as u64)).await;
            }

            user_ops
        });
    }

    // Wait for all users to complete
    let mut user_operations = Vec::new();
    while let Some(result) = tasks.join_next().await {
        if let Ok(ops) = result {
            user_operations.push(ops);
        }
    }

    let total_duration = start.elapsed();
    let total_ops = operations.load(Ordering::Relaxed);
    let total_errors = errors.load(Ordering::Relaxed);
    let actual_rps = total_ops as f64 / total_duration.as_secs_f64();

    // Calculate statistics
    let avg_ops_per_user = total_ops as f64 / num_users as f64;
    let min_ops = user_operations.iter().min().unwrap_or(&0);
    let max_ops = user_operations.iter().max().unwrap_or(&0);

    println!("Duration: {total_duration:?}");
    println!("Total operations: {total_ops}");
    println!("Total errors: {total_errors}");
    println!("Overall RPS: {actual_rps:.2}");
    println!("Average ops per user: {avg_ops_per_user:.2}");
    println!("Min ops by user: {min_ops}");
    println!("Max ops by user: {max_ops}");

    // Verify concurrent user handling
    assert_eq!(
        user_operations.len(),
        num_users,
        "Not all users completed: {} < {}",
        user_operations.len(),
        num_users
    );
    assert!(*min_ops > 0, "Some users made no operations");
    assert_eq!(
        total_errors, 0,
        "Errors occurred during concurrent user test"
    );
}

/// Test database connection pool performance under load
#[tokio::test]
#[ignore] // This test requires a running database
async fn test_database_pool_under_load() {
    use sqlx::postgres::PgPoolOptions;

    println!("\n=== Testing Database Connection Pool Under Load ===");

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/union_square".to_string());

    let pool = PgPoolOptions::new()
        .max_connections(100)
        .min_connections(10)
        .connect(&database_url)
        .await
        .expect("Failed to create pool");

    let operations = Arc::new(AtomicU64::new(0));
    let errors = Arc::new(AtomicU64::new(0));
    let duration = Duration::from_secs(10);

    let start = Instant::now();
    let mut tasks = JoinSet::new();

    // Simulate 500 RPS database operations
    let target_rps = 500;
    let ops_per_ms = target_rps as f64 / 1000.0;
    let mut next_op_time = start;

    while start.elapsed() < duration {
        let pool = pool.clone();
        let ops = operations.clone();
        let errs = errors.clone();

        tasks.spawn(async move {
            // Simple query to test pool performance
            match sqlx::query("SELECT 1").execute(&pool).await {
                Ok(_) => {
                    ops.fetch_add(1, Ordering::Relaxed);
                }
                Err(_) => {
                    errs.fetch_add(1, Ordering::Relaxed);
                }
            }
        });

        next_op_time += Duration::from_micros((1000.0 / ops_per_ms) as u64);
        if let Some(sleep_duration) = next_op_time.checked_duration_since(Instant::now()) {
            tokio::time::sleep(sleep_duration).await;
        }
    }

    // Wait for all tasks
    while tasks.join_next().await.is_some() {}

    let total_duration = start.elapsed();
    let total_ops = operations.load(Ordering::Relaxed);
    let total_errors = errors.load(Ordering::Relaxed);
    let actual_rps = total_ops as f64 / total_duration.as_secs_f64();

    println!("Duration: {total_duration:?}");
    println!("Total operations: {total_ops}");
    println!("Total errors: {total_errors}");
    println!("Actual RPS: {actual_rps:.2}");

    // Verify database can handle load
    assert!(
        actual_rps >= target_rps as f64 * 0.9,
        "Database couldn't handle target RPS: {:.2} < {}",
        actual_rps,
        target_rps as f64 * 0.9
    );
    assert!(
        total_errors < total_ops / 100,
        "Too many database errors: {total_errors} errors out of {total_ops} operations"
    );
}
