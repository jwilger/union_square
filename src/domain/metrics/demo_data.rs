//! Demo data generation for F-score tracking and analytics
//!
//! This module provides functionality to generate realistic demo data for
//! F-score tracking to support MVP Phase 4 dashboard visualization with
//! placeholder data until the full test execution engine is available.

use crate::domain::{
    config_types::ProviderName,
    llm::{LlmProvider, ModelVersion},
    metrics::{
        constants, ui_types::PerformanceCategory, ApplicationCount, ConfidenceLevel,
        DataPointCount, DaysBack, FScore, FScoreDataPoint, MetricValue, ModelCount, PointsPerDay,
        Precision, Recall, SampleCount, StabilityThreshold, TimePeriod, Timestamp, TrendAnalysis,
        TrendDirection, TrendMagnitude,
    },
    session::ApplicationId,
    test_data::{self, f_scores},
    types::ModelId,
};
use chrono::{Duration, Utc};

/// Demo F-score data generator for MVP visualization
pub struct FScoreDemoDataGenerator;

impl FScoreDemoDataGenerator {
    /// Generate demo F-score data points for a model version over time
    pub fn generate_model_timeseries(
        _model_version: &ModelVersion,
        time_period: TimePeriod,
    ) -> Vec<FScoreDataPoint> {
        let mut data_points = Vec::new();
        let now = Utc::now();
        let days_back = time_period.days_back().into_inner();
        let points_per_day = time_period.points_per_day().into_inner();

        for day in 0..days_back {
            let day_start = now - Duration::days(day);

            for point in 0..points_per_day {
                let datetime =
                    day_start + Duration::hours(point as i64 * 24 / points_per_day as i64);
                let timestamp = Timestamp::try_new(datetime).unwrap_or_else(|_| Timestamp::now());

                // Generate realistic F-score trends (slightly declining over time for demo)
                let base_precision = f_scores::HIGH_PRECISION
                    - (day as f64 * constants::demo_generation::trends::PRECISION_DECLINE_RATE);
                let base_recall = f_scores::HIGH_RECALL
                    - (day as f64 * constants::demo_generation::trends::RECALL_DECLINE_RATE);

                // Add some randomness to make it realistic
                let precision_variance =
                    constants::demo_generation::trends::PRECISION_VARIANCE_AMPLITUDE
                        * ((point as f64
                            * constants::demo_generation::trends::PRECISION_WAVE_FREQUENCY)
                            .sin());
                let recall_variance = constants::demo_generation::trends::RECALL_VARIANCE_AMPLITUDE
                    * ((point as f64 * constants::demo_generation::trends::RECALL_WAVE_FREQUENCY)
                        .cos());

                let precision = Precision::try_new((base_precision + precision_variance).clamp(
                    constants::demo_generation::bounds::MIN_DEMO_VALUE,
                    constants::demo_generation::bounds::MAX_VALUE,
                ))
                .unwrap();

                let recall = Recall::try_new((base_recall + recall_variance).clamp(
                    constants::demo_generation::bounds::MIN_DEMO_VALUE,
                    constants::demo_generation::bounds::MAX_VALUE,
                ))
                .unwrap();

                let sample_count = match day
                    % constants::demo_generation::sample_increments::SAMPLE_CYCLE_MODULO
                {
                    0 => SampleCount::try_new(f_scores::LARGE_SAMPLE).unwrap(),
                    1 => SampleCount::try_new(f_scores::MEDIUM_SAMPLE).unwrap(),
                    _ => SampleCount::try_new(f_scores::SMALL_SAMPLE).unwrap(),
                };

                if let Ok(data_point) = FScoreDataPoint::with_precision_recall(
                    timestamp,
                    precision,
                    recall,
                    sample_count,
                ) {
                    let data_point =
                        data_point.with_confidence(ConfidenceLevel::ninety_five_percent());
                    data_points.push(data_point);
                }
            }
        }

        // Sort by timestamp
        data_points.sort_by_key(|dp| dp.timestamp());
        data_points
    }

