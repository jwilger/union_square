//! Time duration domain types for metrics
//!
//! Provides type-safe representations for time durations used throughout
//! the metrics system, replacing magic numbers with semantic types.

use chrono::Duration;
use nutype::nutype;

/// Hours as a domain type
///
/// Represents a number of hours for time calculations.
/// Range: 1 to 8760 (1 year in hours)
#[nutype(
    validate(greater = 0, less_or_equal = 8760),
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
pub struct Hours(i64);

impl Hours {
    /// One hour
    pub fn one() -> Self {
        Self::try_new(1).unwrap()
    }

    /// One day (24 hours)
    pub fn one_day() -> Self {
        Self::try_new(24).unwrap()
    }

    /// One week (168 hours)
    pub fn one_week() -> Self {
        Self::try_new(168).unwrap()
    }

    /// Convert to chrono Duration
    pub fn to_duration(&self) -> Duration {
        Duration::hours(self.into_inner())
    }
}

/// Days as a domain type
///
/// Represents a number of days for time calculations.
/// Range: 1 to 3650 (10 years)
#[nutype(
    validate(greater = 0, less_or_equal = 3650),
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
pub struct Days(i64);

impl Days {
    /// One day
    pub fn one() -> Self {
        Self::try_new(1).unwrap()
    }

    /// One week (7 days)
    pub fn one_week() -> Self {
        Self::try_new(7).unwrap()
    }

    /// One month (30 days)
    pub fn one_month() -> Self {
        Self::try_new(30).unwrap()
    }

    /// One quarter (90 days)
    pub fn one_quarter() -> Self {
        Self::try_new(90).unwrap()
    }

    /// One year (365 days)
    pub fn one_year() -> Self {
        Self::try_new(365).unwrap()
    }

    /// Convert to chrono Duration
    pub fn to_duration(&self) -> Duration {
        Duration::days(self.into_inner())
    }

    /// Convert to hours
    pub fn to_hours(&self) -> i64 {
        self.into_inner() * 24
    }
}

/// Minutes as a domain type for fine-grained time measurements
///
/// Range: 1 to 10080 (1 week in minutes)
#[nutype(
    validate(greater = 0, less_or_equal = 10080),
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
pub struct Minutes(i64);

impl Minutes {
    /// One minute
    pub fn one() -> Self {
        Self::try_new(1).unwrap()
    }

    /// Five minutes (common tolerance for clock skew)
    pub fn five() -> Self {
        Self::try_new(5).unwrap()
    }

    /// One hour (60 minutes)
    pub fn one_hour() -> Self {
        Self::try_new(60).unwrap()
    }

    /// Convert to chrono Duration
    pub fn to_duration(&self) -> Duration {
        Duration::minutes(self.into_inner())
    }
}

/// Timestamp in seconds since Unix epoch
///
/// Used for historical date constants and validation.
/// Range: 0 to i64::MAX
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
pub struct EpochSeconds(i64);

impl EpochSeconds {
    /// January 1, 2000 (common "too old" threshold)
    pub fn year_2000() -> Self {
        Self::try_new(946684800).unwrap()
    }

    /// January 1, 2020 (system start date threshold)
    pub fn year_2020() -> Self {
        Self::try_new(1577836800).unwrap()
    }

    /// January 1, 2024
    pub fn year_2024() -> Self {
        Self::try_new(1704067200).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hours_validation() {
        assert!(Hours::try_new(1).is_ok());
        assert!(Hours::try_new(24).is_ok());
        assert!(Hours::try_new(8760).is_ok());
        assert!(Hours::try_new(0).is_err());
        assert!(Hours::try_new(8761).is_err());
    }

    #[test]
    fn test_days_validation() {
        assert!(Days::try_new(1).is_ok());
        assert!(Days::try_new(365).is_ok());
        assert!(Days::try_new(3650).is_ok());
        assert!(Days::try_new(0).is_err());
        assert!(Days::try_new(3651).is_err());
    }

    #[test]
    fn test_minutes_validation() {
        assert!(Minutes::try_new(1).is_ok());
        assert!(Minutes::try_new(60).is_ok());
        assert!(Minutes::try_new(10080).is_ok());
        assert!(Minutes::try_new(0).is_err());
        assert!(Minutes::try_new(10081).is_err());
    }

    #[test]
    fn test_duration_conversions() {
        assert_eq!(Hours::one_day().to_duration(), Duration::hours(24));
        assert_eq!(Days::one_week().to_duration(), Duration::days(7));
        assert_eq!(Minutes::one_hour().to_duration(), Duration::minutes(60));
        assert_eq!(Days::one_week().to_hours(), 168);
    }
}
