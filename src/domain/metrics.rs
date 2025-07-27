//! F-score tracking and analytics domain types
//!
//! This module provides type-safe F-score calculation and tracking functionality
//! following type-driven development principles for precision/recall metrics.

pub mod constants;
pub mod core_metrics;
pub mod counts;
pub mod data_point;
pub mod demo_data;
pub mod demo_types;
pub mod durations;
pub mod errors;
pub mod performance;
pub mod sample_count;
pub mod time_period;
pub mod timestamp;
pub mod trend;
pub mod ui_types;
pub mod values;

// Re-export commonly used types
pub use core_metrics::{Beta, ConfidenceLevel, FScore, Precision, Recall};
pub use counts::{ApplicationCount, DataPointCount, ModelCount};
pub use data_point::FScoreDataPoint;
pub use errors::MetricsError;
pub use performance::{PerformanceAssessment, PerformanceLevel, QualityRating};
pub use sample_count::{SampleConfidence, SampleCount};
pub use time_period::{DaysBack, PointsPerDay, TimePeriod};
pub use timestamp::{Timestamp, TimestampAge};
pub use trend::{TrendAnalysis, TrendDirection, TrendMagnitude};
pub use values::{MetricValue, PercentageChange, StabilityThreshold};
