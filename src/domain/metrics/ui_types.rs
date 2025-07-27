//! UI-specific domain types for metrics display
//!
//! Provides type-safe representations for UI elements like colors and recommendations.

use nutype::nutype;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Hex color code for UI display
///
/// Validates that the color is a valid hex color code (e.g., "#FF0000")
#[nutype(
    validate(regex = r"^#[0-9A-Fa-f]{6}$"),
    derive(
        Debug,
        Clone,
        PartialEq,
        Eq,
        Hash,
        Serialize,
        Deserialize,
        AsRef,
        Display
    )
)]
pub struct ColorCode(String);

impl ColorCode {
    /// Green - Exceptional performance
    pub fn green() -> Self {
        Self::try_new("#00C851".to_string()).unwrap()
    }

    /// Dark Green - Excellent performance
    pub fn dark_green() -> Self {
        Self::try_new("#007E33".to_string()).unwrap()
    }

    /// Light Blue - Good performance
    pub fn light_blue() -> Self {
        Self::try_new("#39C0ED".to_string()).unwrap()
    }

    /// Orange - Acceptable performance
    pub fn orange() -> Self {
        Self::try_new("#FFBB33".to_string()).unwrap()
    }

    /// Dark Orange - Poor performance
    pub fn dark_orange() -> Self {
        Self::try_new("#FF6900".to_string()).unwrap()
    }

    /// Red - Critical performance
    pub fn red() -> Self {
        Self::try_new("#FF1744".to_string()).unwrap()
    }
}

/// Recommendation for performance improvement
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Recommendation {
    /// Model is performing exceptionally well
    Outstanding {
        /// Specific advice for maintaining performance
        advice: RecommendationAdvice,
    },
    /// Model needs immediate attention
    Critical {
        /// Specific remediation steps
        remediation: RemediationSteps,
    },
    /// Model needs improvement
    NeedsImprovement {
        /// Quality-specific recommendation
        quality_advice: QualityAdvice,
        /// Performance level context
        level_context: PerformanceLevelContext,
    },
    /// Model is performing adequately
    Standard {
        /// General recommendation text
        advice: RecommendationAdvice,
    },
}

impl Recommendation {
    /// Create outstanding performance recommendation
    pub fn outstanding() -> Self {
        Self::Outstanding {
            advice: RecommendationAdvice::DocumentAndReplicate,
        }
    }

    /// Create critical performance recommendation
    pub fn critical() -> Self {
        Self::Critical {
            remediation: RemediationSteps::ImmediateInvestigation,
        }
    }

    /// Get human-readable recommendation text
    pub fn as_text(&self) -> &'static str {
        match self {
            Self::Outstanding { advice } => advice.as_text(),
            Self::Critical { remediation } => remediation.as_text(),
            Self::NeedsImprovement {
                quality_advice,
                level_context: _,
            } => quality_advice.as_text(),
            Self::Standard { advice } => advice.as_text(),
        }
    }
}

impl fmt::Display for Recommendation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_text())
    }
}

/// Specific recommendation advice
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecommendationAdvice {
    DocumentAndReplicate,
    MaintainCurrentApproach,
    OptimizeForSpecificNeeds,
}

impl RecommendationAdvice {
    pub fn as_text(&self) -> &'static str {
        match self {
            Self::DocumentAndReplicate => {
                "Outstanding performance - document and replicate approach"
            }
            Self::MaintainCurrentApproach => "Excellent balance - maintain current approach",
            Self::OptimizeForSpecificNeeds => "Good foundation - optimize for specific needs",
        }
    }
}

/// Remediation steps for critical performance
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RemediationSteps {
    ImmediateInvestigation,
    AddressWeakerMetric,
    SignificantImprovement,
}

impl RemediationSteps {
    pub fn as_text(&self) -> &'static str {
        match self {
            Self::ImmediateInvestigation => {
                "Critical performance requires immediate investigation and remediation"
            }
            Self::AddressWeakerMetric => "Address the weaker metric for better balance",
            Self::SignificantImprovement => "Both metrics need significant improvement",
        }
    }
}

/// Quality-specific advice
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QualityAdvice {
    ImproveRecall,
    ImprovePrecision,
    ImproveBalance,
}

impl QualityAdvice {
    pub fn as_text(&self) -> &'static str {
        match self {
            Self::ImproveRecall => "Consider techniques to improve recall",
            Self::ImprovePrecision => "Consider techniques to improve precision",
            Self::ImproveBalance => "Performance needs improvement. Focus on balance",
        }
    }
}

/// Performance level context for recommendations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PerformanceLevelContext {
    RequiresAttention,
    MonitorClosely,
    ApproachingAcceptable,
}

/// Performance category labels for demo data display
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PerformanceCategory {
    /// High quality performance (90%+ F-score)
    Excellent,
    /// Good performance (75-90% F-score)
    Good,
    /// Needs improvement (50-75% F-score)
    NeedsImprovement,
    /// Critical issues (<50% F-score)
    Critical,
}

impl PerformanceCategory {
    /// Get the display label for this category
    pub fn display_label(&self) -> &'static str {
        match self {
            Self::Excellent => "Excellent Performance",
            Self::Good => "Good Performance",
            Self::NeedsImprovement => "Needs Improvement",
            Self::Critical => "Critical Issues",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_code_validation() {
        // Valid color codes
        assert!(ColorCode::try_new("#FF0000".to_string()).is_ok());
        assert!(ColorCode::try_new("#00FF00".to_string()).is_ok());
        assert!(ColorCode::try_new("#0000FF".to_string()).is_ok());
        assert!(ColorCode::try_new("#ABCDEF".to_string()).is_ok());

        // Invalid color codes
        assert!(ColorCode::try_new("FF0000".to_string()).is_err()); // Missing #
        assert!(ColorCode::try_new("#FF00".to_string()).is_err()); // Too short
        assert!(ColorCode::try_new("#FF00000".to_string()).is_err()); // Too long
        assert!(ColorCode::try_new("#GGGGGG".to_string()).is_err()); // Invalid chars
        assert!(ColorCode::try_new("#ffbb33".to_string()).is_ok()); // Lowercase is valid
    }

    #[test]
    fn test_color_code_constants() {
        assert_eq!(ColorCode::green().as_ref(), "#00C851");
        assert_eq!(ColorCode::red().as_ref(), "#FF1744");
        assert_eq!(ColorCode::orange().as_ref(), "#FFBB33");
    }

    #[test]
    fn test_recommendation_creation() {
        let outstanding = Recommendation::outstanding();
        assert!(matches!(outstanding, Recommendation::Outstanding { .. }));
        assert_eq!(
            outstanding.as_text(),
            "Outstanding performance - document and replicate approach"
        );

        let critical = Recommendation::critical();
        assert!(matches!(critical, Recommendation::Critical { .. }));
        assert_eq!(
            critical.as_text(),
            "Critical performance requires immediate investigation and remediation"
        );
    }
}
