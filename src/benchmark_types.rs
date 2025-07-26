//! Domain types for benchmarking and performance testing
//!
//! This module provides type-safe wrappers for benchmark configuration
//! values, ensuring that invalid benchmark parameters cannot be constructed.

use nutype::nutype;
use std::time::Duration;

/// Maximum number of tasks that can be spawned before draining completed ones
#[nutype(
    validate(greater = 0),
    derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)
)]
pub struct TaskBatchThreshold(usize);

/// Maximum number of concurrent tasks allowed in burst tests
#[nutype(
    validate(greater = 0),
    derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)
)]
pub struct MaxConcurrentTasks(usize);

/// Error threshold as a percentage (0-100)
#[nutype(
    validate(less_or_equal = 100),
    derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)
)]
pub struct ErrorThresholdPercent(u8);

/// Target requests per second for load tests
#[nutype(
    validate(greater = 0),
    derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)
)]
pub struct TargetRps(u32);

/// Duration for running a test
#[nutype(
    validate(predicate = |d| d.as_secs() > 0),
    derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)
)]
pub struct TestDuration(Duration);

/// Size of payload in bytes
#[nutype(
    validate(greater = 0),
    derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)
)]
pub struct PayloadSize(usize);

/// Number of concurrent users to simulate
#[nutype(
    validate(greater = 0),
    derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)
)]
pub struct ConcurrentUsers(usize);

/// Maximum number of database connections in pool
#[nutype(
    validate(greater = 0),
    derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)
)]
pub struct MaxConnections(u32);

/// Minimum number of database connections in pool
#[nutype(
    validate(greater = 0),
    derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)
)]
pub struct MinConnections(u32);

/// Operations per millisecond rate
#[nutype(validate(finite), derive(Debug, Clone, Copy, PartialEq, PartialOrd))]
pub struct OpsPerMillisecond(f64);

/// Tolerance for RPS measurements (0.0 to 1.0)
#[nutype(
    validate(greater_or_equal = 0.0, less_or_equal = 1.0),
    derive(Debug, Clone, Copy, PartialEq, PartialOrd)
)]
pub struct RpsTolerance(f64);

/// Latency threshold for performance requirements
#[nutype(
    validate(predicate = |d| d.as_nanos() > 0),
    derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)
)]
pub struct LatencyThreshold(Duration);

/// Number of iterations for benchmarks
#[nutype(
    validate(greater = 0),
    derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)
)]
pub struct BenchmarkIterations(u32);

/// Number of threads for concurrent tests
#[nutype(
    validate(greater = 0),
    derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)
)]
pub struct ThreadCount(usize);

/// Number of operations per thread
#[nutype(
    validate(greater = 0),
    derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)
)]
pub struct OperationsPerThread(u32);

/// Timeout for concurrency tests
#[nutype(
    validate(predicate = |d| d.as_millis() > 0),
    derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)
)]
pub struct ConcurrencyTestTimeout(Duration);

/// URL for database connections
#[nutype(
    validate(predicate = |s: &str| !s.is_empty() && s.starts_with("postgres://")),
    derive(Debug, Clone, PartialEq, Eq, Hash)
)]
pub struct DatabaseUrl(String);

// Common benchmark payload sizes
impl PayloadSize {
    /// 1KB payload - typical small request
    pub fn one_kb() -> Self {
        Self::try_new(1024).unwrap()
    }

    /// 10KB payload - medium request
    pub fn ten_kb() -> Self {
        Self::try_new(10 * 1024).unwrap()
    }

    /// 64KB payload - large request
    pub fn sixty_four_kb() -> Self {
        Self::try_new(64 * 1024).unwrap()
    }

    /// 128KB payload - maximum slot size
    pub fn one_twenty_eight_kb() -> Self {
        Self::try_new(128 * 1024).unwrap()
    }
}

// Common test durations
impl TestDuration {
    /// 10 second test duration
    pub fn ten_seconds() -> Result<Self, TestDurationError> {
        Self::try_new(Duration::from_secs(10))
    }

    /// 20 second test duration
    pub fn twenty_seconds() -> Result<Self, TestDurationError> {
        Self::try_new(Duration::from_secs(20))
    }

    /// 30 second test duration
    pub fn thirty_seconds() -> Result<Self, TestDurationError> {
        Self::try_new(Duration::from_secs(30))
    }
}

// Common latency thresholds
impl LatencyThreshold {
    /// 1ms latency threshold
    pub fn one_ms() -> Result<Self, LatencyThresholdError> {
        Self::try_new(Duration::from_millis(1))
    }

    /// 5ms latency threshold (P99 requirement)
    pub fn five_ms() -> Result<Self, LatencyThresholdError> {
        Self::try_new(Duration::from_millis(5))
    }
}

// Common RPS tolerances
impl RpsTolerance {
    /// 5% tolerance (0.95 multiplier)
    pub fn five_percent() -> Self {
        Self::try_new(0.95).unwrap()
    }

    /// 10% tolerance (0.90 multiplier)
    pub fn ten_percent() -> Self {
        Self::try_new(0.90).unwrap()
    }
}

// Helper to calculate operations per millisecond
impl From<TargetRps> for OpsPerMillisecond {
    fn from(rps: TargetRps) -> Self {
        // Safe because TargetRps is always > 0, so result is finite
        Self::try_new(rps.into_inner() as f64 / 1000.0).unwrap()
    }
}
