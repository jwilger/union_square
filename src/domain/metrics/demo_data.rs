//! Demo data generation for F-score tracking and analytics
//!
//! This module provides functionality to generate realistic demo data for
//! F-score tracking to support MVP Phase 4 dashboard visualization with
//! placeholder data until the full test execution engine is available.

use chrono::{DateTime, Duration, Utc};
use crate::domain::{
    llm::{LlmProvider, ModelVersion},
    metrics::{FScore, FScoreDataPoint, Precision, Recall, ConfidenceLevel},
    session::{ApplicationId, SessionId},
    test_data::{self, f_scores},
    types::ModelId,
};

/// Demo F-score data generator for MVP visualization
pub struct FScoreDemoDataGenerator;

impl FScoreDemoDataGenerator {
    /// Generate demo F-score data points for a model version over time
    pub fn generate_model_timeseries(
        model_version: &ModelVersion,
        days_back: i64,
        points_per_day: usize,
    ) -> Vec<FScoreDataPoint> {
        let mut data_points = Vec::new();
        let now = Utc::now();

        for day in 0..days_back {
            let day_start = now - Duration::days(day);

            for point in 0..points_per_day {
                let timestamp = day_start + Duration::hours(point as i64 * 24 / points_per_day as i64);

                // Generate realistic F-score trends (slightly declining over time for demo)
                let base_precision = f_scores::HIGH_PRECISION - (day as f64 * 0.001);
                let base_recall = f_scores::HIGH_RECALL - (day as f64 * 0.0015);

                // Add some randomness to make it realistic
                let precision_variance = 0.02 * ((point as f64 * 7.0).sin());
                let recall_variance = 0.015 * ((point as f64 * 5.0).cos());

                let precision = Precision::try_new(
                    (base_precision + precision_variance).max(0.5).min(1.0)
                ).unwrap();

                let recall = Recall::try_new(
                    (base_recall + recall_variance).max(0.5).min(1.0)
                ).unwrap();

                let sample_count = match day % 3 {
                    0 => f_scores::LARGE_SAMPLE,
                    1 => f_scores::MEDIUM_SAMPLE,
                    _ => f_scores::SMALL_SAMPLE,
                };

                if let Ok(data_point) = FScoreDataPoint::with_precision_recall(
                    timestamp,
                    precision,
                    recall,
                    sample_count,
                ) {
                    let data_point = data_point.with_confidence(
                        ConfidenceLevel::ninety_five_percent()
                    );
                    data_points.push(data_point);
                }
            }
        }

        // Sort by timestamp
        data_points.sort_by_key(|dp| dp.timestamp);
        data_points
    }

    /// Generate demo F-score data for different model providers
    pub fn generate_provider_comparison_data() -> Vec<(ModelVersion, Vec<FScoreDataPoint>)> {
        let providers_and_models = vec![
            (LlmProvider::OpenAI, test_data::model_ids::GPT_4_TURBO),
            (LlmProvider::OpenAI, test_data::model_ids::GPT_35_TURBO),
            (LlmProvider::Anthropic, test_data::model_ids::CLAUDE_OPUS),
            (LlmProvider::Anthropic, test_data::model_ids::CLAUDE_SONNET),
            (LlmProvider::AmazonBedrock, test_data::model_ids::TITAN_EXPRESS),
        ];

        providers_and_models
            .into_iter()
            .map(|(provider, model_id_str)| {
                let model_id = ModelId::try_new(model_id_str.to_string()).unwrap();
                let model_version = ModelVersion { provider, model_id };
                let data = Self::generate_model_timeseries(&model_version, 30, 4);
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
                let data = Self::generate_application_timeseries(&app_id, 14, 6);
                (app_id, data)
            })
            .collect()
    }

