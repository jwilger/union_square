//! F-score tracking and analytics domain types
//!
//! This module provides type-safe F-score calculation and tracking functionality
//! following type-driven development principles for precision/recall metrics.

pub mod constants;
pub mod counts;
pub mod demo_data;
pub mod performance;
pub mod sample_count;
pub mod time_period;
pub mod timestamp;
pub mod trend;
pub mod ui_types;
pub mod values;

// Re-export commonly used types
pub use counts::{ApplicationCount, DataPointCount, ModelCount};
pub use performance::{PerformanceAssessment, PerformanceLevel, QualityRating};
pub use sample_count::{SampleConfidence, SampleCount};
pub use time_period::{DaysBack, PointsPerDay, TimePeriod};
pub use timestamp::{Timestamp, TimestampAge};
pub use trend::{TrendAnalysis, TrendDirection, TrendMagnitude};
pub use values::{MetricValue, PercentageChange, StabilityThreshold};

use nutype::nutype;
#[allow(unused_imports)] // These are used by nutype derive macros
use serde::{Deserialize, Serialize};
use std::fmt;

/// F-score value (0.0 to 1.0)
///
/// Represents the harmonic mean of precision and recall, constrained to valid range.
#[nutype(
    validate(finite, greater_or_equal = 0.0, less_or_equal = 1.0),
    derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)
)]
pub struct FScore(f64);

impl Eq for FScore {} // Safe since validation ensures finite values

impl FScore {
    /// Perfect F-score (1.0)
    pub fn perfect() -> Self {
        Self::try_new(1.0).unwrap()
    }

    /// Zero F-score (0.0)
    pub fn zero() -> Self {
        Self::try_new(0.0).unwrap()
    }

    /// Calculate F-score from precision and recall using harmonic mean
    pub fn from_precision_recall(
        precision: Precision,
        recall: Recall,
    ) -> Result<Self, MetricsError> {
        let p = precision.into_inner();
        let r = recall.into_inner();

        if p + r == 0.0 {
            return Ok(Self::zero());
        }

        let f_score = constants::calculation::F1_MULTIPLIER * (p * r) / (p + r);
        Self::try_new(f_score).map_err(|_| MetricsError::InvalidValue(f_score))
    }

    /// Calculate F-beta score with custom beta parameter
    pub fn from_precision_recall_beta(
        precision: Precision,
        recall: Recall,
        beta: Beta,
    ) -> Result<Self, MetricsError> {
        let p = precision.into_inner();
        let r = recall.into_inner();

        if p + r == 0.0 {
            return Ok(Self::zero());
        }

        let f_score = Self::calculate_f_beta_formula(p, r, beta.into_inner());
        Self::try_new(f_score).map_err(|_| MetricsError::InvalidValue(f_score))
    }

    /// Calculate F-beta score using the mathematical formula
    /// F-beta = (1 + beta²) × (precision × recall) / ((beta² × precision) + recall)
    fn calculate_f_beta_formula(precision: f64, recall: f64, beta: f64) -> f64 {
        let beta_squared = beta * beta;
        let numerator = (1.0 + beta_squared) * (precision * recall);
        let denominator = (beta_squared * precision) + recall;
        numerator / denominator
    }
}

/// Precision value (0.0 to 1.0)
///
/// Represents the fraction of relevant instances among the retrieved instances.
#[nutype(
    validate(finite, greater_or_equal = 0.0, less_or_equal = 1.0),
    derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)
)]
pub struct Precision(f64);

impl Eq for Precision {} // Safe since validation ensures finite values

impl Precision {
    /// Perfect precision (1.0)
    pub fn perfect() -> Self {
        Self::try_new(1.0).unwrap()
    }

    /// Zero precision (0.0)
    pub fn zero() -> Self {
        Self::try_new(0.0).unwrap()
    }
}

/// Recall value (0.0 to 1.0)
///
/// Represents the fraction of relevant instances that were retrieved.
#[nutype(
    validate(finite, greater_or_equal = 0.0, less_or_equal = 1.0),
    derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)
)]
pub struct Recall(f64);

impl Eq for Recall {} // Safe since validation ensures finite values

impl Recall {
    /// Perfect recall (1.0)
    pub fn perfect() -> Self {
        Self::try_new(1.0).unwrap()
    }

    /// Zero recall (0.0)
    pub fn zero() -> Self {
        Self::try_new(0.0).unwrap()
    }
}

/// Beta parameter for F-beta scores
///
/// Beta > 1 emphasizes recall; Beta < 1 emphasizes precision.
/// Reasonable range: 0.1 to 10.0
#[nutype(
    validate(finite, greater = 0.0, less_or_equal = 10.0),
    derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)
)]
pub struct Beta(f64);

