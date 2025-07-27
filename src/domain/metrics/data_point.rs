//! Time-series data point for F-score tracking

use crate::domain::metrics::{
    core_metrics::{ConfidenceLevel, FScore, Precision, Recall},
    performance::PerformanceAssessment,
    sample_count::SampleCount,
    timestamp::{Timestamp, TimestampAge},
    MetricsError,
};
use serde::{Deserialize, Serialize};

/// Time-series data point for F-score tracking
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FScoreDataPoint {
    /// Timestamp for this measurement
    timestamp: Timestamp,
    /// F-score value at this time
    f_score: FScore,
    /// Optional precision value
    precision: Option<Precision>,
    /// Optional recall value
    recall: Option<Recall>,
    /// Number of samples this measurement is based on
    sample_count: SampleCount,
    /// Optional confidence level for statistical analysis
    confidence_level: Option<ConfidenceLevel>,
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

    /// Get the timestamp
    pub fn timestamp(&self) -> Timestamp {
        self.timestamp
    }

    /// Get the F-score value
    pub fn f_score(&self) -> FScore {
        self.f_score
    }

    /// Get the precision value if present
    pub fn precision(&self) -> Option<Precision> {
        self.precision
    }

    /// Get the recall value if present
    pub fn recall(&self) -> Option<Recall> {
        self.recall
    }

    /// Get the sample count
    pub fn sample_count(&self) -> SampleCount {
        self.sample_count
    }

    /// Get the confidence level if present
    pub fn confidence_level(&self) -> Option<ConfidenceLevel> {
        self.confidence_level
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::metrics::{constants, performance::PerformanceLevel};

    #[test]
    fn test_f_score_data_point() {
        let timestamp = Timestamp::now();
        let precision = Precision::try_new(0.8).unwrap();
        let recall = Recall::try_new(0.7).unwrap();
        let sample_count = SampleCount::try_new(100).unwrap();

        let data_point =
            FScoreDataPoint::with_precision_recall(timestamp, precision, recall, sample_count)
                .unwrap();

        assert_eq!(data_point.timestamp(), timestamp);
        assert_eq!(data_point.precision(), Some(precision));
        assert_eq!(data_point.recall(), Some(recall));
        assert_eq!(data_point.sample_count(), sample_count);

        // Verify F-score calculation
        let expected_f_score = constants::calculation::F1_MULTIPLIER * (0.8 * 0.7) / (0.8 + 0.7);
        assert!((data_point.f_score().into_inner() - expected_f_score).abs() < 1e-10);

        // Test new methods
        assert!(data_point.is_recent());
        let assessment = data_point.performance_assessment();
        assert_eq!(assessment.f_score_level(), PerformanceLevel::Good);
    }

    #[test]
    fn test_data_point_with_confidence() {
        let timestamp = Timestamp::now();
        let f_score = FScore::try_new(0.85).unwrap();
        let sample_count = SampleCount::try_new(500).unwrap();
        let confidence = ConfidenceLevel::ninety_five_percent();

        let data_point =
            FScoreDataPoint::new(timestamp, f_score, sample_count).with_confidence(confidence);

        assert_eq!(data_point.confidence_level(), Some(confidence));
    }
}
