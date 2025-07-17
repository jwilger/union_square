//! EventCore commands for version tracking
//!
//! These commands implement the EventCore CommandLogic trait to provide
//! multi-stream event sourcing for version tracking operations.

use async_trait::async_trait;
use eventcore::{
    CommandError, CommandLogic, CommandResult, CommandStreams, ReadStreams, StoredEvent, StreamId,
    StreamResolver, StreamWrite,
};
use serde::{Deserialize, Serialize};

use crate::domain::{
    entity::EntityId,
    events::DomainEvent,
    llm::ModelVersion,
    session::SessionId,
    version::{TrackedVersion, VersionChangeId},
};

/// State for version tracking
#[derive(Debug, Default, Clone)]
pub struct VersionState {
    pub tracked_versions: Vec<TrackedVersion>,
}

impl VersionState {
    /// Apply an event to update the state
    pub fn apply(&mut self, event: &DomainEvent) {
        match event {
            DomainEvent::VersionFirstSeen { model_version, .. } => {
                let tracked = TrackedVersion::new(model_version.clone());
                self.tracked_versions.push(tracked);
            }
            DomainEvent::VersionUsageRecorded { model_version, .. } => {
                // Find and update the tracked version
                if let Some(tracked) = self
                    .tracked_versions
                    .iter_mut()
                    .find(|v| v.version == *model_version)
                {
                    tracked.record_usage();
                }
            }
            DomainEvent::VersionDeactivated { model_version, .. } => {
                if let Some(tracked) = self
                    .tracked_versions
                    .iter_mut()
                    .find(|v| v.version == *model_version)
                {
                    tracked.deactivate();
                }
            }
            _ => {} // Ignore other events
        }
    }
}

/// Command to record the usage of a model version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordVersionUsage {
    pub session_id: SessionId,
    pub model_version: ModelVersion,
}

impl RecordVersionUsage {
    pub fn new(session_id: SessionId, model_version: ModelVersion) -> Self {
        Self {
            session_id,
            model_version,
        }
    }
}

/// Phantom type for RecordVersionUsage stream access
pub struct RecordVersionUsageStreams;

impl CommandStreams for RecordVersionUsage {
    type StreamSet = RecordVersionUsageStreams;

    fn read_streams(&self) -> Vec<StreamId> {
        vec![StreamId::try_new(
            EntityId::version(&self.model_version.to_version_string()).into_inner(),
        )
        .expect("Valid stream ID")]
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
        let stream_id = StreamId::try_new(
            EntityId::version(&self.model_version.to_version_string()).into_inner(),
        )
        .map_err(|_| CommandError::ValidationFailed("Invalid stream ID".into()))?;

        // Check if this is the first time we've seen this version
        let is_first_seen = !state
            .tracked_versions
            .iter()
            .any(|v| v.version == self.model_version);

        if is_first_seen {
            events.push(StreamWrite::new(
                &_read_streams,
                stream_id.clone(),
                DomainEvent::VersionFirstSeen {
                    model_version: self.model_version.clone(),
                    session_id: self.session_id.clone(),
                    first_seen_at: chrono::Utc::now(),
                },
            )?);
        }

        // Always record usage
        events.push(StreamWrite::new(
            &_read_streams,
            stream_id,
            DomainEvent::VersionUsageRecorded {
                model_version: self.model_version.clone(),
                session_id: self.session_id.clone(),
                recorded_at: chrono::Utc::now(),
            },
        )?);

        Ok(events)
    }
}

/// Command to record a version change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordVersionChange {
    pub session_id: SessionId,
    pub from_version: ModelVersion,
    pub to_version: ModelVersion,
    pub reason: Option<String>,
}

impl RecordVersionChange {
    pub fn new(
        session_id: SessionId,
        from_version: ModelVersion,
        to_version: ModelVersion,
        reason: Option<String>,
    ) -> Self {
        Self {
            session_id,
            from_version,
            to_version,
            reason,
        }
    }
}

/// Phantom type for RecordVersionChange stream access
pub struct RecordVersionChangeStreams;

impl CommandStreams for RecordVersionChange {
    type StreamSet = RecordVersionChangeStreams;

    fn read_streams(&self) -> Vec<StreamId> {
        vec![
            StreamId::try_new(
                EntityId::version(&self.from_version.to_version_string()).into_inner(),
            )
            .expect("Valid stream ID"),
            StreamId::try_new(EntityId::version(&self.to_version.to_version_string()).into_inner())
                .expect("Valid stream ID"),
        ]
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
        let change_type = self.from_version.compare(&self.to_version);
        let change_id = VersionChangeId::generate();
        let stream_id = StreamId::try_new(
            EntityId::version(&self.from_version.to_version_string()).into_inner(),
        )
        .map_err(|_| CommandError::ValidationFailed("Invalid stream ID".into()))?;

        Ok(vec![StreamWrite::new(
            &_read_streams,
            stream_id,
            DomainEvent::VersionChanged {
                change_id,
                session_id: self.session_id.clone(),
                from_version: self.from_version.clone(),
                to_version: self.to_version.clone(),
                change_type,
                reason: self.reason.clone(),
                changed_at: chrono::Utc::now(),
            },
        )?])
    }
}

/// Command to deactivate a version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeactivateVersion {
    pub model_version: ModelVersion,
    pub reason: Option<String>,
}

