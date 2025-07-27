//! Constants for F-score metrics calculations and demo data generation

/// Mathematical constants for F-score calculations
pub mod calculation {
    /// Harmonic mean multiplier for F1 score
    pub const F1_MULTIPLIER: f64 = 2.0;

    /// Beta value for standard F1 score
    pub const STANDARD_BETA: f64 = 1.0;

    /// Beta value for F2 score (emphasizes recall)
    pub const F2_BETA: f64 = 2.0;

    /// Beta value for F0.5 score (emphasizes precision)
    pub const F05_BETA: f64 = 0.5;
}

/// Constants for demo data generation
pub mod demo_generation {
    /// Time-based trends and variance
    pub mod trends {
        /// Daily precision decline rate for demo data
        pub const PRECISION_DECLINE_RATE: f64 = 0.001;

        /// Daily recall decline rate for demo data
        pub const RECALL_DECLINE_RATE: f64 = 0.0015;

        /// Precision variance amplitude for realistic fluctuation
        pub const PRECISION_VARIANCE_AMPLITUDE: f64 = 0.02;

        /// Recall variance amplitude for realistic fluctuation
        pub const RECALL_VARIANCE_AMPLITUDE: f64 = 0.015;

        /// Sine wave frequency for precision variance
        pub const PRECISION_WAVE_FREQUENCY: f64 = 7.0;

        /// Cosine wave frequency for recall variance
        pub const RECALL_WAVE_FREQUENCY: f64 = 5.0;
    }

    /// Bounds and thresholds
    pub mod bounds {
        /// Minimum acceptable precision/recall for demo data
        pub const MIN_DEMO_VALUE: f64 = 0.5;

        /// Maximum precision/recall value
        pub const MAX_VALUE: f64 = 1.0;

        /// Lower bound for application-specific demo data
        pub const APPLICATION_MIN_VALUE: f64 = 0.3;

        /// Critical threshold for "Critical Issues" category
        pub const CRITICAL_PRECISION_THRESHOLD: f64 = 0.45;

        /// Critical threshold for recall in "Critical Issues" category
        pub const CRITICAL_RECALL_THRESHOLD: f64 = 0.42;
    }

    /// Application-specific trend rates
    pub mod application_trends {
        /// Improving application trend rate (positive)
        pub const IMPROVING_TREND_RATE: f64 = 0.001;

        /// Slightly declining application trend rate
        pub const SLIGHT_DECLINE_RATE: f64 = -0.0005;

        /// Rapidly declining application trend rate
        pub const RAPID_DECLINE_RATE: f64 = -0.002;

        /// Recall trend factor relative to precision
        pub const RECALL_TREND_FACTOR: f64 = 0.8;
    }

    /// Sample count increments and base values
    pub mod sample_increments {
        /// Base increment for sample count variation
        pub const BASE_SAMPLE_INCREMENT: u64 = 50;

        /// Sample count modulo for cycling through sizes
        pub const SAMPLE_CYCLE_MODULO: i64 = 3;
    }
}

/// Performance categorization thresholds
pub mod performance_thresholds {
    /// Threshold for excellent performance
    pub const EXCELLENT_THRESHOLD: f64 = 0.9;

    /// Threshold for good performance
    pub const GOOD_THRESHOLD: f64 = 0.8;

    /// Threshold for acceptable performance
    pub const ACCEPTABLE_THRESHOLD: f64 = 0.7;

    /// Threshold for needs improvement
    pub const NEEDS_IMPROVEMENT_THRESHOLD: f64 = 0.6;

    /// Below this threshold is considered critical
    pub const CRITICAL_THRESHOLD: f64 = 0.5;
}

/// Quality rating thresholds for precision/recall balance
pub mod quality_thresholds {
    /// High precision/recall threshold
    pub const HIGH_THRESHOLD: f64 = 0.8;

    /// Moderate precision/recall threshold
    pub const MODERATE_THRESHOLD: f64 = 0.6;

    /// Good but not great threshold for imbalance detection
    pub const GOOD_THRESHOLD: f64 = 0.7;

    /// Poor performance threshold for imbalance detection
    pub const POOR_THRESHOLD: f64 = 0.5;
}

/// Statistical significance constants
pub mod statistical {
    /// Minimum sample size for statistical significance (Central Limit Theorem)
    pub const MIN_SIGNIFICANT_SAMPLE_SIZE: u64 = 30;

    /// Recommended minimum sample size for reliable metrics
    pub const RECOMMENDED_MIN_SAMPLE_SIZE: u64 = 100;

    /// Large sample size threshold
    pub const LARGE_SAMPLE_THRESHOLD: u64 = 1000;

    /// Default stability threshold for trend analysis
    pub const DEFAULT_STABILITY_THRESHOLD: f64 = 0.01; // 1%

    /// Default confidence level
    pub const DEFAULT_CONFIDENCE_LEVEL: f64 = 0.95; // 95%
}

/// Time-based constants
pub mod time_intervals {
    /// Hours in a day
    pub const HOURS_PER_DAY: i64 = 24;

    /// Maximum reasonable lookback period in days
    pub const MAX_LOOKBACK_DAYS: i64 = 365;

    /// Default demo period in days
    pub const DEFAULT_DEMO_PERIOD_DAYS: i64 = 30;

    /// Default points per day for demo data
    pub const DEFAULT_DEMO_POINTS_PER_DAY: usize = 4;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculation_constants() {
        assert_eq!(calculation::F1_MULTIPLIER, 2.0);
        assert_eq!(calculation::STANDARD_BETA, 1.0);
        assert_eq!(calculation::F2_BETA, 2.0);
        assert_eq!(calculation::F05_BETA, 0.5);
    }

    #[test]
    fn test_performance_thresholds_are_ordered() {
        // Compile-time checks for threshold ordering
        const _: () = assert!(
            performance_thresholds::EXCELLENT_THRESHOLD > performance_thresholds::GOOD_THRESHOLD
        );
        const _: () = assert!(
            performance_thresholds::GOOD_THRESHOLD > performance_thresholds::ACCEPTABLE_THRESHOLD
        );
        const _: () = assert!(
            performance_thresholds::ACCEPTABLE_THRESHOLD
                > performance_thresholds::NEEDS_IMPROVEMENT_THRESHOLD
        );
        const _: () = assert!(
            performance_thresholds::NEEDS_IMPROVEMENT_THRESHOLD
                > performance_thresholds::CRITICAL_THRESHOLD
        );
    }

    #[test]
    fn test_statistical_constants() {
        // Compile-time checks for statistical constants
        const _: () = assert!(
            statistical::RECOMMENDED_MIN_SAMPLE_SIZE >= statistical::MIN_SIGNIFICANT_SAMPLE_SIZE
        );
        const _: () =
            assert!(statistical::LARGE_SAMPLE_THRESHOLD > statistical::RECOMMENDED_MIN_SAMPLE_SIZE);
    }
}
