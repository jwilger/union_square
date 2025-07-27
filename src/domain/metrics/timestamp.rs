//! Timestamp domain type for F-score metrics
//!
//! Provides a validated timestamp type for metric data points with
//! reasonable bounds for system operation.

use chrono::{DateTime, Duration, Utc};
use nutype::nutype;
use serde::{Deserialize, Serialize};

/// A validated timestamp for metric measurements
///
/// Ensures timestamps are within reasonable bounds for the system.
/// Prevents future timestamps and very old timestamps that might indicate data corruption.
#[nutype(
    validate(predicate = is_valid_metric_timestamp),
    derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)
)]
pub struct Timestamp(DateTime<Utc>);

impl Timestamp {
    /// Current timestamp
    pub fn now() -> Self {
        Self::try_new(Utc::now()).expect("Current time should always be valid")
    }

    /// Create timestamp from seconds since Unix epoch
    pub fn from_timestamp_secs(secs: i64) -> Option<Self> {
        DateTime::from_timestamp(secs, 0).and_then(|dt| Self::try_new(dt).ok())
    }

    /// Get the underlying DateTime
    pub fn into_datetime(self) -> DateTime<Utc> {
        self.into_inner()
    }

    /// Check if this timestamp is recent (within last 24 hours)
    pub fn is_recent(&self) -> bool {
        let now = Utc::now();
        let age = now.signed_duration_since(self.into_inner());
        age <= Duration::hours(24)
    }

    /// Check if this timestamp is very old (older than 1 year)
    pub fn is_very_old(&self) -> bool {
        let now = Utc::now();
        let age = now.signed_duration_since(self.into_inner());
        age >= Duration::days(365)
    }

    /// Get age category for this timestamp
    pub fn age_category(&self) -> TimestampAge {
        let now = Utc::now();
        let age = now.signed_duration_since(self.into_inner());

        match age {
            d if d <= Duration::hours(1) => TimestampAge::VeryRecent,
            d if d <= Duration::hours(24) => TimestampAge::Recent,
            d if d <= Duration::days(7) => TimestampAge::ThisWeek,
            d if d <= Duration::days(30) => TimestampAge::ThisMonth,
            d if d <= Duration::days(90) => TimestampAge::ThisQuarter,
            d if d <= Duration::days(365) => TimestampAge::ThisYear,
            _ => TimestampAge::Historical,
        }
    }
}

/// Age category for timestamps
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimestampAge {
    /// Within the last hour
    VeryRecent,
    /// Within the last 24 hours
    Recent,
    /// Within the last week
    ThisWeek,
    /// Within the last month
    ThisMonth,
    /// Within the last quarter
    ThisQuarter,
    /// Within the last year
    ThisYear,
    /// Older than 1 year
    Historical,
}

/// Validation function for metric timestamps
fn is_valid_metric_timestamp(dt: &DateTime<Utc>) -> bool {
    let now = Utc::now();

    // Not in the future (with small tolerance for clock skew)
    let future_tolerance = Duration::minutes(5);
    if *dt > now + future_tolerance {
        return false;
    }

    // Not too old (system started around 2024, so anything before 2020 is suspicious)
    let min_valid_date = DateTime::from_timestamp(1577836800, 0).unwrap(); // 2020-01-01
    if *dt < min_valid_date {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timestamp_validation() {
        // Valid: current time
        let _timestamp = Timestamp::now(); // This should always succeed

        // Valid: recent past
        let recent = Utc::now() - Duration::hours(1);
        assert!(Timestamp::try_new(recent).is_ok());

        // Invalid: future
        let future = Utc::now() + Duration::hours(1);
        assert!(Timestamp::try_new(future).is_err());

        // Invalid: too old
        let too_old = DateTime::from_timestamp(946684800, 0).unwrap(); // 2000-01-01
        assert!(Timestamp::try_new(too_old).is_err());
    }

    #[test]
    fn test_age_categories() {
        let now = Utc::now();

        let very_recent = Timestamp::try_new(now - Duration::minutes(30)).unwrap();
        assert_eq!(very_recent.age_category(), TimestampAge::VeryRecent);

        let recent = Timestamp::try_new(now - Duration::hours(12)).unwrap();
        assert_eq!(recent.age_category(), TimestampAge::Recent);

        let this_week = Timestamp::try_new(now - Duration::days(3)).unwrap();
        assert_eq!(this_week.age_category(), TimestampAge::ThisWeek);
    }

    #[test]
    fn test_from_timestamp_secs() {
        // Valid timestamp
        let valid_ts = 1704067200; // 2024-01-01
        assert!(Timestamp::from_timestamp_secs(valid_ts).is_some());

        // Invalid timestamp (too old)
        let invalid_ts = 946684800; // 2000-01-01
        assert!(Timestamp::from_timestamp_secs(invalid_ts).is_none());
    }
}