impl DeactivateVersion {
    pub fn new(model_version: ModelVersion, reason: Option<String>) -> Self {
        Self {
            model_version,
            reason,
        }
    }
}

/// Phantom type for DeactivateVersion stream access
pub struct DeactivateVersionStreams;

impl CommandStreams for DeactivateVersion {
    type StreamSet = DeactivateVersionStreams;

    fn read_streams(&self) -> Vec<StreamId> {
        vec![StreamId::try_new(
            EntityId::version(&self.model_version.to_version_string()).into_inner(),
        )
        .expect("Valid stream ID")]
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
            .iter()
            .any(|v| v.version == self.model_version && v.is_active);

        if !version_exists {
            return Err(CommandError::ValidationFailed(
                "Version not found or already deactivated".into(),
            ));
        }

        let stream_id = StreamId::try_new(
            EntityId::version(&self.model_version.to_version_string()).into_inner(),
        )
        .map_err(|_| CommandError::ValidationFailed("Invalid stream ID".into()))?;

        Ok(vec![StreamWrite::new(
            &_read_streams,
            stream_id,
            DomainEvent::VersionDeactivated {
                model_version: self.model_version.clone(),
                reason: self.reason.clone(),
                deactivated_at: chrono::Utc::now(),
            },
        )?])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eventcore::{CommandExecutor, EventStore, ExecutionOptions, ReadOptions};
    use eventcore_memory::InMemoryEventStore;

    #[tokio::test]
    async fn test_record_version_usage_first_seen() {
        let session_id = SessionId::generate();
        let model_version = ModelVersion {
            provider: crate::domain::llm::LlmProvider::OpenAI,
            model_id: "gpt-4-turbo".to_string(),
        };

        let command = RecordVersionUsage::new(session_id, model_version.clone());
        let event_store = InMemoryEventStore::new();
        let executor = CommandExecutor::new(event_store);

        // Execute command with the executor
        let result = executor.execute(command, ExecutionOptions::default()).await;
        assert!(result.is_ok());

        // Read events from the version stream
        let stream_id =
            StreamId::try_new(EntityId::version(&model_version.to_version_string()).into_inner())
                .unwrap();
        let stream_data = executor
            .event_store()
            .read_streams(&[stream_id], &ReadOptions::default())
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
            model_id: "gpt-4-turbo".to_string(),
        };

        let event_store = InMemoryEventStore::new();
        let executor = CommandExecutor::new(event_store);

        // First, record the version to make it exist
        let first_command = RecordVersionUsage::new(session_id.clone(), model_version.clone());
        executor
            .execute(first_command, ExecutionOptions::default())
            .await
            .unwrap();

        // Now record usage again
        let second_command = RecordVersionUsage::new(session_id, model_version.clone());
        let result = executor
            .execute(second_command, ExecutionOptions::default())
            .await;
        assert!(result.is_ok());

        // Read all events from the version stream
        let stream_id =
            StreamId::try_new(EntityId::version(&model_version.to_version_string()).into_inner())
                .unwrap();
        let stream_data = executor
            .event_store()
            .read_streams(&[stream_id], &ReadOptions::default())
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
            model_id: "gpt-3.5-turbo".to_string(),
        };
        let to_version = ModelVersion {
            provider: crate::domain::llm::LlmProvider::OpenAI,
            model_id: "gpt-4-turbo".to_string(),
        };

        let command = RecordVersionChange::new(
            session_id,
            from_version,
            to_version,
            Some("Performance upgrade".to_string()),
        );
        let event_store = InMemoryEventStore::new();
        let executor = CommandExecutor::new(event_store);

        let result = executor.execute(command, ExecutionOptions::default()).await;
        assert!(result.is_ok());

        // The event is written to a change-specific stream
        // We'll need to extract the change_id from the result to read it
        // For now, just verify the command executes successfully
    }

    #[tokio::test]
    async fn test_deactivate_version() {
        let session_id = SessionId::generate();
        let model_version = ModelVersion {
            provider: crate::domain::llm::LlmProvider::OpenAI,
            model_id: "gpt-4-deprecated".to_string(),
        };

        let event_store = InMemoryEventStore::new();
        let executor = CommandExecutor::new(event_store);

        // First, record the version to make it exist
        let record_command = RecordVersionUsage::new(session_id, model_version.clone());
        executor
            .execute(record_command, ExecutionOptions::default())
            .await
            .unwrap();

        // Now deactivate it
        let deactivate_command =
            DeactivateVersion::new(model_version.clone(), Some("Model deprecated".to_string()));
        let result = executor
            .execute(deactivate_command, ExecutionOptions::default())
            .await;
        assert!(result.is_ok());

        // Read events from the version stream
        let stream_id =
            StreamId::try_new(EntityId::version(&model_version.to_version_string()).into_inner())
                .unwrap();
        let stream_data = executor
            .event_store()
            .read_streams(&[stream_id], &ReadOptions::default())
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
            model_id: "gpt-4-nonexistent".to_string(),
        };

        let command = DeactivateVersion::new(model_version, None);
        let event_store = InMemoryEventStore::new();
        let executor = CommandExecutor::new(event_store);

        let result = executor.execute(command, ExecutionOptions::default()).await;
        assert!(result.is_err());
    }
}