    /// Generate demo F-score data for different model providers
    pub fn generate_provider_comparison_data() -> Vec<(ModelVersion, Vec<FScoreDataPoint>)> {
        let providers_and_models = vec![
            (LlmProvider::OpenAI, test_data::model_ids::GPT_4_TURBO),
            (LlmProvider::OpenAI, test_data::model_ids::GPT_35_TURBO),
            (LlmProvider::Anthropic, test_data::model_ids::CLAUDE_OPUS),
            (LlmProvider::Anthropic, test_data::model_ids::CLAUDE_SONNET),
            (
                LlmProvider::Other(ProviderName::try_new("AmazonBedrock".to_string()).unwrap()),
                test_data::model_ids::TITAN_EXPRESS,
            ),
        ];

        providers_and_models
            .into_iter()
            .map(|(provider, model_id_str)| {
                let model_id = ModelId::try_new(model_id_str.to_string()).unwrap();
                let model_version = ModelVersion { provider, model_id };
                let data =
                    Self::generate_model_timeseries(&model_version, TimePeriod::demo_period());
                (model_version, data)
            })
            .collect()
    }

    /// Generate demo application-specific F-score data
    pub fn generate_application_data() -> Vec<(ApplicationId, Vec<FScoreDataPoint>)> {
        let applications = vec![
            test_data::application_ids::MY_APP,
            test_data::application_ids::MY_APPLICATION,
            test_data::application_ids::APP_123,
        ];

        applications
            .into_iter()
            .map(|app_id_str| {
                let app_id = ApplicationId::try_new(app_id_str.to_string()).unwrap();
                let data = Self::generate_application_timeseries(
                    &app_id,
                    TimePeriod::new(DaysBack::two_weeks(), PointsPerDay::four_hourly()),
                );
                (app_id, data)
            })
            .collect()
    }

    /// Generate demo F-score data for a specific application
    pub fn generate_application_timeseries(
        application_id: &ApplicationId,
        time_period: TimePeriod,
    ) -> Vec<FScoreDataPoint> {
        let mut data_points = Vec::new();
        let now = Utc::now();
        let days_back = time_period.days_back().into_inner();
        let points_per_day = time_period.points_per_day().into_inner();

        // Each application has different baseline performance
        // For demo purposes, we map specific test applications to performance levels
        let (base_precision, base_recall) =
            if application_id.as_ref() == test_data::application_ids::MY_APP {
                (f_scores::HIGH_PRECISION, f_scores::HIGH_RECALL)
            } else if application_id.as_ref() == test_data::application_ids::MY_APPLICATION {
                (f_scores::MEDIUM_PRECISION, f_scores::MEDIUM_RECALL)
            } else {
                (f_scores::LOW_PRECISION, f_scores::LOW_RECALL)
            };

        for day in 0..days_back {
            let day_start = now - Duration::days(day);

            for point in 0..points_per_day {
                let datetime =
                    day_start + Duration::hours(point as i64 * 24 / points_per_day as i64);
                let timestamp = Timestamp::try_new(datetime).unwrap_or_else(|_| Timestamp::now());

                // Application-specific trends
                let precision_trend = if application_id.as_ref()
                    == test_data::application_ids::MY_APP
                {
                    constants::demo_generation::application_trends::IMPROVING_TREND_RATE
                // Improving
                } else if application_id.as_ref() == test_data::application_ids::MY_APPLICATION {
                    constants::demo_generation::application_trends::SLIGHT_DECLINE_RATE
                // Slightly declining
                } else {
                    constants::demo_generation::application_trends::RAPID_DECLINE_RATE
                    // Declining more rapidly
                };

                let precision =
                    Precision::try_new((base_precision + day as f64 * precision_trend).clamp(
                        constants::demo_generation::bounds::APPLICATION_MIN_VALUE,
                        constants::demo_generation::bounds::MAX_VALUE,
                    ))
                    .unwrap();

                let recall = Recall::try_new(
                    (base_recall
                        + day as f64
                            * precision_trend
                            * constants::demo_generation::application_trends::RECALL_TREND_FACTOR)
                        .clamp(
                            constants::demo_generation::bounds::APPLICATION_MIN_VALUE,
                            constants::demo_generation::bounds::MAX_VALUE,
                        ),
                )
                .unwrap();

                let sample_count = SampleCount::try_new(
                    f_scores::MEDIUM_SAMPLE
                        + ((point as u64)
                            * constants::demo_generation::sample_increments::BASE_SAMPLE_INCREMENT),
                )
                .unwrap();

                if let Ok(data_point) = FScoreDataPoint::with_precision_recall(
                    timestamp,
                    precision,
                    recall,
                    sample_count,
                ) {
                    data_points.push(data_point);
                }
            }
        }

        data_points.sort_by_key(|dp| dp.timestamp());
        data_points
    }

