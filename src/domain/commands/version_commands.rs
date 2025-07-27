//! EventCore commands for version tracking
//!
//! These commands implement the EventCore CommandLogic trait to provide
//! multi-stream event sourcing for version tracking operations.

use async_trait::async_trait;
use eventcore::{
    emit, require, CommandLogic, CommandResult, ReadStreams, StoredEvent, StreamId, StreamResolver,
    StreamWrite,
};
use eventcore_macros::Command;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::domain::{
    events::DomainEvent,
    llm::ModelVersion,
    session::SessionId,
    types::ChangeReason,
    version::{TrackedVersion, VersionChangeId},
};

/// State for version tracking
#[derive(Debug, Default, Clone)]
pub struct VersionState {
    pub tracked_versions: HashMap<ModelVersion, TrackedVersion>,
}

impl VersionState {
    /// Apply an event to update the state
    pub fn apply(&mut self, event: &DomainEvent) {
        match event {
            DomainEvent::VersionFirstSeen { model_version, .. } => {
                let tracked = TrackedVersion::new(model_version.clone());
                self.tracked_versions.insert(model_version.clone(), tracked);
            }
            DomainEvent::VersionUsageRecorded { model_version, .. } => {
                // Find and update the tracked version
                if let Some(tracked) = self.tracked_versions.get_mut(model_version) {
                    tracked.record_usage();
                }
            }
            DomainEvent::VersionDeactivated { model_version, .. } => {
                if let Some(tracked) = self.tracked_versions.get_mut(model_version) {
                    tracked.deactivate();
                }
            }
            // VersionChanged events record transitions between versions but don't
            // modify the state of tracked versions themselves. They are stored
            // in both the from and to version streams as historical records.
            _ => {} // Ignore other events
        }
    }
}

/// Command to record the usage of a model version
#[derive(Debug, Clone, Serialize, Deserialize, Command)]
pub struct RecordVersionUsage {
    #[stream]
    version_stream: StreamId,
    pub session_id: SessionId,
    pub model_version: ModelVersion,
}

impl RecordVersionUsage {
    pub fn new(session_id: SessionId, model_version: ModelVersion) -> Self {
        let version_stream = Self::version_stream_id(&model_version);
        Self {
            version_stream,
            session_id,
            model_version,
        }
    }

    fn version_stream_id(model_version: &ModelVersion) -> StreamId {
        StreamId::try_new(format!("version:{}", model_version.to_version_string()))
            .expect("Valid stream ID")
    }
}

#[async_trait]
impl CommandLogic for RecordVersionUsage {
    type State = VersionState;
    type Event = DomainEvent;

    fn apply(&self, state: &mut Self::State, event: &StoredEvent<Self::Event>) {
        state.apply(&event.payload);
    }

    async fn handle(
        &self,
        _read_streams: ReadStreams<Self::StreamSet>,
        state: Self::State,
        _stream_resolver: &mut StreamResolver,
    ) -> CommandResult<Vec<StreamWrite<Self::StreamSet, Self::Event>>> {
        let mut events = Vec::new();

        // Check if this is the first time we've seen this version
        let is_first_seen = !state.tracked_versions.contains_key(&self.model_version);

        if is_first_seen {
            emit!(
                events,
                &_read_streams,
                self.version_stream.clone(),
                DomainEvent::VersionFirstSeen {
                    model_version: self.model_version.clone(),
                    session_id: self.session_id.clone(),
                    first_seen_at: chrono::Utc::now(),
                }
            );
        }

        // Always record usage
        emit!(
            events,
            &_read_streams,
            self.version_stream.clone(),
            DomainEvent::VersionUsageRecorded {
                model_version: self.model_version.clone(),
                session_id: self.session_id.clone(),
                recorded_at: chrono::Utc::now(),
            }
        );

        Ok(events)
    }
}

/// Command to record a version change
#[derive(Debug, Clone, Serialize, Deserialize, Command)]
pub struct RecordVersionChange {
    #[stream]
    from_stream: StreamId,
    #[stream]
    to_stream: StreamId,
    pub session_id: SessionId,
    pub from_version: ModelVersion,
    pub to_version: ModelVersion,
    pub reason: Option<ChangeReason>,
}

impl RecordVersionChange {
    pub fn new(
        session_id: SessionId,
        from_version: ModelVersion,
        to_version: ModelVersion,
        reason: Option<ChangeReason>,
    ) -> Self {
        let from_stream = Self::version_stream_id(&from_version);
        let to_stream = Self::version_stream_id(&to_version);
        Self {
            from_stream,
            to_stream,
            session_id,
            from_version,
            to_version,
            reason,
        }
    }

    fn version_stream_id(model_version: &ModelVersion) -> StreamId {
        StreamId::try_new(format!("version:{}", model_version.to_version_string()))
            .expect("Valid stream ID")
    }
}

