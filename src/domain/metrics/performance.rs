//! Performance categorization types for F-score metrics
//!
//! Provides domain types for categorizing and rating the quality of
//! machine learning model performance based on F-scores and other metrics.

use crate::domain::metrics::{FScore, Precision, Recall};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Performance level categorization for F-scores
///
/// Categorizes F-score values into meaningful business performance levels
/// based on industry standards and practical thresholds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Hash)]
pub enum PerformanceLevel {
    /// F-score >= 0.9 (Exceptional performance)
    Exceptional,
    /// F-score >= 0.8 (Excellent performance)
    Excellent,
    /// F-score >= 0.7 (Good performance)
    Good,
    /// F-score >= 0.6 (Acceptable performance)
    Acceptable,
    /// F-score >= 0.5 (Poor performance)
    Poor,
    /// F-score < 0.5 (Critical - needs immediate attention)
    Critical,
}

impl PerformanceLevel {
    /// Categorize an F-score into a performance level
    pub fn from_f_score(f_score: FScore) -> Self {
        let value = f_score.into_inner();
        match value {
            v if v >= 0.9 => Self::Exceptional,
            v if v >= 0.8 => Self::Excellent,
            v if v >= 0.7 => Self::Good,
            v if v >= 0.6 => Self::Acceptable,
            v if v >= 0.5 => Self::Poor,
            _ => Self::Critical,
        }
    }

    /// Get the minimum F-score for this performance level
    pub fn min_f_score(&self) -> FScore {
        let value = match self {
            Self::Exceptional => 0.9,
            Self::Excellent => 0.8,
            Self::Good => 0.7,
            Self::Acceptable => 0.6,
            Self::Poor => 0.5,
            Self::Critical => 0.0,
        };
        FScore::try_new(value).unwrap()
    }

    /// Check if this performance level requires immediate attention
    pub fn requires_attention(&self) -> bool {
        matches!(self, Self::Poor | Self::Critical)
    }

    /// Check if this performance level is production-ready
    pub fn is_production_ready(&self) -> bool {
        matches!(self, Self::Exceptional | Self::Excellent | Self::Good)
    }

    /// Get a human-readable description
    pub fn description(&self) -> &'static str {
        match self {
            Self::Exceptional => "Exceptional performance - industry leading",
            Self::Excellent => "Excellent performance - production ready",
            Self::Good => "Good performance - meets expectations",
            Self::Acceptable => "Acceptable performance - monitor closely",
            Self::Poor => "Poor performance - needs improvement",
            Self::Critical => "Critical performance - immediate action required",
        }
    }

    /// Get color code for UI display
    pub fn color_code(&self) -> &'static str {
        match self {
            Self::Exceptional => "#00C851", // Green
            Self::Excellent => "#007E33",   // Dark Green
            Self::Good => "#39C0ED",        // Light Blue
            Self::Acceptable => "#ffbb33",  // Orange
            Self::Poor => "#FF6900",        // Dark Orange
            Self::Critical => "#FF1744",    // Red
        }
    }
}

impl fmt::Display for PerformanceLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Exceptional => "Exceptional",
            Self::Excellent => "Excellent",
            Self::Good => "Good",
            Self::Acceptable => "Acceptable",
            Self::Poor => "Poor",
            Self::Critical => "Critical",
        };
        write!(f, "{name}")
    }
}

/// Quality rating based on precision and recall balance
///
/// Provides a more nuanced quality assessment by considering both
/// precision and recall values and their balance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QualityRating {
    /// Both precision and recall are high (>= 0.8)
    Balanced,
    /// High precision (>= 0.8), moderate recall (>= 0.6)
    PrecisionFocused,
    /// High recall (>= 0.8), moderate precision (>= 0.6)
    RecallFocused,
    /// Both precision and recall are moderate (>= 0.6)
    Moderate,
    /// One metric is good, the other is poor
    Imbalanced,
    /// Both metrics are poor (< 0.6)
    Inadequate,
}

impl QualityRating {
    /// Calculate quality rating from precision and recall
    pub fn from_precision_recall(precision: Precision, recall: Recall) -> Self {
        let p = precision.into_inner();
        let r = recall.into_inner();

        match (p, r) {
            (p, r) if p >= 0.8 && r >= 0.8 => Self::Balanced,
            (p, r) if p >= 0.8 && r >= 0.6 => Self::PrecisionFocused,
            (p, r) if r >= 0.8 && p >= 0.6 => Self::RecallFocused,
            (p, r) if p >= 0.6 && r >= 0.6 => Self::Moderate,
            (p, r) if (p >= 0.7 && r < 0.5) || (r >= 0.7 && p < 0.5) => Self::Imbalanced,
            _ => Self::Inadequate,
        }
    }

    /// Get recommendation for this quality rating
    pub fn recommendation(&self) -> &'static str {
        match self {
            Self::Balanced => "Excellent balance - maintain current approach",
            Self::PrecisionFocused => "Consider techniques to improve recall",
            Self::RecallFocused => "Consider techniques to improve precision",
            Self::Moderate => "Good foundation - optimize for specific needs",
            Self::Imbalanced => "Address the weaker metric for better balance",
            Self::Inadequate => "Both metrics need significant improvement",
        }
    }

    /// Check if this rating indicates a problematic model
    pub fn is_problematic(&self) -> bool {
        matches!(self, Self::Imbalanced | Self::Inadequate)
    }
}