    /// Generate demo data showing F-score performance ranges
    pub fn generate_performance_categories() -> Vec<(PerformanceCategory, FScoreDataPoint)> {
        let now = Timestamp::now();

        vec![
            (
                PerformanceCategory::Excellent,
                Self::create_demo_point(
                    now,
                    f_scores::HIGH_PRECISION,
                    f_scores::HIGH_RECALL,
                    SampleCount::try_new(f_scores::LARGE_SAMPLE).unwrap(),
                ),
            ),
            (
                PerformanceCategory::Good,
                Self::create_demo_point(
                    // Use slightly older timestamp that's still valid
                    Timestamp::try_new(now.into_datetime() - Duration::hours(1)).unwrap_or(now),
                    f_scores::MEDIUM_PRECISION,
                    f_scores::MEDIUM_RECALL,
                    SampleCount::try_new(f_scores::MEDIUM_SAMPLE).unwrap(),
                ),
            ),
            (
                PerformanceCategory::NeedsImprovement,
                Self::create_demo_point(
                    Timestamp::try_new(now.into_datetime() - Duration::hours(2)).unwrap_or(now),
                    f_scores::LOW_PRECISION,
                    f_scores::LOW_RECALL,
                    SampleCount::try_new(f_scores::SMALL_SAMPLE).unwrap(),
                ),
            ),
            (
                PerformanceCategory::Critical,
                Self::create_demo_point(
                    Timestamp::try_new(now.into_datetime() - Duration::hours(3)).unwrap_or(now),
                    constants::demo_generation::bounds::CRITICAL_PRECISION_THRESHOLD,
                    constants::demo_generation::bounds::CRITICAL_RECALL_THRESHOLD,
                    SampleCount::try_new(f_scores::TINY_SAMPLE).unwrap(),
                ),
            ),
        ]
    }

    /// Helper to create a demo data point
    fn create_demo_point(
        timestamp: Timestamp,
        precision_val: f64,
        recall_val: f64,
        sample_count: SampleCount,
    ) -> FScoreDataPoint {
        let precision = Precision::try_new(precision_val).unwrap();
        let recall = Recall::try_new(recall_val).unwrap();

        FScoreDataPoint::with_precision_recall(timestamp, precision, recall, sample_count)
            .unwrap()
            .with_confidence(ConfidenceLevel::ninety_five_percent())
    }

    /// Generate summary statistics for demo dashboard
    pub fn generate_summary_stats() -> DemoSummaryStats {
        let overall_data = Self::generate_model_timeseries(
            &ModelVersion {
                provider: LlmProvider::OpenAI,
                model_id: ModelId::try_new(test_data::model_ids::GPT_4_TURBO.to_string()).unwrap(),
            },
            TimePeriod::recent_detailed(),
        );

        let current_f_score = overall_data
            .last()
            .map(|dp| dp.f_score())
            .unwrap_or(FScore::zero());
        let previous_f_score = overall_data
            .get(overall_data.len().saturating_sub(24))
            .map(|dp| dp.f_score())
            .unwrap_or(FScore::zero());

        let _trend_value = current_f_score.into_inner() - previous_f_score.into_inner();

        let current_metric = MetricValue::try_new(current_f_score.into_inner()).unwrap();
        let previous_metric = MetricValue::try_new(previous_f_score.into_inner()).unwrap();
        let stability_threshold =
            StabilityThreshold::try_new(constants::statistical::DEFAULT_STABILITY_THRESHOLD)
                .unwrap();

        let trend_analysis =
            TrendAnalysis::from_values(current_metric, previous_metric, stability_threshold)
                .unwrap_or_else(|_| {
                    // Fallback to stable trend if calculation fails
                    TrendAnalysis::new(
                        TrendDirection::Stable,
                        TrendMagnitude::try_new(0.0).unwrap(),
                    )
                });

        DemoSummaryStats {
            current_f_score,
            previous_f_score,
            trend_analysis,
            total_models_tracked: ModelCount::try_new(5).unwrap(),
            total_applications_tracked: ApplicationCount::try_new(3).unwrap(),
            total_data_points: DataPointCount::try_new(overall_data.len()).unwrap(),
        }
    }
}

