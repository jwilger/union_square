//! Network and URL-related domain types
//!
//! This module provides type-safe wrappers for network-related values
//! to prevent common errors and enforce validation.

use nutype::nutype;

/// Maximum URL length for practical use and security
pub const MAX_URL_LENGTH: usize = 2048;

/// Timeout duration in milliseconds
#[nutype(
    validate(greater = 0, less_or_equal = 300000), // Max 5 minutes
    derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)
)]
pub struct TimeoutMs(u32);

impl TimeoutMs {
    /// Default HTTP timeout (30 seconds)
    pub fn default_http() -> Self {
        Self::try_new(30_000).unwrap()
    }

    /// Short timeout for health checks (5 seconds)
    pub fn health_check() -> Self {
        Self::try_new(5_000).unwrap()
    }

    /// Long timeout for large uploads (2 minutes)
    pub fn long_operation() -> Self {
        Self::try_new(120_000).unwrap()
    }
}

/// Buffer size for network operations
#[nutype(
    validate(greater = 0),
    derive(
        Debug,
        Clone,
        Copy,
        PartialEq,
        Eq,
        PartialOrd,
        Ord,
        Hash,
        Serialize,
        Deserialize
    )
)]
pub struct BufferSize(usize);

impl BufferSize {
    /// 16KB buffer (common for HTTP)
    pub fn sixteen_kb() -> Self {
        Self::try_new(16 * 1024).unwrap()
    }

    /// 64KB buffer (large transfers)
    pub fn sixty_four_kb() -> Self {
        Self::try_new(64 * 1024).unwrap()
    }

    /// 1MB buffer (very large transfers)
    pub fn one_mb() -> Self {
        Self::try_new(1024 * 1024).unwrap()
    }

    /// 4MB buffer (performance testing)
    pub fn four_mb() -> Self {
        Self::try_new(4 * 1024 * 1024).unwrap()
    }
}

/// Slot size for ring buffers
#[nutype(
    validate(greater = 0),
    derive(
        Debug,
        Clone,
        Copy,
        PartialEq,
        Eq,
        PartialOrd,
        Ord,
        Hash,
        Serialize,
        Deserialize
    )
)]
pub struct SlotSize(usize);

impl SlotSize {
    /// 1KB slot (small messages)
    pub fn one_kb() -> Self {
        Self::try_new(1024).unwrap()
    }

    /// 4KB slot (medium messages)
    pub fn four_kb() -> Self {
        Self::try_new(4 * 1024).unwrap()
    }

    /// 16KB slot (large messages)
    pub fn sixteen_kb() -> Self {
        Self::try_new(16 * 1024).unwrap()
    }
}

/// Maximum URL length validation
#[nutype(
    validate(greater = 0, less_or_equal = 10000), // Reasonable maximum
    derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)
)]
pub struct MaxUrlLength(usize);

impl MaxUrlLength {
    /// Standard web URL limit
    pub fn standard() -> Self {
        Self::try_new(2048).unwrap()
    }

    /// Extended URL limit for APIs
    pub fn extended() -> Self {
        Self::try_new(4096).unwrap()
    }
}

/// Performance requirements and thresholds
#[nutype(
    validate(greater = 0.0),
    derive(Debug, Clone, Copy, PartialEq, PartialOrd)
)]
pub struct PerformanceThresholdNs(f64);

impl PerformanceThresholdNs {
    /// Sub-microsecond threshold (1000ns = 1μs)
    pub fn one_microsecond() -> Self {
        Self::try_new(1000.0).unwrap()
    }

    /// Very fast threshold (100ns)
    pub fn one_hundred_ns() -> Self {
        Self::try_new(100.0).unwrap()
    }

    /// Ultra-fast threshold (50ns)
    pub fn fifty_ns() -> Self {
        Self::try_new(50.0).unwrap()
    }
}

/// Thread count for concurrent operations
#[nutype(
    validate(greater = 0, less_or_equal = 1000), // Reasonable maximum
    derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)
)]
pub struct ThreadCount(usize);

impl ThreadCount {
    /// Single thread
    pub fn single() -> Self {
        Self::try_new(1).unwrap()
    }

    /// CPU core count (typical 4-16)
    pub fn cpu_cores() -> Self {
        let cores = std::thread::available_parallelism()
            .map(|p| p.get())
            .unwrap_or(4);
        Self::try_new(cores).unwrap()
    }