impl fmt::Display for QualityRating {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Balanced => "Balanced",
            Self::PrecisionFocused => "Precision-Focused",
            Self::RecallFocused => "Recall-Focused",
            Self::Moderate => "Moderate",
            Self::Imbalanced => "Imbalanced",
            Self::Inadequate => "Inadequate",
        };
        write!(f, "{name}")
    }
}

/// Combined performance assessment
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PerformanceAssessment {
    pub f_score_level: PerformanceLevel,
    pub quality_rating: QualityRating,
    pub recommendation: String,
}

impl PerformanceAssessment {
    /// Create comprehensive assessment from F-score components
    pub fn from_components(
        f_score: FScore,
        precision: Option<Precision>,
        recall: Option<Recall>,
    ) -> Self {
        let f_score_level = PerformanceLevel::from_f_score(f_score);

        let quality_rating = match (precision, recall) {
            (Some(p), Some(r)) => QualityRating::from_precision_recall(p, r),
            _ => {
                // If we only have F-score, infer quality based on level
                match f_score_level {
                    PerformanceLevel::Exceptional | PerformanceLevel::Excellent => {
                        QualityRating::Balanced
                    }
                    PerformanceLevel::Good => QualityRating::Moderate,
                    PerformanceLevel::Acceptable => QualityRating::Imbalanced,
                    _ => QualityRating::Inadequate,
                }
            }
        };

        let recommendation = match (&f_score_level, &quality_rating) {
            (PerformanceLevel::Exceptional, QualityRating::Balanced) => {
                "Outstanding performance - document and replicate approach".to_string()
            }
            (PerformanceLevel::Critical, _) => {
                "Critical performance requires immediate investigation and remediation".to_string()
            }
            (level, rating) if level.requires_attention() => {
                format!("Performance needs improvement. {}", rating.recommendation())
            }
            (_, rating) => rating.recommendation().to_string(),
        };

        Self {
            f_score_level,
            quality_rating,
            recommendation,
        }
    }

    /// Check if this assessment indicates urgent action is needed
    pub fn needs_urgent_action(&self) -> bool {
        self.f_score_level.requires_attention() || self.quality_rating.is_problematic()
    }

    /// Get overall confidence in this model's performance
    pub fn confidence_level(&self) -> ConfidenceInPerformance {
        match (&self.f_score_level, &self.quality_rating) {
            (PerformanceLevel::Exceptional, QualityRating::Balanced) => {
                ConfidenceInPerformance::VeryHigh
            }
            (PerformanceLevel::Excellent, QualityRating::Balanced) => ConfidenceInPerformance::High,
            (level, rating) if level.is_production_ready() && !rating.is_problematic() => {
                ConfidenceInPerformance::Moderate
            }
            (level, _) if level.requires_attention() => ConfidenceInPerformance::Low,
            _ => ConfidenceInPerformance::VeryLow,
        }
    }
}

/// Confidence level in model performance
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfidenceInPerformance {
    VeryHigh,
    High,
    Moderate,
    Low,
    VeryLow,
}

impl fmt::Display for ConfidenceInPerformance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::VeryHigh => "Very High",
            Self::High => "High",
            Self::Moderate => "Moderate",
            Self::Low => "Low",
            Self::VeryLow => "Very Low",
        };
        write!(f, "{name}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_performance_level_categorization() {
        assert_eq!(
            PerformanceLevel::from_f_score(FScore::try_new(0.95).unwrap()),
            PerformanceLevel::Exceptional
        );
        assert_eq!(
            PerformanceLevel::from_f_score(FScore::try_new(0.85).unwrap()),
            PerformanceLevel::Excellent
        );
        assert_eq!(
            PerformanceLevel::from_f_score(FScore::try_new(0.45).unwrap()),
            PerformanceLevel::Critical
        );
    }

    #[test]
    fn test_quality_rating_from_precision_recall() {
        let high_p = Precision::try_new(0.9).unwrap();
        let high_r = Recall::try_new(0.85).unwrap();
        assert_eq!(
            QualityRating::from_precision_recall(high_p, high_r),
            QualityRating::Balanced
        );

        let high_p = Precision::try_new(0.9).unwrap();
        let med_r = Recall::try_new(0.65).unwrap();
        assert_eq!(
            QualityRating::from_precision_recall(high_p, med_r),
            QualityRating::PrecisionFocused
        );
    }

    #[test]
    fn test_performance_assessment() {
        let f_score = FScore::try_new(0.85).unwrap();
        let precision = Some(Precision::try_new(0.9).unwrap());
        let recall = Some(Recall::try_new(0.8).unwrap());

        let assessment = PerformanceAssessment::from_components(f_score, precision, recall);

        assert_eq!(assessment.f_score_level, PerformanceLevel::Excellent);
        assert_eq!(assessment.quality_rating, QualityRating::Balanced);
        assert!(!assessment.needs_urgent_action());
    }

    #[test]
    fn test_production_readiness() {
        assert!(PerformanceLevel::Excellent.is_production_ready());
        assert!(!PerformanceLevel::Poor.is_production_ready());
        assert!(PerformanceLevel::Critical.requires_attention());
    }
}
