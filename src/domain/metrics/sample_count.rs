//! Sample count domain type for statistical significance validation

use crate::domain::metrics::constants;
use nutype::nutype;
use serde::{Deserialize, Serialize};

/// Sample count for statistical measurements
///
/// Represents the number of samples used in a metric calculation.
/// Must be greater than 0 for statistical validity.
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
        Serialize,
        Deserialize,
        Hash
    )
)]
pub struct SampleCount(u64);

impl SampleCount {
    /// Small sample size (may have higher variance)
    pub fn small() -> Self {
        Self::try_new(50).unwrap()
    }

    /// Medium sample size (moderate confidence)
    pub fn medium() -> Self {
        Self::try_new(250).unwrap()
    }

    /// Large sample size (high confidence)
    pub fn large() -> Self {
        Self::try_new(1000).unwrap()
    }

    /// Tiny sample size (low confidence, for demo only)
    pub fn tiny() -> Self {
        Self::try_new(10).unwrap()
    }

    /// Check if sample size provides reliable statistics
    pub fn is_statistically_significant(&self) -> bool {
        self.into_inner() >= constants::statistical::MIN_SIGNIFICANT_SAMPLE_SIZE
    }

    /// Get confidence level based on sample size
    pub fn confidence_category(&self) -> SampleConfidence {
        match self.into_inner() {
            n if n < constants::statistical::MIN_SIGNIFICANT_SAMPLE_SIZE => SampleConfidence::Low,
            n if n < constants::statistical::RECOMMENDED_MIN_SAMPLE_SIZE => {
                SampleConfidence::Moderate
            }
            n if n < constants::statistical::LARGE_SAMPLE_THRESHOLD => SampleConfidence::High,
            _ => SampleConfidence::VeryHigh,
        }
    }
}

/// Confidence level based on sample size
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SampleConfidence {
    /// Low confidence (< 30 samples)
    Low,
    /// Moderate confidence (30-99 samples)
    Moderate,
    /// High confidence (100-999 samples)
    High,
    /// Very high confidence (1000+ samples)
    VeryHigh,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sample_count_validation() {
        assert!(SampleCount::try_new(1).is_ok());
        assert!(SampleCount::try_new(1000).is_ok());
        assert!(SampleCount::try_new(0).is_err());
    }

    #[test]
    fn test_statistical_significance() {
        assert!(!SampleCount::try_new(10)
            .unwrap()
            .is_statistically_significant());
        assert!(SampleCount::try_new(30)
            .unwrap()
            .is_statistically_significant());
        assert!(SampleCount::try_new(100)
            .unwrap()
            .is_statistically_significant());
    }

    #[test]
    fn test_confidence_categories() {
        assert_eq!(
            SampleCount::try_new(10).unwrap().confidence_category(),
            SampleConfidence::Low
        );
        assert_eq!(
            SampleCount::try_new(50).unwrap().confidence_category(),
            SampleConfidence::Moderate
        );
        assert_eq!(
            SampleCount::try_new(250).unwrap().confidence_category(),
            SampleConfidence::High
        );
    }
}
