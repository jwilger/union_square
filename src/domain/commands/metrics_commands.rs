//! EventCore commands for F-score and metrics tracking
//!
//! These commands implement the EventCore CommandLogic trait to provide
//! multi-stream event sourcing for F-score tracking and analytics operations.

use async_trait::async_trait;
use eventcore::{
    emit, CommandLogic, CommandResult, ReadStreams, StoredEvent, StreamId, StreamResolver,
    StreamWrite,
};
use eventcore_macros::Command;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::domain::{
    events::DomainEvent,
    llm::ModelVersion,
    metrics::{FScore, FScoreDataPoint, Precision, Recall},
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
                let data_point = FScoreDataPoint {
                    timestamp: *calculated_at,
                    f_score: *f_score,
                    precision: *precision,
                    recall: *recall,
                    sample_count: *sample_count,
                    confidence_level: None,
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
                let data_point = FScoreDataPoint {
                    timestamp: *calculated_at,
                    f_score: *f_score,
                    precision: *precision,
                    recall: *recall,
                    sample_count: *sample_count,
                    confidence_level: None,
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
    pub sample_count: u64,
}

impl RecordModelFScore {
    pub fn new(
        session_id: SessionId,
        model_version: ModelVersion,
        precision: Precision,
        recall: Recall,
        sample_count: u64,
    ) -> Self {
        let model_stream = Self::model_stream_id(&model_version);
        Self {
            model_stream,
            session_id,
            model_version,
            precision,
            recall,
            sample_count,
        }
    }

    fn model_stream_id(model_version: &ModelVersion) -> StreamId {
        StreamId::try_new(format!(
            "metrics:model:{}",
            model_version.to_version_string()
        ))
        .expect("Valid stream ID")
    }
}

#[async_trait]
impl CommandLogic for RecordModelFScore {
    type State = MetricsState;
    type Event = DomainEvent;

    fn apply(&self, state: &mut Self::State, event: &StoredEvent<Self::Event>) {
        state.apply(&event.payload);
    }

    async fn handle(
        &self,
        read_streams: ReadStreams<Self::StreamSet>,
        _state: Self::State,
        _stream_resolver: &mut StreamResolver,
    ) -> CommandResult<Vec<StreamWrite<Self::StreamSet, Self::Event>>> {
        let mut events = Vec::new();

        // Calculate F-score from precision and recall
        // This should never fail since precision and recall are validated at construction
        let f_score = FScore::from_precision_recall(self.precision, self.recall)
            .expect("F-score calculation should succeed with valid precision and recall");

        emit!(
            events,
            &read_streams,
            self.model_stream.clone(),
            DomainEvent::FScoreCalculated {
                session_id: self.session_id.clone(),
                model_version: self.model_version.clone(),
                f_score,
                precision: Some(self.precision),
                recall: Some(self.recall),
                sample_count: self.sample_count,
                calculated_at: chrono::Utc::now(),
            }
        );

        Ok(events)
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
    pub sample_count: u64,
}

impl RecordApplicationFScore {
    pub fn new(
        session_id: SessionId,
        application_id: ApplicationId,
        model_version: ModelVersion,
        precision: Precision,
        recall: Recall,
        sample_count: u64,
    ) -> Self {
        let application_stream = Self::application_stream_id(&application_id);
        let model_stream = Self::model_stream_id(&model_version);
        Self {
            application_stream,
            model_stream,
            session_id,
            application_id,
            model_version,
            precision,
            recall,
            sample_count,
        }
    }

    fn application_stream_id(application_id: &ApplicationId) -> StreamId {
        StreamId::try_new(format!("metrics:app:{}", application_id.as_ref()))
            .expect("Valid stream ID")
    }

    fn model_stream_id(model_version: &ModelVersion) -> StreamId {
        StreamId::try_new(format!(
            "metrics:model:{}",
            model_version.to_version_string()
        ))
        .expect("Valid stream ID")
    }
}

#[async_trait]
impl CommandLogic for RecordApplicationFScore {
    type State = MetricsState;
    type Event = DomainEvent;

    fn apply(&self, state: &mut Self::State, event: &StoredEvent<Self::Event>) {
        state.apply(&event.payload);
    }

    async fn handle(
        &self,
        read_streams: ReadStreams<Self::StreamSet>,
        _state: Self::State,
        _stream_resolver: &mut StreamResolver,
    ) -> CommandResult<Vec<StreamWrite<Self::StreamSet, Self::Event>>> {
        let mut events = Vec::new();

        // Calculate F-score from precision and recall
        // This should never fail since precision and recall are validated at construction
        let f_score = FScore::from_precision_recall(self.precision, self.recall)
            .expect("F-score calculation should succeed with valid precision and recall");

        // Emit to application stream
        emit!(
            events,
            &read_streams,
            self.application_stream.clone(),
            DomainEvent::ApplicationFScoreCalculated {
                session_id: self.session_id.clone(),
                application_id: self.application_id.clone(),
                model_version: self.model_version.clone(),
                f_score,
                precision: Some(self.precision),
                recall: Some(self.recall),
                sample_count: self.sample_count,
                calculated_at: chrono::Utc::now(),
            }
        );

        // Also emit to model stream for cross-application analysis
        emit!(
            events,
            &read_streams,
            self.model_stream.clone(),
            DomainEvent::FScoreCalculated {
                session_id: self.session_id.clone(),
                model_version: self.model_version.clone(),
                f_score,
                precision: Some(self.precision),
                recall: Some(self.recall),
                sample_count: self.sample_count,
                calculated_at: chrono::Utc::now(),
            }
        );

        Ok(events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{llm::LlmProvider, test_data, types::ModelId};
    use eventcore::{CommandExecutor, EventStore, ExecutionOptions, ReadOptions};
    use eventcore_memory::InMemoryEventStore;

    #[tokio::test]
    async fn test_record_model_f_score() {
        let session_id = SessionId::generate();
        let model_version = ModelVersion {
            provider: LlmProvider::OpenAI,
            model_id: ModelId::try_new(test_data::model_ids::GPT_4_TURBO.to_string()).unwrap(),
        };
        let precision = Precision::try_new(0.8).unwrap();
        let recall = Recall::try_new(0.7).unwrap();

        let command =
            RecordModelFScore::new(session_id, model_version.clone(), precision, recall, 100);
        let event_store = InMemoryEventStore::new();
        let executor = CommandExecutor::new(event_store);

        let result = executor
            .execute(command.clone(), ExecutionOptions::default())
            .await;
        assert!(result.is_ok());

        // Read events from the model stream
        let stream_data = executor
            .event_store()
            .read_streams(&[command.model_stream], &ReadOptions::default())
            .await
            .unwrap();
        let events = stream_data.events;

        assert_eq!(events.len(), 1);
        match &events[0].payload {
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
                assert_eq!(sample_count, &100);

                // Verify F-score calculation
                let expected_f_score = 2.0 * (0.8 * 0.7) / (0.8 + 0.7);
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
        let precision = Precision::try_new(0.9).unwrap();
        let recall = Recall::try_new(0.85).unwrap();

        let command = RecordApplicationFScore::new(
            session_id,
            application_id.clone(),
            model_version.clone(),
            precision,
            recall,
            200,
        );
        let event_store = InMemoryEventStore::new();
        let executor = CommandExecutor::new(event_store);

        let result = executor
            .execute(command.clone(), ExecutionOptions::default())
            .await;
        assert!(result.is_ok());

        // Read events from application stream
        let app_stream_data = executor
            .event_store()
            .read_streams(&[command.application_stream], &ReadOptions::default())
            .await
            .unwrap();
        let app_events = app_stream_data.events;

        assert_eq!(app_events.len(), 1);
        match &app_events[0].payload {
            DomainEvent::ApplicationFScoreCalculated {
                application_id: event_app_id,
                model_version: event_model,
                f_score,
                sample_count,
                ..
            } => {
                assert_eq!(event_app_id, &application_id);
                assert_eq!(event_model, &model_version);
                assert_eq!(sample_count, &200);

                // Verify F-score calculation
                let expected_f_score = 2.0 * (0.9 * 0.85) / (0.9 + 0.85);
                assert!((f_score.into_inner() - expected_f_score).abs() < 1e-10);
            }
            _ => panic!("Expected ApplicationFScoreCalculated event"),
        }

        // Read events from model stream (should also have an event)
        let model_stream_data = executor
            .event_store()
            .read_streams(&[command.model_stream], &ReadOptions::default())
            .await
            .unwrap();
        let model_events = model_stream_data.events;

        assert_eq!(model_events.len(), 1);
        assert!(matches!(
            model_events[0].payload,
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
        let f_score = FScore::try_new(0.85).unwrap();
        let precision = Precision::try_new(0.9).unwrap();
        let recall = Recall::try_new(0.8).unwrap();

        let event = DomainEvent::FScoreCalculated {
            session_id: SessionId::generate(),
            model_version: model_version.clone(),
            f_score,
            precision: Some(precision),
            recall: Some(recall),
            sample_count: 150,
            calculated_at: chrono::Utc::now(),
        };

        state.apply(&event);

        // Verify state was updated
        let latest = state.latest_f_score(&model_version);
        assert!(latest.is_some());
        let latest = latest.unwrap();
        assert_eq!(latest.f_score, f_score);
        assert_eq!(latest.precision, Some(precision));
        assert_eq!(latest.recall, Some(recall));
        assert_eq!(latest.sample_count, 150);

        // Verify history tracking
        let history = state.f_score_history(&model_version);
        assert!(history.is_some());
        assert_eq!(history.unwrap().len(), 1);
    }
}
