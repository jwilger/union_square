//! Trend analysis types for F-score metrics

use crate::domain::metrics::{
    constants,
    values::{MetricValue, StabilityThreshold},
};
use nutype::nutype;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Direction of a metric trend
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum TrendDirection {
    /// Metric is improving
    Improving,
    /// Metric is declining
    Declining,
    /// Metric is stable (within threshold)
    Stable,
}

impl TrendDirection {
    /// Determine trend direction from change value and threshold
    #[deprecated(note = "Use from_values with MetricValue types instead")]
    pub fn from_change(change: f64, stability_threshold: StabilityThreshold) -> Self {
        if change.abs() <= stability_threshold.into_inner() {
            Self::Stable
        } else if change > 0.0 {
            Self::Improving
        } else {
            Self::Declining
        }
    }

    /// Determine trend direction from two metric values
    pub fn from_values(
        current: MetricValue,
        previous: MetricValue,
        stability_threshold: StabilityThreshold,
    ) -> Self {
        // Check if we can calculate percentage change
        if let Some(percentage_change) = current.percentage_change_from(previous) {
            // Use percentage change to determine if change is significant
            if percentage_change.into_inner().abs() <= stability_threshold.into_inner() {
                Self::Stable
            } else if percentage_change.is_improvement() {
                Self::Improving
            } else {
                Self::Declining
            }
        } else {
            // If previous is zero, use absolute change
            let change = current.into_inner() - previous.into_inner();
            if change.abs() <= stability_threshold.into_inner() {
                Self::Stable
            } else if change > 0.0 {
                Self::Improving
            } else {
                Self::Declining
            }
        }
    }
}

impl fmt::Display for TrendDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Improving => write!(f, "improving"),
            Self::Declining => write!(f, "declining"),
            Self::Stable => write!(f, "stable"),
        }
    }
}

/// Magnitude of change in a metric (absolute value, 0.0-1.0)
#[nutype(
    validate(finite, greater_or_equal = 0.0, less_or_equal = 1.0),
    derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)
)]
pub struct TrendMagnitude(f64);

impl TrendMagnitude {
    /// Negligible change
    pub fn negligible() -> Self {
        Self::try_new(0.01).unwrap() // 1%
    }

    /// Small change
    pub fn small() -> Self {
        Self::try_new(0.05).unwrap() // 5%
    }

    /// Moderate change
    pub fn moderate() -> Self {
        Self::try_new(0.1).unwrap() // 10%
    }

    /// Large change
    pub fn large() -> Self {
        Self::try_new(0.2).unwrap() // 20%
    }

    /// Calculate magnitude from two metric values
    pub fn from_values(current: MetricValue, previous: MetricValue) -> Result<Self, TrendError> {
        if previous.into_inner() == 0.0 {
            return Err(TrendError::ZeroDivision);
        }
        let magnitude =
            ((current.into_inner() - previous.into_inner()) / previous.into_inner()).abs();
        Self::try_new(magnitude).map_err(|_| TrendError::InvalidMagnitude(magnitude))
    }

    /// Calculate magnitude from raw values (for backward compatibility)
    #[deprecated(note = "Use from_values with MetricValue types instead")]
    pub fn from_raw_values(current: f64, previous: f64) -> Result<Self, TrendError> {
        if previous == 0.0 {
            return Err(TrendError::ZeroDivision);
        }
        let magnitude = ((current - previous) / previous).abs();
        Self::try_new(magnitude).map_err(|_| TrendError::InvalidMagnitude(magnitude))
    }

    /// Categorize the magnitude of change
    pub fn category(&self) -> TrendCategory {
        match self.into_inner() {
            x if x <= constants::statistical::DEFAULT_STABILITY_THRESHOLD => {
                TrendCategory::Negligible
            }
            x if x <= 0.05 => TrendCategory::Small,
            x if x <= 0.15 => TrendCategory::Moderate,
            _ => TrendCategory::Large,
        }
    }
}

/// Category of trend magnitude
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrendCategory {
    Negligible,
    Small,
    Moderate,
    Large,
}

/// Complete trend analysis combining direction and magnitude
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrendAnalysis {
    pub direction: TrendDirection,
    pub magnitude: TrendMagnitude,
    pub category: TrendCategory,
}

impl TrendAnalysis {
    /// Create trend analysis from current and previous metric values
    pub fn from_values(
        current: MetricValue,
        previous: MetricValue,
        stability_threshold: StabilityThreshold,
    ) -> Result<Self, TrendError> {
        let direction = TrendDirection::from_values(current, previous, stability_threshold);
        let magnitude = TrendMagnitude::from_values(current, previous)?;
        let category = magnitude.category();

        Ok(Self {
            direction,
            magnitude,
            category,
        })
    }

