//! Count types for metrics dashboard statistics

use nutype::nutype;
use serde::{Deserialize, Serialize};

/// Number of models being tracked
#[nutype(
    validate(greater_or_equal = 0, less_or_equal = 1000),
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
pub struct ModelCount(usize);

impl ModelCount {
    /// No models tracked
    pub fn none() -> Self {
        Self::try_new(0).unwrap()
    }

    /// Typical small deployment
    pub fn small_deployment() -> Self {
        Self::try_new(3).unwrap()
    }

    /// Medium deployment
    pub fn medium_deployment() -> Self {
        Self::try_new(10).unwrap()
    }

    /// Large deployment
    pub fn large_deployment() -> Self {
        Self::try_new(50).unwrap()
    }
}

/// Number of applications being tracked
#[nutype(
    validate(greater_or_equal = 0, less_or_equal = 10000),
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
pub struct ApplicationCount(usize);

impl ApplicationCount {
    /// No applications tracked
    pub fn none() -> Self {
        Self::try_new(0).unwrap()
    }

    /// Small team (few applications)
    pub fn small_team() -> Self {
        Self::try_new(5).unwrap()
    }

    /// Medium organization
    pub fn medium_organization() -> Self {
        Self::try_new(25).unwrap()
    }

    /// Large enterprise
    pub fn large_enterprise() -> Self {
        Self::try_new(100).unwrap()
    }
}

/// Number of data points collected
#[nutype(
    validate(greater_or_equal = 0),
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
pub struct DataPointCount(usize);

impl DataPointCount {
    /// No data points
    pub fn none() -> Self {
        Self::try_new(0).unwrap()
    }

    /// Limited data
    pub fn limited() -> Self {
        Self::try_new(100).unwrap()
    }

    /// Moderate amount of data
    pub fn moderate() -> Self {
        Self::try_new(1000).unwrap()
    }

    /// Large dataset
    pub fn large() -> Self {
        Self::try_new(10000).unwrap()
    }

    /// Very large dataset
    pub fn very_large() -> Self {
        Self::try_new(100000).unwrap()
    }

    /// Check if we have sufficient data for analysis
    pub fn is_sufficient_for_analysis(&self) -> bool {
        self.into_inner() >= 30 // Statistical significance threshold
    }

    /// Get data quality level based on count
    pub fn quality_level(&self) -> DataQuality {
        match self.into_inner() {
            0 => DataQuality::NoData,
            1..=29 => DataQuality::Insufficient,
            30..=99 => DataQuality::Limited,
            100..=999 => DataQuality::Good,
            1000..=9999 => DataQuality::Excellent,
            _ => DataQuality::Exceptional,
        }
    }
}

/// Quality level of data based on sample size
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataQuality {
    /// No data available
    NoData,
    /// Insufficient for reliable analysis
    Insufficient,
    /// Limited but usable
    Limited,
    /// Good quality data
    Good,
    /// Excellent quality data
    Excellent,
    /// Exceptional quality data
    Exceptional,
}

impl DataQuality {
    /// Get confidence level description
    pub fn confidence_description(&self) -> &'static str {
        match self {
            Self::NoData => "No data available",
            Self::Insufficient => "Insufficient data for reliable analysis",
            Self::Limited => "Limited data - use with caution",
            Self::Good => "Good quality data - reliable for analysis",
            Self::Excellent => "Excellent quality data - high confidence",
            Self::Exceptional => "Exceptional quality data - very high confidence",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_count_validation() {
        assert!(ModelCount::try_new(0).is_ok());
        assert!(ModelCount::try_new(100).is_ok());
        assert!(ModelCount::try_new(1000).is_ok());
        assert!(ModelCount::try_new(1001).is_err());
    }

    #[test]
    fn test_application_count_validation() {
        assert!(ApplicationCount::try_new(0).is_ok());
        assert!(ApplicationCount::try_new(5000).is_ok());
        assert!(ApplicationCount::try_new(10000).is_ok());
        assert!(ApplicationCount::try_new(10001).is_err());
    }

    #[test]
    fn test_data_point_count_validation() {
        assert!(DataPointCount::try_new(0).is_ok());
        assert!(DataPointCount::try_new(usize::MAX).is_ok());
    }

    #[test]
    fn test_data_quality_levels() {
        assert_eq!(
            DataPointCount::try_new(0).unwrap().quality_level(),
            DataQuality::NoData
        );
        assert_eq!(
            DataPointCount::try_new(15).unwrap().quality_level(),
            DataQuality::Insufficient
        );
        assert_eq!(
            DataPointCount::try_new(50).unwrap().quality_level(),
            DataQuality::Limited
        );
        assert_eq!(
            DataPointCount::try_new(500).unwrap().quality_level(),
            DataQuality::Good
        );
    }

    #[test]
    fn test_sufficient_for_analysis() {
        assert!(!DataPointCount::try_new(10)
            .unwrap()
            .is_sufficient_for_analysis());
        assert!(DataPointCount::try_new(50)
            .unwrap()
            .is_sufficient_for_analysis());
    }
}