#[async_trait]
impl CommandLogic for RecordVersionChange {
    type State = VersionState;
    type Event = DomainEvent;

    fn apply(&self, state: &mut Self::State, event: &StoredEvent<Self::Event>) {
        state.apply(&event.payload);
    }

    async fn handle(
        &self,
        _read_streams: ReadStreams<Self::StreamSet>,
        _state: Self::State,
        _stream_resolver: &mut StreamResolver,
    ) -> CommandResult<Vec<StreamWrite<Self::StreamSet, Self::Event>>> {
        let mut events = Vec::new();
        let change_type = self.from_version.compare(&self.to_version);
        let change_id = VersionChangeId::generate();

        let event = DomainEvent::VersionChanged {
            change_id: change_id.clone(),
            session_id: self.session_id.clone(),
            from_version: self.from_version.clone(),
            to_version: self.to_version.clone(),
            change_type,
            reason: self.reason.clone(),
            changed_at: chrono::Utc::now(),
        };

        // Write to both streams
        emit!(
            events,
            &_read_streams,
            self.from_stream.clone(),
            event.clone()
        );
        emit!(events, &_read_streams, self.to_stream.clone(), event);

        Ok(events)
    }
}

/// Command to deactivate a version
#[derive(Debug, Clone, Serialize, Deserialize, Command)]
pub struct DeactivateVersion {
    #[stream]
    version_stream: StreamId,
    pub model_version: ModelVersion,
    pub reason: Option<ChangeReason>,
}

impl DeactivateVersion {
    pub fn new(model_version: ModelVersion, reason: Option<ChangeReason>) -> Self {
        let version_stream = Self::version_stream_id(&model_version);
        Self {
            version_stream,
            model_version,
            reason,
        }
    }

    fn version_stream_id(model_version: &ModelVersion) -> StreamId {
        StreamId::try_new(format!("version:{}", model_version.to_version_string()))
            .expect("Valid stream ID")
    }
}

#[async_trait]
impl CommandLogic for DeactivateVersion {
    type State = VersionState;
    type Event = DomainEvent;

    fn apply(&self, state: &mut Self::State, event: &StoredEvent<Self::Event>) {
        state.apply(&event.payload);
    }

