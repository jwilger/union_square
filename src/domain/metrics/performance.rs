//! Performance categorization types for F-score metrics
//!
//! Provides domain types for categorizing and rating the quality of
//! machine learning model performance based on F-scores and other metrics.

use crate::domain::metrics::{
    constants,
    ui_types::{
        ColorCode, PerformanceLevelContext, QualityAdvice, Recommendation, RecommendationAdvice,
        RemediationSteps,
    },
    FScore, Precision, Recall,
};
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
            v if v >= constants::performance_thresholds::EXCELLENT_THRESHOLD => Self::Exceptional,
            v if v >= constants::performance_thresholds::GOOD_THRESHOLD => Self::Excellent,
            v if v >= constants::performance_thresholds::ACCEPTABLE_THRESHOLD => Self::Good,
            v if v >= constants::performance_thresholds::NEEDS_IMPROVEMENT_THRESHOLD => {
                Self::Acceptable
            }
            v if v >= constants::performance_thresholds::CRITICAL_THRESHOLD => Self::Poor,
            _ => Self::Critical,
        }
    }

    /// Get the minimum F-score for this performance level
    pub fn min_f_score(&self) -> FScore {
        let value = match self {
            Self::Exceptional => constants::performance_thresholds::EXCELLENT_THRESHOLD,
            Self::Excellent => constants::performance_thresholds::GOOD_THRESHOLD,
            Self::Good => constants::performance_thresholds::ACCEPTABLE_THRESHOLD,
            Self::Acceptable => constants::performance_thresholds::NEEDS_IMPROVEMENT_THRESHOLD,
            Self::Poor => constants::performance_thresholds::CRITICAL_THRESHOLD,
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
    pub fn color_code(&self) -> ColorCode {
        match self {
            Self::Exceptional => ColorCode::green(),
            Self::Excellent => ColorCode::dark_green(),
            Self::Good => ColorCode::light_blue(),
            Self::Acceptable => ColorCode::orange(),
            Self::Poor => ColorCode::dark_orange(),
            Self::Critical => ColorCode::red(),
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
            (p, r)
                if p >= constants::quality_thresholds::HIGH_THRESHOLD
                    && r >= constants::quality_thresholds::HIGH_THRESHOLD =>
            {
                Self::Balanced
            }
            (p, r)
                if p >= constants::quality_thresholds::HIGH_THRESHOLD
                    && r >= constants::quality_thresholds::MODERATE_THRESHOLD =>
            {
                Self::PrecisionFocused
            }
            (p, r)
                if r >= constants::quality_thresholds::HIGH_THRESHOLD
                    && p >= constants::quality_thresholds::MODERATE_THRESHOLD =>
            {
                Self::RecallFocused
            }
            (p, r)
                if p >= constants::quality_thresholds::MODERATE_THRESHOLD
                    && r >= constants::quality_thresholds::MODERATE_THRESHOLD =>
            {
                Self::Moderate
            }
            (p, r)
                if (p >= constants::quality_thresholds::GOOD_THRESHOLD
                    && r < constants::quality_thresholds::POOR_THRESHOLD)
                    || (r >= constants::quality_thresholds::GOOD_THRESHOLD
                        && p < constants::quality_thresholds::POOR_THRESHOLD) =>
            {
                Self::Imbalanced
            }
            _ => Self::Inadequate,
        }
    }

    /// Get recommendation for this quality rating
    pub fn recommendation(&self) -> Recommendation {
        match self {
            Self::Balanced => Recommendation::Standard {
                advice: RecommendationAdvice::MaintainCurrentApproach,
            },
            Self::PrecisionFocused => Recommendation::NeedsImprovement {
                quality_advice: QualityAdvice::ImproveRecall,
                level_context: PerformanceLevelContext::MonitorClosely,
            },
            Self::RecallFocused => Recommendation::NeedsImprovement {
                quality_advice: QualityAdvice::ImprovePrecision,
                level_context: PerformanceLevelContext::MonitorClosely,
            },
            Self::Moderate => Recommendation::Standard {
                advice: RecommendationAdvice::OptimizeForSpecificNeeds,
            },
            Self::Imbalanced => Recommendation::NeedsImprovement {
                quality_advice: QualityAdvice::ImproveBalance,
                level_context: PerformanceLevelContext::RequiresAttention,
            },
            Self::Inadequate => Recommendation::Critical {
                remediation: RemediationSteps::SignificantImprovement,
            },
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
    f_score_level: PerformanceLevel,
    quality_rating: QualityRating,
    recommendation: Recommendation,
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
                Recommendation::outstanding()
            }
            (PerformanceLevel::Critical, _) => Recommendation::critical(),
            (level, _) if level.requires_attention() => quality_rating.recommendation(),
            _ => quality_rating.recommendation(),
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

    /// Get the F-score performance level
    pub fn f_score_level(&self) -> PerformanceLevel {
        self.f_score_level
    }

    /// Get the quality rating
    pub fn quality_rating(&self) -> QualityRating {
        self.quality_rating
    }

    /// Get the recommendation
    pub fn recommendation(&self) -> &Recommendation {
        &self.recommendation
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

        assert_eq!(assessment.f_score_level(), PerformanceLevel::Excellent);
        assert_eq!(assessment.quality_rating(), QualityRating::Balanced);
        assert!(!assessment.needs_urgent_action());
    }

    #[test]
    fn test_production_readiness() {
        assert!(PerformanceLevel::Excellent.is_production_ready());
        assert!(!PerformanceLevel::Poor.is_production_ready());
        assert!(PerformanceLevel::Critical.requires_attention());
    }
}
