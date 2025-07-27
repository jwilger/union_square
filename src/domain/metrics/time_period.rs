//! Time period types for metrics analysis

use nutype::nutype;
use serde::{Deserialize, Serialize};

/// Number of days to look back for historical data
#[nutype(
    validate(greater = 0, less_or_equal = 365),
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
pub struct DaysBack(i64);

impl DaysBack {
    /// Last week (7 days)
    pub fn week() -> Self {
        Self::try_new(7).unwrap()
    }

    /// Last two weeks (14 days)
    pub fn two_weeks() -> Self {
        Self::try_new(14).unwrap()
    }

    /// Last month (30 days)
    pub fn month() -> Self {
        Self::try_new(30).unwrap()
    }

    /// Last quarter (90 days)
    pub fn quarter() -> Self {
        Self::try_new(90).unwrap()
    }

    /// Last year (365 days)
    pub fn year() -> Self {
        Self::try_new(365).unwrap()
    }
}

/// Number of data points to generate per day
#[nutype(
    validate(greater = 0, less_or_equal = 288), // Max every 5 minutes
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
pub struct PointsPerDay(usize);

impl PointsPerDay {
    /// Hourly data points (24 per day)
    pub fn hourly() -> Self {
        Self::try_new(24).unwrap()
    }

    /// Every 6 hours (4 per day)
    pub fn six_hourly() -> Self {
        Self::try_new(4).unwrap()
    }

    /// Every 4 hours (6 per day)
    pub fn four_hourly() -> Self {
        Self::try_new(6).unwrap()
    }

    /// Every 3 hours (8 per day)
    pub fn three_hourly() -> Self {
        Self::try_new(8).unwrap()
    }

    /// Every 2 hours (12 per day)
    pub fn two_hourly() -> Self {
        Self::try_new(12).unwrap()
    }

    /// Calculate hours between points
    pub fn hours_between_points(&self) -> f64 {
        24.0 / self.into_inner() as f64
    }
}

/// Time period configuration for data generation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimePeriod {
    pub days_back: DaysBack,
    pub points_per_day: PointsPerDay,
}

impl TimePeriod {
    /// Create a new time period
    pub fn new(days_back: DaysBack, points_per_day: PointsPerDay) -> Self {
        Self {
            days_back,
            points_per_day,
        }
    }

    /// Total number of data points this period will generate
    pub fn total_points(&self) -> usize {
        self.days_back.into_inner() as usize * self.points_per_day.into_inner()
    }

    /// Duration in hours
    pub fn duration_hours(&self) -> f64 {
        self.days_back.into_inner() as f64 * 24.0
    }

    /// Data density (points per hour)
    pub fn data_density(&self) -> f64 {
        self.points_per_day.into_inner() as f64 / 24.0
    }
}

/// Common time period presets
impl TimePeriod {
    /// High-resolution recent data (hourly for last week)
    pub fn recent_detailed() -> Self {
        Self::new(DaysBack::week(), PointsPerDay::hourly())
    }

    /// Medium-resolution monthly data (6-hourly for last month)
    pub fn monthly_overview() -> Self {
        Self::new(DaysBack::month(), PointsPerDay::six_hourly())
    }

    /// Low-resolution quarterly data (daily for last quarter)
    pub fn quarterly_trends() -> Self {
        Self::new(DaysBack::quarter(), PointsPerDay::try_new(1).unwrap())
    }

    /// Demo data period (4 points per day for 30 days)
    pub fn demo_period() -> Self {
        Self::new(DaysBack::month(), PointsPerDay::six_hourly())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_days_back_validation() {
        assert!(DaysBack::try_new(1).is_ok());
        assert!(DaysBack::try_new(365).is_ok());
        assert!(DaysBack::try_new(0).is_err());
        assert!(DaysBack::try_new(366).is_err());
        assert!(DaysBack::try_new(-1).is_err());
    }

    #[test]
    fn test_points_per_day_validation() {
        assert!(PointsPerDay::try_new(1).is_ok());
        assert!(PointsPerDay::try_new(288).is_ok());
        assert!(PointsPerDay::try_new(0).is_err());
        assert!(PointsPerDay::try_new(289).is_err());
    }

    #[test]
    fn test_time_period_calculations() {
        let period = TimePeriod::new(DaysBack::week(), PointsPerDay::hourly());
        assert_eq!(period.total_points(), 7 * 24);
        assert_eq!(period.duration_hours(), 7.0 * 24.0);
        assert_eq!(period.data_density(), 1.0); // 1 point per hour
    }

    #[test]
    fn test_preset_periods() {
        let recent = TimePeriod::recent_detailed();
        assert_eq!(recent.days_back, DaysBack::week());
        assert_eq!(recent.points_per_day, PointsPerDay::hourly());

        let monthly = TimePeriod::monthly_overview();
        assert_eq!(monthly.days_back, DaysBack::month());
        assert_eq!(monthly.points_per_day, PointsPerDay::six_hourly());
    }
}
