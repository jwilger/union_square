//! EventCore commands for F-score and metrics tracking
//!
//! These commands implement the EventCore CommandLogic trait to provide
//! multi-stream event sourcing for F-score tracking and analytics operations.

use eventcore::{CommandError, CommandLogic, NewEvents, StreamId};
use eventcore_macros::Command;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::domain::{
    events::DomainEvent,
    llm::ModelVersion,
    metrics::{FScore, FScoreDataPoint, Precision, Recall, SampleCount, Timestamp},
    session::{ApplicationId, SessionId},
};

/// State for F-score and metrics tracking
#[derive(Debug, Default, Clone)]
pub struct MetricsState {
    /// F-score data points indexed by model version
    pub f_score_data: HashMap<ModelVersion, Vec<FScoreDataPoint>>,
    /// Application-specific F-score data
    pub application_f_scores: HashMap<ApplicationId, Vec<FScoreDataPoint>>,
}

impl MetricsState {
    /// Apply an event to update the state
    pub fn apply(&mut self, event: &DomainEvent) {
        match event {
            DomainEvent::FScoreCalculated {
                model_version,
                f_score,
                precision,
                recall,
                sample_count,
                calculated_at,
                ..
            } => {
                let data_point = if let (Some(precision), Some(recall)) = (precision, recall) {
                    FScoreDataPoint::with_precision_recall(
                        *calculated_at,
                        *precision,
                        *recall,
                        *sample_count,
                    )
                    .unwrap_or_else(|_| {
                        FScoreDataPoint::new(*calculated_at, *f_score, *sample_count)
                    })
                } else {
                    FScoreDataPoint::new(*calculated_at, *f_score, *sample_count)
                };

                self.f_score_data
                    .entry(model_version.clone())
                    .or_default()
                    .push(data_point);
            }
            DomainEvent::ApplicationFScoreCalculated {
                application_id,
                f_score,
                precision,
                recall,
                sample_count,
                calculated_at,
                ..
            } => {
                let data_point = if let (Some(precision), Some(recall)) = (precision, recall) {
                    FScoreDataPoint::with_precision_recall(
                        *calculated_at,
                        *precision,
                        *recall,
                        *sample_count,
                    )
                    .unwrap_or_else(|_| {
                        FScoreDataPoint::new(*calculated_at, *f_score, *sample_count)
                    })
                } else {
                    FScoreDataPoint::new(*calculated_at, *f_score, *sample_count)
                };

                self.application_f_scores
                    .entry(application_id.clone())
                    .or_default()
                    .push(data_point);
            }
            _ => {} // Ignore other events
        }
    }

    /// Get the latest F-score for a model version
    pub fn latest_f_score(&self, model_version: &ModelVersion) -> Option<&FScoreDataPoint> {
        self.f_score_data.get(model_version)?.last()
    }

    /// Get F-score history for a model version
    pub fn f_score_history(&self, model_version: &ModelVersion) -> Option<&[FScoreDataPoint]> {
        self.f_score_data
            .get(model_version)
            .map(|data| data.as_slice())
    }
}

/// Command to record F-score calculation for a model version
#[derive(Debug, Clone, Serialize, Deserialize, Command)]
pub struct RecordModelFScore {
    #[stream]
    model_stream: StreamId,
    pub session_id: SessionId,
    pub model_version: ModelVersion,
    pub precision: Precision,
    pub recall: Recall,
    pub sample_count: SampleCount,
    pub timestamp: Timestamp,
}

/// Stream ID utilities for metrics commands
mod stream_ids {
    use super::*;

    /// Stream ID prefix for model metrics
    const MODEL_STREAM_PREFIX: &str = "metrics:model:";

    /// Stream ID prefix for application metrics
    const APPLICATION_STREAM_PREFIX: &str = "metrics:app:";

    pub fn model_stream_id(model_version: &ModelVersion) -> Result<StreamId, CommandError> {
        StreamId::try_new(format!(
            "{}{}",
            MODEL_STREAM_PREFIX,
            model_version.to_version_string()
        ))
        .map_err(|e| CommandError::ValidationError(format!("Invalid model stream ID: {e}")))
    }