    async fn handle(
        &self,
        _read_streams: ReadStreams<Self::StreamSet>,
        state: Self::State,
        _stream_resolver: &mut StreamResolver,
    ) -> CommandResult<Vec<StreamWrite<Self::StreamSet, Self::Event>>> {
        // Check if version exists and is active
        let version_exists = state
            .tracked_versions
            .get(&self.model_version)
            .map(|v| v.is_active)
            .unwrap_or(false);

        require!(version_exists, "Version not found or already deactivated");

        let mut events = vec![];

        emit!(
            events,
            &_read_streams,
            self.version_stream.clone(),
            DomainEvent::VersionDeactivated {
                model_version: self.model_version.clone(),
                reason: self.reason.clone(),
                deactivated_at: chrono::Utc::now(),
            }
        );

        Ok(events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::types::ModelId;
    use eventcore::{CommandExecutor, EventStore, ExecutionOptions, ReadOptions};
    use eventcore_memory::InMemoryEventStore;

    #[tokio::test]
    async fn test_record_version_usage_first_seen() {
        let session_id = SessionId::generate();
        let model_version = ModelVersion {
            provider: crate::domain::llm::LlmProvider::OpenAI,
            model_id: ModelId::try_new("gpt-4-turbo".to_string()).unwrap(),
        };

        let command = RecordVersionUsage::new(session_id, model_version.clone());
        let event_store = InMemoryEventStore::new();
        let executor = CommandExecutor::new(event_store);

        // Execute command with the executor
        let result = executor
            .execute(command.clone(), ExecutionOptions::default())
            .await;
        assert!(result.is_ok());

        // Read events from the version stream
        let stream_data = executor
            .event_store()
            .read_streams(&[command.version_stream], &ReadOptions::default())
            .await
            .unwrap();
        let events = stream_data.events;

        assert_eq!(events.len(), 2); // VersionFirstSeen and VersionUsageRecorded
        assert!(matches!(
            events[0].payload,
            DomainEvent::VersionFirstSeen { .. }
        ));
        assert!(matches!(
            events[1].payload,
            DomainEvent::VersionUsageRecorded { .. }
        ));
    }

    #[tokio::test]
    async fn test_record_version_usage_existing() {
        let session_id = SessionId::generate();
        let model_version = ModelVersion {
            provider: crate::domain::llm::LlmProvider::OpenAI,
            model_id: ModelId::try_new("gpt-4-turbo".to_string()).unwrap(),
        };

        let event_store = InMemoryEventStore::new();
        let executor = CommandExecutor::new(event_store);

        // First, record the version to make it exist
        let first_command = RecordVersionUsage::new(session_id.clone(), model_version.clone());
        executor
            .execute(first_command.clone(), ExecutionOptions::default())
            .await
            .unwrap();

        // Now record usage again
        let second_command = RecordVersionUsage::new(session_id, model_version.clone());
        let result = executor
            .execute(second_command, ExecutionOptions::default())
            .await;
        assert!(result.is_ok());

        // Read all events from the version stream
        let stream_data = executor
            .event_store()
            .read_streams(&[first_command.version_stream], &ReadOptions::default())
            .await
            .unwrap();
        let events = stream_data.events;

        // Should have 3 events total: VersionFirstSeen, VersionUsageRecorded, VersionUsageRecorded
        assert_eq!(events.len(), 3);
        assert!(matches!(
            events[2].payload,
            DomainEvent::VersionUsageRecorded { .. }
        ));
    }

    #[tokio::test]
    async fn test_record_version_change() {
        let session_id = SessionId::generate();
        let from_version = ModelVersion {
            provider: crate::domain::llm::LlmProvider::OpenAI,
            model_id: ModelId::try_new("gpt-3.5-turbo".to_string()).unwrap(),
        };
        let to_version = ModelVersion {
            provider: crate::domain::llm::LlmProvider::OpenAI,
            model_id: ModelId::try_new("gpt-4-turbo".to_string()).unwrap(),
        };

        let command = RecordVersionChange::new(
            session_id.clone(),
            from_version.clone(),
            to_version.clone(),
            Some(ChangeReason::try_new("Performance upgrade".to_string()).unwrap()),
        );
        let event_store = InMemoryEventStore::new();
        let executor = CommandExecutor::new(event_store);

        let result = executor
            .execute(command.clone(), ExecutionOptions::default())
            .await;
        assert!(result.is_ok());

        // Read events from both version streams
        // Check from_version stream
        let from_stream_data = executor
            .event_store()
            .read_streams(&[command.from_stream.clone()], &ReadOptions::default())
            .await
            .unwrap();
        let from_events = from_stream_data.events;

        assert_eq!(from_events.len(), 1);
        match &from_events[0].payload {
            DomainEvent::VersionChanged {
                session_id: event_session_id,
                from_version: event_from,
                to_version: event_to,
                reason,
                change_type,
                ..
            } => {
                assert_eq!(event_session_id, &session_id);
                assert_eq!(event_from, &from_version);
                assert_eq!(event_to, &to_version);
                assert_eq!(
                    reason.as_ref().map(|r| r.as_ref()),
                    Some("Performance upgrade")
                );
                assert_eq!(change_type, &from_version.compare(&to_version));
            }
            _ => panic!("Expected VersionChanged event"),
        }

        // Check to_version stream
        let to_stream_data = executor
            .event_store()
            .read_streams(&[command.to_stream], &ReadOptions::default())
            .await
            .unwrap();
        let to_events = to_stream_data.events;

        assert_eq!(to_events.len(), 1);
        // Verify it's the same event (both streams should have the same event)
        assert_eq!(from_events[0].payload, to_events[0].payload);
    }

    #[tokio::test]
    async fn test_deactivate_version() {
        let session_id = SessionId::generate();
        let model_version = ModelVersion {
            provider: crate::domain::llm::LlmProvider::OpenAI,
            model_id: ModelId::try_new("gpt-4-deprecated".to_string()).unwrap(),
        };

        let event_store = InMemoryEventStore::new();
        let executor = CommandExecutor::new(event_store);

        // First, record the version to make it exist
        let record_command = RecordVersionUsage::new(session_id, model_version.clone());
        executor
            .execute(record_command.clone(), ExecutionOptions::default())
            .await
            .unwrap();

        // Now deactivate it
        let deactivate_command = DeactivateVersion::new(
            model_version.clone(),
            Some(ChangeReason::try_new("Model deprecated".to_string()).unwrap()),
        );
        let result = executor
            .execute(deactivate_command.clone(), ExecutionOptions::default())
            .await;
        assert!(result.is_ok());

        // Read events from the version stream
        let stream_data = executor
            .event_store()
            .read_streams(&[record_command.version_stream], &ReadOptions::default())
            .await
            .unwrap();
        let events = stream_data.events;

        // Should have 3 events: VersionFirstSeen, VersionUsageRecorded, VersionDeactivated
        assert_eq!(events.len(), 3);
        assert!(matches!(
            events[2].payload,
            DomainEvent::VersionDeactivated { .. }
        ));
    }

    #[tokio::test]
    async fn test_deactivate_nonexistent_version() {
        let model_version = ModelVersion {
            provider: crate::domain::llm::LlmProvider::OpenAI,
            model_id: ModelId::try_new("gpt-4-nonexistent".to_string()).unwrap(),
        };

        let command = DeactivateVersion::new(model_version, None);
        let event_store = InMemoryEventStore::new();
        let executor = CommandExecutor::new(event_store);

        let result = executor.execute(command, ExecutionOptions::default()).await;
        assert!(result.is_err());
    }
}