/// Demo summary statistics for dashboard display
#[derive(Debug, Clone)]
pub struct DemoSummaryStats {
    pub current_f_score: FScore,
    pub previous_f_score: FScore,
    pub trend_analysis: TrendAnalysis,
    pub total_models_tracked: ModelCount,
    pub total_applications_tracked: ApplicationCount,
    pub total_data_points: DataPointCount,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_timeseries_generation() {
        let model_version = ModelVersion {
            provider: LlmProvider::OpenAI,
            model_id: ModelId::try_new(test_data::model_ids::GPT_4_TURBO.to_string()).unwrap(),
        };

        let data = FScoreDemoDataGenerator::generate_model_timeseries(
            &model_version,
            TimePeriod::new(DaysBack::week(), PointsPerDay::six_hourly()),
        );

        assert_eq!(data.len(), 28); // 7 days * 4 points per day

        // Check that data is sorted by timestamp
        for i in 1..data.len() {
            assert!(data[i].timestamp() >= data[i - 1].timestamp());
        }

        // Check that all F-scores are valid
        for point in &data {
            assert!(point.f_score().into_inner() >= 0.0);
            assert!(point.f_score().into_inner() <= 1.0);
            assert!(point.sample_count().into_inner() > 0);
        }
    }

    #[test]
    fn test_provider_comparison_data() {
        let data = FScoreDemoDataGenerator::generate_provider_comparison_data();

        assert_eq!(data.len(), 5); // 5 different model versions

        for (model_version, time_series) in data {
            assert!(!time_series.is_empty());
            // Each provider should have data
            assert!(matches!(
                model_version.provider,
                LlmProvider::OpenAI | LlmProvider::Anthropic | LlmProvider::Other(_)
            ));
        }
    }

    #[test]
    fn test_application_data_generation() {
        let data = FScoreDemoDataGenerator::generate_application_data();

        assert_eq!(data.len(), 3); // 3 applications

        for (app_id, time_series) in data {
            assert!(!time_series.is_empty());
            assert!(!app_id.as_ref().is_empty());
        }
    }

    #[test]
    fn test_performance_categories() {
        let categories = FScoreDemoDataGenerator::generate_performance_categories();

        assert_eq!(categories.len(), 4);

        let category_types: Vec<_> = categories.iter().map(|(cat, _)| cat.clone()).collect();
        assert!(category_types.contains(&PerformanceCategory::Excellent));
        assert!(category_types.contains(&PerformanceCategory::Good));
        assert!(category_types.contains(&PerformanceCategory::NeedsImprovement));
        assert!(category_types.contains(&PerformanceCategory::Critical));
    }

    #[test]
    fn test_summary_stats_generation() {
        let stats = FScoreDemoDataGenerator::generate_summary_stats();

        assert!(stats.total_models_tracked.into_inner() > 0);
        assert!(stats.total_applications_tracked.into_inner() > 0);
        assert!(stats.total_data_points.into_inner() > 0);
        // TrendAnalysis should have a valid direction (not checking specific value since it's calculated)
        assert!(matches!(
            stats.trend_analysis.direction,
            TrendDirection::Improving | TrendDirection::Declining | TrendDirection::Stable
        ));
        assert!(stats.trend_analysis.magnitude.into_inner() >= 0.0);
    }
}