    /// Generate demo F-score data for a specific application
    pub fn generate_application_timeseries(
        application_id: &ApplicationId,
        days_back: i64,
        points_per_day: usize,
    ) -> Vec<FScoreDataPoint> {
        let mut data_points = Vec::new();
        let now = Utc::now();

        // Each application has different baseline performance
        let (base_precision, base_recall) = match application_id.as_ref() {
            app if app.contains("MY_APP") => (f_scores::HIGH_PRECISION, f_scores::HIGH_RECALL),
            app if app.contains("MyApplication") => (f_scores::MEDIUM_PRECISION, f_scores::MEDIUM_RECALL),
            _ => (f_scores::LOW_PRECISION, f_scores::LOW_RECALL),
        };

        for day in 0..days_back {
            let day_start = now - Duration::days(day);

            for point in 0..points_per_day {
                let timestamp = day_start + Duration::hours(point as i64 * 24 / points_per_day as i64);

                // Application-specific trends
                let precision_trend = match application_id.as_ref() {
                    app if app.contains("MY_APP") => 0.001, // Improving
                    app if app.contains("MyApplication") => -0.0005, // Slightly declining
                    _ => -0.002, // Declining more rapidly
                };

                let precision = Precision::try_new(
                    (base_precision + day as f64 * precision_trend).max(0.3).min(1.0)
                ).unwrap();

                let recall = Recall::try_new(
                    (base_recall + day as f64 * precision_trend * 0.8).max(0.3).min(1.0)
                ).unwrap();

                let sample_count = f_scores::MEDIUM_SAMPLE + (point * 50) as u64;

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

        data_points.sort_by_key(|dp| dp.timestamp);
        data_points
    }

    /// Generate demo data showing F-score performance ranges
    pub fn generate_performance_categories() -> Vec<(String, FScoreDataPoint)> {
        let now = Utc::now();

        vec![
            (
                "Excellent Performance".to_string(),
                Self::create_demo_point(
                    now,
                    f_scores::HIGH_PRECISION,
                    f_scores::HIGH_RECALL,
                    f_scores::LARGE_SAMPLE,
                ),
            ),
            (
                "Good Performance".to_string(),
                Self::create_demo_point(
                    now - Duration::hours(1),
                    f_scores::MEDIUM_PRECISION,
                    f_scores::MEDIUM_RECALL,
                    f_scores::MEDIUM_SAMPLE,
                ),
            ),
            (
                "Needs Improvement".to_string(),
                Self::create_demo_point(
                    now - Duration::hours(2),
                    f_scores::LOW_PRECISION,
                    f_scores::LOW_RECALL,
                    f_scores::SMALL_SAMPLE,
                ),
            ),
            (
                "Critical Issues".to_string(),
                Self::create_demo_point(
                    now - Duration::hours(3),
                    0.45,
                    0.42,
                    f_scores::TINY_SAMPLE,
                ),
            ),
        ]
    }

    /// Helper to create a demo data point
    fn create_demo_point(
        timestamp: DateTime<Utc>,
        precision_val: f64,
        recall_val: f64,
        sample_count: u64,
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
            7,
            24,
        );

        let current_f_score = overall_data.last().map(|dp| dp.f_score).unwrap_or(FScore::zero());
        let previous_f_score = overall_data.get(overall_data.len().saturating_sub(24))
            .map(|dp| dp.f_score)
            .unwrap_or(FScore::zero());

        let trend = current_f_score.into_inner() - previous_f_score.into_inner();

        DemoSummaryStats {
            current_f_score,
            previous_f_score,
            trend_direction: if trend > 0.0 { "up" } else { "down" }.to_string(),
            trend_magnitude: trend.abs(),
            total_models_tracked: 5,
            total_applications_tracked: 3,
            total_data_points: overall_data.len(),
        }
    }
}

/// Demo summary statistics for dashboard display
#[derive(Debug, Clone)]
pub struct DemoSummaryStats {
    pub current_f_score: FScore,
    pub previous_f_score: FScore,
    pub trend_direction: String,
    pub trend_magnitude: f64,
    pub total_models_tracked: usize,
    pub total_applications_tracked: usize,
    pub total_data_points: usize,
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

        let data = FScoreDemoDataGenerator::generate_model_timeseries(&model_version, 7, 4);

        assert_eq!(data.len(), 28); // 7 days * 4 points per day

        // Check that data is sorted by timestamp
        for i in 1..data.len() {
            assert!(data[i].timestamp >= data[i-1].timestamp);
        }

        // Check that all F-scores are valid
        for point in &data {
            assert!(point.f_score.into_inner() >= 0.0);
            assert!(point.f_score.into_inner() <= 1.0);
            assert!(point.sample_count > 0);
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
                LlmProvider::OpenAI | LlmProvider::Anthropic | LlmProvider::AmazonBedrock
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

        let category_names: Vec<_> = categories.iter().map(|(name, _)| name.clone()).collect();
        assert!(category_names.contains(&"Excellent Performance".to_string()));
        assert!(category_names.contains(&"Good Performance".to_string()));
        assert!(category_names.contains(&"Needs Improvement".to_string()));
        assert!(category_names.contains(&"Critical Issues".to_string()));
    }

    #[test]
    fn test_summary_stats_generation() {
        let stats = FScoreDemoDataGenerator::generate_summary_stats();

        assert!(stats.total_models_tracked > 0);
        assert!(stats.total_applications_tracked > 0);
        assert!(stats.total_data_points > 0);
        assert!(stats.trend_direction == "up" || stats.trend_direction == "down");
        assert!(stats.trend_magnitude >= 0.0);
    }
}