impl Beta {
    /// Standard F1-score (beta = 1.0)
    pub fn f1() -> Self {
        Self::try_new(1.0).unwrap()
    }

    /// F2-score emphasizing recall (beta = 2.0)
    pub fn f2() -> Self {
        Self::try_new(2.0).unwrap()
    }

    /// F0.5-score emphasizing precision (beta = 0.5)
    pub fn f05() -> Self {
        Self::try_new(0.5).unwrap()
    }
}

/// Confidence interval for statistical analysis
///
/// Represents statistical confidence level (e.g., 0.95 for 95% confidence).
#[nutype(
    validate(finite, greater = 0.0, less = 1.0),
    derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)
)]
pub struct ConfidenceLevel(f64);

impl ConfidenceLevel {
    /// 95% confidence level (most common)
    pub fn ninety_five_percent() -> Self {
        Self::try_new(0.95).unwrap()
    }

    /// 99% confidence level (high confidence)
    pub fn ninety_nine_percent() -> Self {
        Self::try_new(0.99).unwrap()
    }

    /// 90% confidence level (lower confidence)
    pub fn ninety_percent() -> Self {
        Self::try_new(0.90).unwrap()
    }
}

/// Errors that can occur during metrics calculations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MetricsError {
    /// Invalid F-score value (outside 0.0-1.0 range)
    InvalidValue(f64),
    /// Invalid precision value
    InvalidPrecision(String),
    /// Invalid recall value
    InvalidRecall(String),
    /// Invalid beta parameter
    InvalidBeta(String),
    /// Calculation error
    CalculationError(String),
}

impl fmt::Display for MetricsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MetricsError::InvalidValue(value) => {
                write!(
                    f,
                    "Invalid F-score value: {value} (must be between 0.0 and 1.0)"
                )
            }
            MetricsError::InvalidPrecision(msg) => write!(f, "Invalid precision: {msg}"),
            MetricsError::InvalidRecall(msg) => write!(f, "Invalid recall: {msg}"),
            MetricsError::InvalidBeta(msg) => write!(f, "Invalid beta parameter: {msg}"),
            MetricsError::CalculationError(msg) => write!(f, "F-score calculation error: {msg}"),
        }
    }
}

impl std::error::Error for MetricsError {}

/// Time-series data point for F-score tracking
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FScoreDataPoint {
    /// Timestamp for this measurement
    pub timestamp: Timestamp,
    /// F-score value at this time
    pub f_score: FScore,
    /// Optional precision value
    pub precision: Option<Precision>,
    /// Optional recall value
    pub recall: Option<Recall>,
    /// Number of samples this measurement is based on
    pub sample_count: SampleCount,
    /// Optional confidence level for statistical analysis
    pub confidence_level: Option<ConfidenceLevel>,
}

impl FScoreDataPoint {
    /// Create a new F-score data point
    pub fn new(timestamp: Timestamp, f_score: FScore, sample_count: SampleCount) -> Self {
        Self {
            timestamp,
            f_score,
            precision: None,
            recall: None,
            sample_count,
            confidence_level: None,
        }
    }

    /// Create a new F-score data point with precision and recall
    pub fn with_precision_recall(
        timestamp: Timestamp,
        precision: Precision,
        recall: Recall,
        sample_count: SampleCount,
    ) -> Result<Self, MetricsError> {
        let f_score = FScore::from_precision_recall(precision, recall)?;
        Ok(Self {
            timestamp,
            f_score,
            precision: Some(precision),
            recall: Some(recall),
            sample_count,
            confidence_level: None,
        })
    }

    /// Add confidence level to this data point
    pub fn with_confidence(mut self, confidence_level: ConfidenceLevel) -> Self {
        self.confidence_level = Some(confidence_level);
        self
    }

    /// Get performance assessment for this data point
    pub fn performance_assessment(&self) -> PerformanceAssessment {
        PerformanceAssessment::from_components(self.f_score, self.precision, self.recall)
    }

    /// Check if this data point is recent
    pub fn is_recent(&self) -> bool {
        self.timestamp.is_recent()
    }

