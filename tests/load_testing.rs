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
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use union_square::benchmark_types::*;
use union_square::proxy::storage::RingBuffer;
use union_square::proxy::types::*;

// Helper functions for load testing configuration
fn task_batch_threshold() -> TaskBatchThreshold {
    TaskBatchThreshold::try_new(1000).expect("1000 is valid threshold")
}

fn max_concurrent_tasks() -> MaxConcurrentTasks {
    MaxConcurrentTasks::try_new(1000).expect("1000 is valid task count")
}

fn db_error_threshold_percent() -> ErrorThresholdPercent {
    ErrorThresholdPercent::try_new(1).expect("1% is valid threshold")
}

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

    let target_rps = TargetRps::try_new(500).expect("500 is valid RPS");
    let duration = TestDuration::thirty_seconds().expect("30s is valid duration");
    let operations = Arc::new(AtomicU64::new(0));
    let errors = Arc::new(AtomicU64::new(0));

    let start = Instant::now();
    let mut tasks = JoinSet::new();

    // Calculate operations per millisecond to maintain steady rate
    let ops_per_ms = OpsPerMillisecond::from(target_rps);
    let mut next_op_time = start;

    while start.elapsed() < duration.into_inner() {
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
        next_op_time += Duration::from_micros((1000.0 / ops_per_ms.into_inner()) as u64);
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
        actual_rps >= target_rps.into_inner() as f64 * RpsTolerance::five_percent().into_inner(),
        "Failed to achieve target RPS: {:.2} < {}",
        actual_rps,
        target_rps.into_inner() as f64 * RpsTolerance::five_percent().into_inner()
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

    let target_rps = TargetRps::try_new(2000).expect("2000 is valid RPS");
    let duration = TestDuration::ten_seconds().expect("10s is valid duration");
    let operations = Arc::new(AtomicU64::new(0));
    let errors = Arc::new(AtomicU64::new(0));
    let mut latencies = Vec::new();

    let start = Instant::now();
    let mut tasks = JoinSet::new();
    let semaphore = Arc::new(Semaphore::new(max_concurrent_tasks().into_inner()));

    // Burst pattern: generate all operations quickly
    let total_operations = target_rps.into_inner() * duration.into_inner().as_secs() as u32;

    for _ in 0..total_operations {
        let rb = ring_buffer.clone();
        let ops = operations.clone();
        let errs = errors.clone();
        let sem = semaphore.clone();

        let op_start = Instant::now();
        tasks.spawn(async move {
            let _permit = sem.acquire().await.expect("semaphore closed");
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

        // Periodically drain completed tasks to avoid unbounded growth
        if tasks.len() >= task_batch_threshold().into_inner() {
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
        actual_rps >= target_rps.into_inner() as f64 * RpsTolerance::ten_percent().into_inner(),
        "Failed to achieve burst RPS: {:.2} < {}",
        actual_rps,
        target_rps.into_inner() as f64 * RpsTolerance::ten_percent().into_inner()
    );
    assert!(
        p99 < LatencyThreshold::five_ms()
            .expect("5ms is valid threshold")
            .into_inner(),
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

    let num_users = ConcurrentUsers::try_new(1000).expect("1000 is valid user count");
    let duration = TestDuration::twenty_seconds().expect("20s is valid duration");
    let operations = Arc::new(AtomicU64::new(0));
    let errors = Arc::new(AtomicU64::new(0));

    let start = Instant::now();
    let mut tasks = JoinSet::new();

    // Spawn concurrent users
    for user_id in 0..num_users.into_inner() {
        let rb = ring_buffer.clone();
        let ops = operations.clone();
        let errs = errors.clone();
        let user_duration = duration;

        tasks.spawn(async move {
            let user_start = Instant::now();
            let mut user_ops = 0u64;

            // Each user performs operations for the duration
            while user_start.elapsed() < user_duration.into_inner() {
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
                let base_delay_ms = 10u64;
                let user_variation = (user_id % 10) as u64;
                tokio::time::sleep(Duration::from_millis(base_delay_ms + user_variation)).await;
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
    let avg_ops_per_user = total_ops as f64 / num_users.into_inner() as f64;
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
        num_users.into_inner(),
        "Not all users completed: {} < {}",
        user_operations.len(),
        num_users.into_inner()
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

    let database_url = DatabaseUrl::try_new(std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://postgres:postgres@localhost:5432/union_square".to_string()
    }))
    .expect("Valid database URL");

    let max_conns = MaxConnections::try_new(100).expect("100 is valid max connections");
    let min_conns = MinConnections::try_new(10).expect("10 is valid min connections");

    let pool = PgPoolOptions::new()
        .max_connections(max_conns.into_inner())
        .min_connections(min_conns.into_inner())
        .connect(&database_url.into_inner())
        .await
        .expect("Failed to create pool");

    let operations = Arc::new(AtomicU64::new(0));
    let errors = Arc::new(AtomicU64::new(0));
    let duration = TestDuration::ten_seconds().expect("10s is valid duration");

    let start = Instant::now();
    let mut tasks = JoinSet::new();

    // Simulate 500 RPS database operations
    let target_rps = TargetRps::try_new(500).expect("500 is valid RPS");
    let ops_per_ms = OpsPerMillisecond::from(target_rps);
    let mut next_op_time = start;

    while start.elapsed() < duration.into_inner() {
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

        next_op_time += Duration::from_micros((1000.0 / ops_per_ms.into_inner()) as u64);
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
        actual_rps >= target_rps.into_inner() as f64 * RpsTolerance::ten_percent().into_inner(),
        "Database couldn't handle target RPS: {:.2} < {}",
        actual_rps,
        target_rps.into_inner() as f64 * RpsTolerance::ten_percent().into_inner()
    );
    assert!(
        total_errors < total_ops * db_error_threshold_percent().into_inner() as u64 / 100,
        "Too many database errors: {total_errors} errors out of {total_ops} operations"
    );
}