    pub fn application_stream_id(application_id: &ApplicationId) -> Result<StreamId, CommandError> {
        StreamId::try_new(format!(
            "{}{}",
            APPLICATION_STREAM_PREFIX,
            application_id.as_ref()
        ))
        .map_err(|e| CommandError::ValidationError(format!("Invalid application stream ID: {e}")))
    }
}

/// Event construction utilities for metrics commands
mod event_builders {
    use super::*;

    #[allow(clippy::too_many_arguments)]
    pub fn build_f_score_calculated_event(
        stream_id: StreamId,
        session_id: SessionId,
        model_version: ModelVersion,
        f_score: FScore,
        precision: Precision,
        recall: Recall,
        sample_count: SampleCount,
        calculated_at: Timestamp,
    ) -> DomainEvent {
        DomainEvent::FScoreCalculated {
            stream_id,
            session_id,
            model_version,
            f_score,
            precision: Some(precision),
            recall: Some(recall),
            sample_count,
            calculated_at,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn build_application_f_score_calculated_event(
        stream_id: StreamId,
        session_id: SessionId,
        application_id: ApplicationId,
        model_version: ModelVersion,
        f_score: FScore,
        precision: Precision,
        recall: Recall,
        sample_count: SampleCount,
        calculated_at: Timestamp,
    ) -> DomainEvent {
        DomainEvent::ApplicationFScoreCalculated {
            stream_id,
            session_id,
            application_id,
            model_version,
            f_score,
            precision: Some(precision),
            recall: Some(recall),
            sample_count,
            calculated_at,
        }
    }
}

/// Common trait for F-score commands
trait FScoreCommand {
    fn precision(&self) -> Precision;
    fn recall(&self) -> Recall;

    /// Calculate F-score from precision and recall
    fn calculate_f_score(&self) -> Result<FScore, CommandError> {
        FScore::from_precision_recall(self.precision(), self.recall())
            .map_err(|e| CommandError::ValidationError(format!("F-score calculation failed: {e}")))
    }
}

impl RecordModelFScore {
    pub fn new(
        session_id: SessionId,
        model_version: ModelVersion,
        precision: Precision,
        recall: Recall,
        sample_count: SampleCount,
        timestamp: Timestamp,
    ) -> Result<Self, CommandError> {
        let model_stream = stream_ids::model_stream_id(&model_version)?;
        Ok(Self {
            model_stream,
            session_id,
            model_version,
            precision,
            recall,
            sample_count,
            timestamp,
        })
    }
}

impl FScoreCommand for RecordModelFScore {
    fn precision(&self) -> Precision {
        self.precision
    }

    fn recall(&self) -> Recall {
        self.recall
    }
}

impl CommandLogic for RecordModelFScore {
    type State = MetricsState;
    type Event = DomainEvent;

    fn apply(&self, mut state: Self::State, event: &Self::Event) -> Self::State {
        state.apply(event);
        state
    }

    fn handle(&self, _state: Self::State) -> Result<NewEvents<Self::Event>, CommandError> {
        let mut events = Vec::new();

        let f_score = self.calculate_f_score()?;

        events.push(event_builders::build_f_score_calculated_event(
            self.model_stream.clone(),
            self.session_id.clone(),
            self.model_version.clone(),
            f_score,
            self.precision,
            self.recall,
            self.sample_count,
            self.timestamp,
        ));

        Ok(events.into())
    }
}

/// Command to record F-score calculation for an application
#[derive(Debug, Clone, Serialize, Deserialize, Command)]
pub struct RecordApplicationFScore {
    #[stream]
    application_stream: StreamId,
    #[stream]
    model_stream: StreamId,
    pub session_id: SessionId,
    pub application_id: ApplicationId,
    pub model_version: ModelVersion,
    pub precision: Precision,
    pub recall: Recall,
    pub sample_count: SampleCount,
    pub timestamp: Timestamp,
}

impl RecordApplicationFScore {
    pub fn new(
        session_id: SessionId,
        application_id: ApplicationId,
        model_version: ModelVersion,
        precision: Precision,
        recall: Recall,
        sample_count: SampleCount,
        timestamp: Timestamp,
    ) -> Result<Self, CommandError> {
        let application_stream = stream_ids::application_stream_id(&application_id)?;
        let model_stream = stream_ids::model_stream_id(&model_version)?;
        Ok(Self {
            application_stream,
            model_stream,
            session_id,
            application_id,
            model_version,
            precision,
            recall,
            sample_count,
            timestamp,
        })
    }
}

impl FScoreCommand for RecordApplicationFScore {
    fn precision(&self) -> Precision {
        self.precision
    }

    fn recall(&self) -> Recall {
        self.recall
    }
}

impl CommandLogic for RecordApplicationFScore {
    type State = MetricsState;
    type Event = DomainEvent;

    fn apply(&self, mut state: Self::State, event: &Self::Event) -> Self::State {
        state.apply(event);
        state
    }

    fn handle(&self, _state: Self::State) -> Result<NewEvents<Self::Event>, CommandError> {
        let mut events = Vec::new();

        let f_score = self.calculate_f_score()?;

        // Emit to application stream
        events.push(event_builders::build_application_f_score_calculated_event(
            self.application_stream.clone(),
            self.session_id.clone(),
            self.application_id.clone(),
            self.model_version.clone(),
            f_score,
            self.precision,
            self.recall,
            self.sample_count,
            self.timestamp,
        ));

        // Also emit to model stream for cross-application analysis
        events.push(event_builders::build_f_score_calculated_event(
            self.model_stream.clone(),
            self.session_id.clone(),
            self.model_version.clone(),
            f_score,
            self.precision,
            self.recall,
            self.sample_count,
            self.timestamp,
        ));

        Ok(events.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        llm::LlmProvider,
        test_data::{self, f_scores, numeric},
        types::ModelId,
    };
    use eventcore::RetryPolicy;
    use eventcore_memory::InMemoryEventStore;
    use eventcore_types::EventStore;

    #[tokio::test]
    async fn test_record_model_f_score() {
        let session_id = SessionId::generate();
        let model_version = ModelVersion {
            provider: LlmProvider::OpenAI,
            model_id: ModelId::try_new(test_data::model_ids::GPT_4_TURBO.to_string()).unwrap(),
        };
        let precision = Precision::try_new(f_scores::MEDIUM_PRECISION).unwrap();
        let recall = Recall::try_new(f_scores::MEDIUM_RECALL).unwrap();

        let command = RecordModelFScore::new(
            session_id,
            model_version.clone(),
            precision,
            recall,
            SampleCount::try_new(numeric::BATCH_SIZE_100 as u64).unwrap(),
            Timestamp::now(),
        )
        .unwrap();
        let store = InMemoryEventStore::new();

        let result = eventcore::execute(&store, command.clone(), RetryPolicy::default()).await;
        assert!(result.is_ok());

        let events = store
            .read_stream::<DomainEvent>(command.model_stream.clone())
            .await
            .unwrap();

        assert_eq!(events.len(), 1);
        let event = events.iter().next().unwrap();
        match event {
            DomainEvent::FScoreCalculated {
                model_version: event_model,
                f_score,
                precision: event_precision,
                recall: event_recall,
                sample_count,
                ..
            } => {
                assert_eq!(event_model, &model_version);
                assert_eq!(event_precision, &Some(precision));
                assert_eq!(event_recall, &Some(recall));
                assert_eq!(
                    sample_count,
                    &SampleCount::try_new(numeric::BATCH_SIZE_100 as u64).unwrap()
                );

                let expected_f_score = 2.0 * (f_scores::MEDIUM_PRECISION * f_scores::MEDIUM_RECALL)
                    / (f_scores::MEDIUM_PRECISION + f_scores::MEDIUM_RECALL);
                assert!((f_score.into_inner() - expected_f_score).abs() < 1e-10);
            }
            _ => panic!("Expected FScoreCalculated event"),
        }
    }

    #[tokio::test]
    async fn test_record_application_f_score() {
        let session_id = SessionId::generate();
        let application_id =
            ApplicationId::try_new(test_data::application_ids::MY_APP.to_string()).unwrap();
        let model_version = ModelVersion {
            provider: LlmProvider::OpenAI,
            model_id: ModelId::try_new(test_data::model_ids::GPT_4_TURBO.to_string()).unwrap(),
        };
        let precision = Precision::try_new(f_scores::HIGH_PRECISION).unwrap();
        let recall = Recall::try_new(f_scores::GOOD_F_SCORE).unwrap();

        let command = RecordApplicationFScore::new(
            session_id,
            application_id.clone(),
            model_version.clone(),
            precision,
            recall,
            SampleCount::try_new(f_scores::MEDIUM_SAMPLE).unwrap(),
            Timestamp::now(),
        )
        .unwrap();
        let store = InMemoryEventStore::new();

        let result = eventcore::execute(&store, command.clone(), RetryPolicy::default()).await;
        assert!(result.is_ok());

        let app_events = store
            .read_stream::<DomainEvent>(command.application_stream.clone())
            .await
            .unwrap();

        assert_eq!(app_events.len(), 1);
        match app_events.iter().next().unwrap() {
            DomainEvent::ApplicationFScoreCalculated {
                application_id: event_app_id,
                model_version: event_model,
                f_score,
                sample_count,
                ..
            } => {
                assert_eq!(*event_app_id, application_id);
                assert_eq!(*event_model, model_version);
                assert_eq!(
                    *sample_count,
                    SampleCount::try_new(f_scores::MEDIUM_SAMPLE).unwrap()
                );

                let expected_f_score = 2.0 * (f_scores::HIGH_PRECISION * f_scores::GOOD_F_SCORE)
                    / (f_scores::HIGH_PRECISION + f_scores::GOOD_F_SCORE);
                assert!((f_score.into_inner() - expected_f_score).abs() < 1e-10);
            }
            _ => panic!("Expected ApplicationFScoreCalculated event"),
        }

        let model_events = store
            .read_stream::<DomainEvent>(command.model_stream.clone())
            .await
            .unwrap();

        assert_eq!(model_events.len(), 1);
        assert!(matches!(
            model_events.iter().next().unwrap(),
            DomainEvent::FScoreCalculated { .. }
        ));
    }

    #[tokio::test]
    async fn test_metrics_state_tracking() {
        let mut state = MetricsState::default();
        let model_version = ModelVersion {
            provider: LlmProvider::OpenAI,
            model_id: ModelId::try_new(test_data::model_ids::GPT_4_TURBO.to_string()).unwrap(),
        };
        let precision = Precision::try_new(f_scores::HIGH_PRECISION).unwrap();
        let recall = Recall::try_new(f_scores::MEDIUM_PRECISION).unwrap();
        let f_score = FScore::from_precision_recall(precision, recall).unwrap();

        let event = DomainEvent::FScoreCalculated {
            stream_id: StreamId::try_new("metrics:test".to_string()).unwrap(),
            session_id: SessionId::generate(),
            model_version: model_version.clone(),
            f_score,
            precision: Some(precision),
            recall: Some(recall),
            sample_count: SampleCount::try_new(numeric::TOKENS_150 as u64).unwrap(),
            calculated_at: Timestamp::now(),
        };

        state.apply(&event);

        let latest = state.latest_f_score(&model_version);
        assert!(latest.is_some());
        let latest = latest.unwrap();
        assert_eq!(latest.f_score(), f_score);
        assert_eq!(latest.precision(), Some(precision));
        assert_eq!(latest.recall(), Some(recall));
        assert_eq!(
            latest.sample_count(),
            SampleCount::try_new(numeric::TOKENS_150 as u64).unwrap()
        );

        let history = state.f_score_history(&model_version);
        assert!(history.is_some());
        assert_eq!(history.unwrap().len(), 1);
    }
}