    /// High concurrency testing
    pub fn high_concurrency() -> Self {
        Self::try_new(100).unwrap()
    }
}

/// Write operations count
#[nutype(
    validate(greater = 0),
    derive(
        Debug,
        Clone,
        Copy,
        PartialEq,
        Eq,
        PartialOrd,
        Ord,
        Hash,
        Serialize,
        Deserialize
    )
)]
pub struct WriteCount(usize);

impl WriteCount {
    /// Small test batch
    pub fn small_batch() -> Self {
        Self::try_new(1000).unwrap()
    }

    /// Medium test batch
    pub fn medium_batch() -> Self {
        Self::try_new(10_000).unwrap()
    }

    /// Large test batch
    pub fn large_batch() -> Self {
        Self::try_new(100_000).unwrap()
    }
}

/// Data size in bytes
#[nutype(
    validate(greater = 0),
    derive(
        Debug,
        Clone,
        Copy,
        PartialEq,
        Eq,
        PartialOrd,
        Ord,
        Hash,
        Serialize,
        Deserialize
    )
)]
pub struct DataSize(usize);

impl DataSize {
    /// 512 bytes
    pub fn five_twelve_bytes() -> Self {
        Self::try_new(512).unwrap()
    }

    /// 1KB data
    pub fn one_kb() -> Self {
        Self::try_new(1024).unwrap()
    }

    /// 64KB data (large HTTP body)
    pub fn sixty_four_kb() -> Self {
        Self::try_new(64 * 1024).unwrap()
    }

    /// 1MB data (very large)
    pub fn one_mb() -> Self {
        Self::try_new(1024 * 1024).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timeout_creation() {
        assert!(TimeoutMs::try_new(1000).is_ok()); // 1 second
        assert!(TimeoutMs::try_new(0).is_err()); // Invalid
        assert!(TimeoutMs::try_new(400_000).is_err()); // Too long
    }

    #[test]
    fn test_timeout_defaults() {
        let default = TimeoutMs::default_http();
        assert_eq!(default.into_inner(), 30_000);

        let health = TimeoutMs::health_check();
        assert_eq!(health.into_inner(), 5_000);

        let long = TimeoutMs::long_operation();
        assert_eq!(long.into_inner(), 120_000);
    }

    #[test]
    fn test_buffer_sizes() {
        let sixteen = BufferSize::sixteen_kb();
        assert_eq!(sixteen.into_inner(), 16 * 1024);

        let mb = BufferSize::one_mb();
        assert_eq!(mb.into_inner(), 1024 * 1024);
    }

    #[test]
    fn test_slot_sizes() {
        let kb = SlotSize::one_kb();
        assert_eq!(kb.into_inner(), 1024);

        let four = SlotSize::four_kb();
        assert_eq!(four.into_inner(), 4 * 1024);
    }

    #[test]
    fn test_max_url_length() {
        let standard = MaxUrlLength::standard();
        assert_eq!(standard.into_inner(), 2048);

        let extended = MaxUrlLength::extended();
        assert_eq!(extended.into_inner(), 4096);
    }

    #[test]
    fn test_performance_thresholds() {
        let micro = PerformanceThresholdNs::one_microsecond();
        assert_eq!(micro.into_inner(), 1000.0);

        let hundred = PerformanceThresholdNs::one_hundred_ns();
        assert_eq!(hundred.into_inner(), 100.0);
    }

    #[test]
    fn test_thread_count() {
        let single = ThreadCount::single();
        assert_eq!(single.into_inner(), 1);

        let cores = ThreadCount::cpu_cores();
        assert!(cores.into_inner() >= 1);
        assert!(cores.into_inner() <= 1000);

        let high = ThreadCount::high_concurrency();
        assert_eq!(high.into_inner(), 100);
    }

    #[test]
    fn test_write_count() {
        let small = WriteCount::small_batch();
        assert_eq!(small.into_inner(), 1000);

        let large = WriteCount::large_batch();
        assert_eq!(large.into_inner(), 100_000);
    }

    #[test]
    fn test_data_size() {
        let bytes = DataSize::five_twelve_bytes();
        assert_eq!(bytes.into_inner(), 512);

        let kb = DataSize::one_kb();
        assert_eq!(kb.into_inner(), 1024);

        let mb = DataSize::one_mb();
        assert_eq!(mb.into_inner(), 1024 * 1024);
    }
}
