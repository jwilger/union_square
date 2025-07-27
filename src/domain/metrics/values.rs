//! Domain value types for metrics calculations
//!
//! Provides validated types for metric values and thresholds used in calculations.

use nutype::nutype;

/// A generic metric value (0.0 to 1.0) used in trend analysis
///
/// Represents any normalized metric value such as F-scores, precision, recall, etc.
/// All values are constrained to the valid range [0.0, 1.0].
#[nutype(
    validate(finite, greater_or_equal = 0.0, less_or_equal = 1.0),
    derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)
)]
pub struct MetricValue(f64);

impl MetricValue {
    /// Perfect metric value (1.0)
    pub fn perfect() -> Self {
        Self::try_new(1.0).unwrap()
    }

    /// Zero metric value (0.0)
    pub fn zero() -> Self {
        Self::try_new(0.0).unwrap()
    }

    /// High quality threshold (0.8)
    pub fn high_quality() -> Self {
        Self::try_new(0.8).unwrap()
    }

    /// Medium quality threshold (0.6)
    pub fn medium_quality() -> Self {
        Self::try_new(0.6).unwrap()
    }

    /// Low quality threshold (0.4)
    pub fn low_quality() -> Self {
        Self::try_new(0.4).unwrap()
    }

    /// Calculate percentage change between two metric values
    pub fn percentage_change_from(&self, other: MetricValue) -> Option<PercentageChange> {
        if other.into_inner() == 0.0 {
            return None;
        }
        let change = (self.into_inner() - other.into_inner()) / other.into_inner();
        PercentageChange::try_new(change).ok()
    }
}

/// Threshold for determining if a trend is stable vs changing
///
/// Represents the minimum change required to consider a metric as trending
/// rather than stable. Typical values range from 0.01 (1%) to 0.1 (10%).
#[nutype(
    validate(finite, greater = 0.0, less_or_equal = 0.5), // Max 50% threshold
    derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)
)]
pub struct StabilityThreshold(f64);

impl StabilityThreshold {
    /// Very sensitive threshold (1%)
    pub fn very_sensitive() -> Self {
        Self::try_new(0.01).unwrap()
    }

    /// Standard threshold (2%)
    pub fn standard() -> Self {
        Self::try_new(0.02).unwrap()
    }

    /// Conservative threshold (5%)
    pub fn conservative() -> Self {
        Self::try_new(0.05).unwrap()
    }

    /// Relaxed threshold (10%)
    pub fn relaxed() -> Self {
        Self::try_new(0.1).unwrap()
    }

    /// Check if a change exceeds this threshold
    pub fn is_significant_change(&self, change: f64) -> bool {
        change.abs() > self.into_inner()
    }
}

/// Percentage change value (-100% to +∞)
///
/// Represents the relative change between two values as a percentage.
/// Negative values indicate decline, positive indicate improvement.
#[nutype(
    validate(finite, greater_or_equal = -1.0), // -100% minimum (complete loss)
    derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)
)]
pub struct PercentageChange(f64);

impl PercentageChange {
    /// No change (0%)
    pub fn none() -> Self {
        Self::try_new(0.0).unwrap()
    }

    /// Small improvement (5%)
    pub fn small_improvement() -> Self {
        Self::try_new(0.05).unwrap()
    }

    /// Large improvement (25%)
    pub fn large_improvement() -> Self {
        Self::try_new(0.25).unwrap()
    }

    /// Small decline (-5%)
    pub fn small_decline() -> Self {
        Self::try_new(-0.05).unwrap()
    }

    /// Large decline (-25%)
    pub fn large_decline() -> Self {
        Self::try_new(-0.25).unwrap()
    }

    /// Complete loss (-100%)
    pub fn complete_loss() -> Self {
        Self::try_new(-1.0).unwrap()
    }

    /// Convert to percentage points (multiply by 100)
    pub fn as_percentage_points(&self) -> f64 {
        self.into_inner() * 100.0
    }

    /// Check if this represents an improvement
    pub fn is_improvement(&self) -> bool {
        self.into_inner() > 0.0
    }

    /// Check if this represents a decline
    pub fn is_decline(&self) -> bool {
        self.into_inner() < 0.0
    }

    /// Check if this represents no significant change
    pub fn is_stable(&self, threshold: StabilityThreshold) -> bool {
        threshold.is_significant_change(self.into_inner())
    }
}

impl Default for StabilityThreshold {
    fn default() -> Self {
        Self::try_new(0.02).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metric_value_validation() {
        assert!(MetricValue::try_new(0.0).is_ok());
        assert!(MetricValue::try_new(0.5).is_ok());
        assert!(MetricValue::try_new(1.0).is_ok());
        assert!(MetricValue::try_new(-0.1).is_err());
        assert!(MetricValue::try_new(1.1).is_err());
    }

    #[test]
    fn test_stability_threshold_validation() {
        assert!(StabilityThreshold::try_new(0.01).is_ok());
        assert!(StabilityThreshold::try_new(0.1).is_ok());
        assert!(StabilityThreshold::try_new(0.5).is_ok());
        assert!(StabilityThreshold::try_new(0.0).is_err()); // Must be > 0
        assert!(StabilityThreshold::try_new(0.6).is_err()); // Max 50%
    }

    #[test]
    fn test_percentage_change_calculation() {
        let current = MetricValue::try_new(0.9).unwrap();
        let previous = MetricValue::try_new(0.8).unwrap();

        let change = current.percentage_change_from(previous).unwrap();
        assert!((change.as_percentage_points() - 12.5).abs() < 1e-10); // 12.5% improvement
        assert!(change.is_improvement());
    }

    #[test]
    fn test_significance_checking() {
        let threshold = StabilityThreshold::default(); // 2%

        assert!(threshold.is_significant_change(0.05)); // 5% change is significant
        assert!(!threshold.is_significant_change(0.01)); // 1% change is not significant
    }

    #[test]
    fn test_semantic_constructors() {
        assert_eq!(MetricValue::perfect().into_inner(), 1.0);
        assert_eq!(MetricValue::zero().into_inner(), 0.0);
        assert_eq!(StabilityThreshold::default().into_inner(), 0.02);
        assert_eq!(PercentageChange::none().into_inner(), 0.0);
    }
}