    /// Create trend analysis from raw f64 values (for backward compatibility)
    #[deprecated(note = "Use from_values with MetricValue types instead")]
    pub fn from_raw_values(
        current: f64,
        previous: f64,
        stability_threshold: f64,
    ) -> Result<Self, TrendError> {
        let current_metric =
            MetricValue::try_new(current).map_err(|_| TrendError::InvalidMagnitude(current))?;
        let previous_metric =
            MetricValue::try_new(previous).map_err(|_| TrendError::InvalidMagnitude(previous))?;
        let threshold = StabilityThreshold::try_new(stability_threshold)
            .map_err(|_| TrendError::InvalidThreshold(stability_threshold))?;

        Self::from_values(current_metric, previous_metric, threshold)
    }

    /// Create a new trend analysis from direction and magnitude
    pub fn new(direction: TrendDirection, magnitude: TrendMagnitude) -> Self {
        let category = magnitude.category();
        Self {
            direction,
            magnitude,
            category,
        }
    }

    /// Check if trend is significant (not negligible)
    pub fn is_significant(&self) -> bool {
        !matches!(self.category, TrendCategory::Negligible)
    }
}

/// Errors that can occur during trend analysis
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TrendError {
    /// Division by zero when calculating percentage change
    ZeroDivision,
    /// Invalid magnitude value
    InvalidMagnitude(f64),
    /// Invalid stability threshold value
    InvalidThreshold(f64),
}

impl fmt::Display for TrendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroDivision => write!(f, "Cannot calculate trend: previous value is zero"),
            Self::InvalidMagnitude(value) => write!(f, "Invalid trend magnitude: {value}"),
            Self::InvalidThreshold(value) => write!(f, "Invalid stability threshold: {value}"),
        }
    }
}

impl std::error::Error for TrendError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trend_direction_from_values() {
        let threshold = StabilityThreshold::try_new(0.01).unwrap();

        // Test improving trend
        assert_eq!(
            TrendDirection::from_values(
                MetricValue::try_new(0.85).unwrap(),
                MetricValue::try_new(0.80).unwrap(),
                threshold
            ),
            TrendDirection::Improving
        );

        // Test declining trend
        assert_eq!(
            TrendDirection::from_values(
                MetricValue::try_new(0.75).unwrap(),
                MetricValue::try_new(0.80).unwrap(),
                threshold
            ),
            TrendDirection::Declining
        );

        // Test stable trend
        assert_eq!(
            TrendDirection::from_values(
                MetricValue::try_new(0.805).unwrap(),
                MetricValue::try_new(0.800).unwrap(),
                threshold
            ),
            TrendDirection::Stable
        );
    }

    #[test]
    fn test_trend_magnitude_validation() {
        assert!(TrendMagnitude::try_new(0.0).is_ok());
        assert!(TrendMagnitude::try_new(0.5).is_ok());
        assert!(TrendMagnitude::try_new(1.0).is_ok());
        assert!(TrendMagnitude::try_new(-0.1).is_err());
        assert!(TrendMagnitude::try_new(1.1).is_err());
    }

    #[test]
    fn test_trend_magnitude_from_values() {
        let magnitude = TrendMagnitude::from_values(
            MetricValue::try_new(1.0).unwrap(),
            MetricValue::try_new(0.9).unwrap(),
        )
        .unwrap();
        // ((1.0 - 0.9) / 0.9).abs() = (0.1 / 0.9) ≈ 0.111
        assert!((magnitude.into_inner() - (0.1 / 0.9)).abs() < 1e-10);

        let magnitude = TrendMagnitude::from_values(
            MetricValue::try_new(0.9).unwrap(),
            MetricValue::try_new(1.0).unwrap(),
        )
        .unwrap();
        assert!((magnitude.into_inner() - 0.1).abs() < 1e-10);

        assert!(TrendMagnitude::from_values(
            MetricValue::try_new(1.0).unwrap(),
            MetricValue::try_new(0.0).unwrap()
        )
        .is_err());
    }

    #[test]
    fn test_trend_analysis() {
        let analysis = TrendAnalysis::from_values(
            MetricValue::try_new(1.0).unwrap(),
            MetricValue::try_new(0.9).unwrap(),
            StabilityThreshold::try_new(0.01).unwrap(),
        )
        .unwrap();
        assert_eq!(analysis.direction, TrendDirection::Improving);
        assert_eq!(analysis.category, TrendCategory::Moderate);
        assert!(analysis.is_significant());
    }
}