    /// Get the age category of this data point
    pub fn age_category(&self) -> TimestampAge {
        self.timestamp.age_category()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_f_score_validation() {
        // Valid F-scores
        assert!(FScore::try_new(0.0).is_ok());
        assert!(FScore::try_new(0.5).is_ok());
        assert!(FScore::try_new(1.0).is_ok());

        // Invalid F-scores
        assert!(FScore::try_new(-0.1).is_err());
        assert!(FScore::try_new(1.1).is_err());
        assert!(FScore::try_new(f64::NAN).is_err());
        assert!(FScore::try_new(f64::INFINITY).is_err());
    }

    #[test]
    fn test_precision_recall_validation() {
        // Valid precision/recall
        assert!(Precision::try_new(0.8).is_ok());
        assert!(Recall::try_new(0.6).is_ok());

        // Invalid precision/recall
        assert!(Precision::try_new(-0.1).is_err());
        assert!(Recall::try_new(1.1).is_err());
    }

    #[test]
    fn test_f_score_calculation() {
        let precision = Precision::try_new(0.8).unwrap();
        let recall = Recall::try_new(0.6).unwrap();

        let f_score = FScore::from_precision_recall(precision, recall).unwrap();

        // F1 = 2 * (0.8 * 0.6) / (0.8 + 0.6) = 2 * 0.48 / 1.4 ≈ 0.6857
        assert!((f_score.into_inner() - 0.6857142857142857).abs() < 1e-10);
    }

    #[test]
    fn test_f_beta_calculation() {
        let precision = Precision::try_new(0.8).unwrap();
        let recall = Recall::try_new(0.6).unwrap();
        let beta = Beta::try_new(2.0).unwrap(); // F2 score

        let f_score = FScore::from_precision_recall_beta(precision, recall, beta).unwrap();

        // F2 = (1 + 4) * (0.8 * 0.6) / ((4 * 0.8) + 0.6) = 5 * 0.48 / 3.8 ≈ 0.6316
        assert!((f_score.into_inner() - 0.631_578_947_368_421).abs() < 1e-10);
    }

    #[test]
    fn test_edge_cases() {
        let zero_precision = Precision::zero();
        let zero_recall = Recall::zero();

        // Zero precision and recall should give zero F-score
        let f_score = FScore::from_precision_recall(zero_precision, zero_recall).unwrap();
        assert_eq!(f_score.into_inner(), 0.0);

        // Perfect precision and recall should give perfect F-score
        let perfect_precision = Precision::perfect();
        let perfect_recall = Recall::perfect();
        let f_score = FScore::from_precision_recall(perfect_precision, perfect_recall).unwrap();
        assert_eq!(f_score.into_inner(), 1.0);
    }

    #[test]
    fn test_beta_validation() {
        // Valid beta values
        assert!(Beta::try_new(0.1).is_ok());
        assert!(Beta::try_new(1.0).is_ok());
        assert!(Beta::try_new(2.0).is_ok());
        assert!(Beta::try_new(10.0).is_ok());

        // Invalid beta values
        assert!(Beta::try_new(0.0).is_err());
        assert!(Beta::try_new(-1.0).is_err());
        assert!(Beta::try_new(11.0).is_err());
    }

    #[test]
    fn test_confidence_level_validation() {
        // Valid confidence levels
        assert!(ConfidenceLevel::try_new(0.95).is_ok());
        assert!(ConfidenceLevel::try_new(0.99).is_ok());
        assert!(ConfidenceLevel::try_new(0.01).is_ok());

        // Invalid confidence levels
        assert!(ConfidenceLevel::try_new(0.0).is_err());
        assert!(ConfidenceLevel::try_new(1.0).is_err());
        assert!(ConfidenceLevel::try_new(1.1).is_err());
    }

    #[test]
    fn test_f_score_data_point() {
        let timestamp = Timestamp::now();
        let precision = Precision::try_new(0.8).unwrap();
        let recall = Recall::try_new(0.7).unwrap();
        let sample_count = SampleCount::try_new(100).unwrap();

        let data_point =
            FScoreDataPoint::with_precision_recall(timestamp, precision, recall, sample_count)
                .unwrap();

        assert_eq!(data_point.timestamp, timestamp);
        assert_eq!(data_point.precision, Some(precision));
        assert_eq!(data_point.recall, Some(recall));
        assert_eq!(data_point.sample_count, sample_count);

        // Verify F-score calculation
        let expected_f_score = constants::calculation::F1_MULTIPLIER * (0.8 * 0.7) / (0.8 + 0.7);
        assert!((data_point.f_score.into_inner() - expected_f_score).abs() < 1e-10);

        // Test new methods
        assert!(data_point.is_recent());
        let assessment = data_point.performance_assessment();
        assert_eq!(assessment.f_score_level, PerformanceLevel::Good);
    }

    #[test]
    fn test_convenience_constructors() {
        assert_eq!(FScore::perfect().into_inner(), 1.0);
        assert_eq!(FScore::zero().into_inner(), 0.0);
        assert_eq!(Precision::perfect().into_inner(), 1.0);
        assert_eq!(Recall::zero().into_inner(), 0.0);
        assert_eq!(Beta::f1().into_inner(), 1.0);
        assert_eq!(Beta::f2().into_inner(), 2.0);
        assert_eq!(ConfidenceLevel::ninety_five_percent().into_inner(), 0.95);
    }
}
