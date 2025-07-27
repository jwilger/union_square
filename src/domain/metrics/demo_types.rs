//! Domain types specific to demo data generation
//!
//! This module provides type-safe representations for demo data generation
//! parameters, ensuring all trend rates, frequencies, and other demo-specific
//! values are properly validated and documented.

use nutype::nutype;

/// Daily trend rate for metrics in demo data
///
/// Represents the rate of change per day for precision/recall metrics.
/// Positive values indicate improvement, negative values indicate decline.
/// Range: -0.01 to 0.01 (±1% per day maximum)
#[nutype(
    validate(finite, greater_or_equal = -0.01, less_or_equal = 0.01),
    derive(
        Debug,
        Clone,
        Copy,
        PartialEq,
        PartialOrd,
        Serialize,
        Deserialize,
        AsRef,
        Display
    )
)]
pub struct DailyTrendRate(f64);

impl DailyTrendRate {
    /// No change trend
    pub fn zero() -> Self {
        Self::try_new(0.0).unwrap()
    }

    /// Standard improvement rate (0.1% per day)
    pub fn improving() -> Self {
        Self::try_new(0.001).unwrap()
    }

    /// Slight decline rate (-0.05% per day)
    pub fn slight_decline() -> Self {
        Self::try_new(-0.0005).unwrap()
    }

    /// Rapid decline rate (-0.2% per day)
    pub fn rapid_decline() -> Self {
        Self::try_new(-0.002).unwrap()
    }
}

/// Variance amplitude for realistic metric fluctuation
///
/// Controls how much metrics can vary from their trend line.
/// Range: 0.0 to 0.1 (up to 10% variance)
#[nutype(
    validate(finite, greater_or_equal = 0.0, less_or_equal = 0.1),
    derive(
        Debug,
        Clone,
        Copy,
        PartialEq,
        PartialOrd,
        Serialize,
        Deserialize,
        AsRef,
        Display
    )
)]
pub struct VarianceAmplitude(f64);

impl VarianceAmplitude {
    /// No variance
    pub fn zero() -> Self {
        Self::try_new(0.0).unwrap()
    }

    /// Standard precision variance (2%)
    pub fn precision_standard() -> Self {
        Self::try_new(0.02).unwrap()
    }

    /// Standard recall variance (1.5%)
    pub fn recall_standard() -> Self {
        Self::try_new(0.015).unwrap()
    }
}

/// Wave frequency for variance patterns
///
/// Controls the frequency of sinusoidal variance in demo data.
/// Higher values create more frequent oscillations.
/// Range: 1.0 to 30.0 (cycles per time period)
#[nutype(
    validate(finite, greater_or_equal = 1.0, less_or_equal = 30.0),
    derive(
        Debug,
        Clone,
        Copy,
        PartialEq,
        PartialOrd,
        Serialize,
        Deserialize,
        AsRef,
        Display
    )
)]
pub struct WaveFrequency(f64);

impl WaveFrequency {
    /// Low frequency (1 cycle)
    pub fn low() -> Self {
        Self::try_new(1.0).unwrap()
    }

    /// Medium frequency (5 cycles)
    pub fn medium() -> Self {
        Self::try_new(5.0).unwrap()
    }

    /// High frequency (7 cycles)
    pub fn high() -> Self {
        Self::try_new(7.0).unwrap()
    }

    /// Standard precision wave frequency
    pub fn precision_standard() -> Self {
        Self::try_new(7.0).unwrap()
    }

    /// Standard recall wave frequency
    pub fn recall_standard() -> Self {
        Self::try_new(5.0).unwrap()
    }
}

/// Trend factor for related metrics
///
/// Represents how one metric's trend affects another.
/// Range: 0.0 to 2.0 (0% to 200% of primary trend)
#[nutype(
    validate(finite, greater_or_equal = 0.0, less_or_equal = 2.0),
    derive(
        Debug,
        Clone,
        Copy,
        PartialEq,
        PartialOrd,
        Serialize,
        Deserialize,
        AsRef,
        Display
    )
)]
pub struct TrendFactor(f64);

impl TrendFactor {
    /// No correlation
    pub fn zero() -> Self {
        Self::try_new(0.0).unwrap()
    }

    /// Full correlation (100%)
    pub fn full() -> Self {
        Self::try_new(1.0).unwrap()
    }

    /// Standard recall correlation to precision (80%)
    pub fn recall_to_precision() -> Self {
        Self::try_new(0.8).unwrap()
    }
}

/// Sample count increment for demo data variation
#[nutype(
    validate(greater = 0, less_or_equal = 1000),
    derive(
        Debug,
        Clone,
        Copy,
        PartialEq,
        Eq,
        PartialOrd,
        Ord,
        Serialize,
        Deserialize,
        Hash
    )
)]
pub struct SampleIncrement(u64);

impl SampleIncrement {
    /// Base increment value
    pub fn base() -> Self {
        Self::try_new(50).unwrap()
    }

    /// Small increment
    pub fn small() -> Self {
        Self::try_new(10).unwrap()
    }

    /// Large increment
    pub fn large() -> Self {
        Self::try_new(100).unwrap()
    }
}

/// Cycle length for sample count variation
#[nutype(
    validate(greater = 0, less_or_equal = 10),
    derive(
        Debug,
        Clone,
        Copy,
        PartialEq,
        Eq,
        PartialOrd,
        Ord,
        Serialize,
        Deserialize,
        Hash
    )
)]
pub struct CycleLength(i64);

impl CycleLength {
    /// Standard 3-day cycle
    pub fn standard() -> Self {
        Self::try_new(3).unwrap()
    }

    /// Weekly cycle
    pub fn weekly() -> Self {
        Self::try_new(7).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_daily_trend_rate_validation() {
        assert!(DailyTrendRate::try_new(0.005).is_ok());
        assert!(DailyTrendRate::try_new(-0.005).is_ok());
        assert!(DailyTrendRate::try_new(0.02).is_err()); // Too high
        assert!(DailyTrendRate::try_new(-0.02).is_err()); // Too low
    }

    #[test]
    fn test_variance_amplitude_validation() {
        assert!(VarianceAmplitude::try_new(0.0).is_ok());
        assert!(VarianceAmplitude::try_new(0.05).is_ok());
        assert!(VarianceAmplitude::try_new(0.15).is_err()); // Too high
        assert!(VarianceAmplitude::try_new(-0.01).is_err()); // Negative
    }

    #[test]
    fn test_wave_frequency_validation() {
        assert!(WaveFrequency::try_new(1.0).is_ok());
        assert!(WaveFrequency::try_new(15.0).is_ok());
        assert!(WaveFrequency::try_new(0.5).is_err()); // Too low
        assert!(WaveFrequency::try_new(35.0).is_err()); // Too high
    }

    #[test]
    fn test_trend_factor_validation() {
        assert!(TrendFactor::try_new(0.0).is_ok());
        assert!(TrendFactor::try_new(1.0).is_ok());
        assert!(TrendFactor::try_new(2.5).is_err()); // Too high
        assert!(TrendFactor::try_new(-0.1).is_err()); // Negative
    }
}
