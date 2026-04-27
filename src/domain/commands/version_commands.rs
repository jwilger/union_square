//! EventCore commands for version tracking
//!
//! These commands implement the EventCore CommandLogic trait to provide
//! multi-stream event sourcing for version tracking operations.

use eventcore::{CommandError, CommandLogic, NewEvents, StreamId};
use eventcore_macros::Command;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::domain::{
    events::DomainEvent,
    llm::ModelVersion,
    metrics::Timestamp,
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

/// Build a canonical stream ID for a model version.
fn version_stream_id(model_version: &ModelVersion) -> Result<StreamId, CommandError> {
    StreamId::try_new(format!("version:{}", model_version.to_version_string()))
        .map_err(|e| CommandError::ValidationError(format!("Invalid version stream ID: {e}")))
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
    pub fn new(session_id: SessionId, model_version: ModelVersion) -> Result<Self, CommandError> {
        let version_stream = version_stream_id(&model_version)?;
        Ok(Self {
            version_stream,
            session_id,
            model_version,
        })
    }
}

impl CommandLogic for RecordVersionUsage {
    type State = VersionState;
    type Event = DomainEvent;

    fn apply(&self, mut state: Self::State, event: &Self::Event) -> Self::State {
        state.apply(event);
        state
    }

    fn handle(&self, state: Self::State) -> Result<NewEvents<Self::Event>, CommandError> {
        let mut events = Vec::new();

        // Check if this is the first time we've seen this version
        let is_first_seen = !state.tracked_versions.contains_key(&self.model_version);

        if is_first_seen {
            events.push(DomainEvent::VersionFirstSeen {
                stream_id: self.version_stream.clone(),
                model_version: self.model_version.clone(),
                session_id: self.session_id.clone(),
                first_seen_at: Timestamp::now(),
            });
        }

        // Always record usage
        events.push(DomainEvent::VersionUsageRecorded {
            stream_id: self.version_stream.clone(),
            model_version: self.model_version.clone(),
            session_id: self.session_id.clone(),
            recorded_at: Timestamp::now(),
        });

        Ok(events.into())
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
    ) -> Result<Self, CommandError> {
        let from_stream = version_stream_id(&from_version)?;
        let to_stream = version_stream_id(&to_version)?;
        Ok(Self {
            from_stream,
            to_stream,
            session_id,
            from_version,
            to_version,
            reason,
        })
    }
}

impl CommandLogic for RecordVersionChange {
    type State = VersionState;
    type Event = DomainEvent;

    fn apply(&self, mut state: Self::State, event: &Self::Event) -> Self::State {
        state.apply(event);
        state
    }

    fn handle(&self, _state: Self::State) -> Result<NewEvents<Self::Event>, CommandError> {
        let mut events = Vec::new();
        let change_type = self.from_version.compare(&self.to_version);
        let change_id = VersionChangeId::generate();

        events.push(DomainEvent::VersionChanged {
            stream_id: self.from_stream.clone(),
            change_id: change_id.clone(),
            session_id: self.session_id.clone(),
            from_version: self.from_version.clone(),
            to_version: self.to_version.clone(),
            change_type,
            reason: self.reason.clone(),
            changed_at: Timestamp::now(),
        });
        events.push(DomainEvent::VersionChanged {
            stream_id: self.to_stream.clone(),
            change_id,
            session_id: self.session_id.clone(),
            from_version: self.from_version.clone(),
            to_version: self.to_version.clone(),
            change_type: self.from_version.compare(&self.to_version),
            reason: self.reason.clone(),
            changed_at: Timestamp::now(),
        });

        Ok(events.into())
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
    pub fn new(
        model_version: ModelVersion,
        reason: Option<ChangeReason>,
    ) -> Result<Self, CommandError> {
        let version_stream = version_stream_id(&model_version)?;
        Ok(Self {
            version_stream,
            model_version,
            reason,
        })
    }
}

impl CommandLogic for DeactivateVersion {
    type State = VersionState;
    type Event = DomainEvent;

    fn apply(&self, mut state: Self::State, event: &Self::Event) -> Self::State {
        state.apply(event);
        state
    }

    fn handle(&self, state: Self::State) -> Result<NewEvents<Self::Event>, CommandError> {
        // Check if version exists and is active
        let version_exists = state
            .tracked_versions
            .get(&self.model_version)
            .map(|v| v.is_active)
            .unwrap_or(false);

        if !version_exists {
            return Err(CommandError::BusinessRuleViolation(Box::new(
                std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Version not found or already deactivated",
                ),
            )));
        }

        let events = vec![DomainEvent::VersionDeactivated {
            stream_id: self.version_stream.clone(),
            model_version: self.model_version.clone(),
            reason: self.reason.clone(),
            deactivated_at: Timestamp::now(),
        }];

        Ok(events.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::types::ModelId;
    use eventcore::RetryPolicy;
    use eventcore_memory::InMemoryEventStore;
    use eventcore_types::EventStore;

    #[tokio::test]
    async fn test_record_version_usage_first_seen() {
        let session_id = SessionId::generate();
        let model_version = ModelVersion {
            provider: crate::domain::llm::LlmProvider::OpenAI,
            model_id: ModelId::try_new("gpt-4-turbo".to_string()).unwrap(),
        };

        let command = RecordVersionUsage::new(session_id, model_version.clone()).unwrap();
        let store = InMemoryEventStore::new();

        let result = eventcore::execute(&store, command.clone(), RetryPolicy::default()).await;
        assert!(result.is_ok());

        let events = store
            .read_stream::<DomainEvent>(command.version_stream.clone())
            .await
            .unwrap();

        assert_eq!(events.len(), 2); // VersionFirstSeen and VersionUsageRecorded
        assert!(matches!(
            events.iter().next().unwrap(),
            DomainEvent::VersionFirstSeen { .. }
        ));
        assert!(matches!(
            events.iter().nth(1).unwrap(),
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

        let store = InMemoryEventStore::new();

        let first_command =
            RecordVersionUsage::new(session_id.clone(), model_version.clone()).unwrap();
        eventcore::execute(&store, first_command.clone(), RetryPolicy::default())
            .await
            .unwrap();

        let second_command = RecordVersionUsage::new(session_id, model_version.clone()).unwrap();
        let result = eventcore::execute(&store, second_command, RetryPolicy::default()).await;
        assert!(result.is_ok());

        let events = store
            .read_stream::<DomainEvent>(first_command.version_stream.clone())
            .await
            .unwrap();

        assert_eq!(events.len(), 3);
        assert!(matches!(
            events.iter().nth(2).unwrap(),
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
        )
        .unwrap();
        let store = InMemoryEventStore::new();

        let result = eventcore::execute(&store, command.clone(), RetryPolicy::default()).await;
        assert!(result.is_ok());

        let from_events = store
            .read_stream::<DomainEvent>(command.from_stream.clone())
            .await
            .unwrap();

        assert_eq!(from_events.len(), 1);
        match from_events.iter().next().unwrap() {
            DomainEvent::VersionChanged {
                session_id: event_session_id,
                from_version: event_from,
                to_version: event_to,
                reason,
                change_type,
                ..
            } => {
                assert_eq!(*event_session_id, session_id);
                assert_eq!(*event_from, from_version);
                assert_eq!(*event_to, to_version);
                assert_eq!(
                    reason.as_ref().map(|r| r.as_ref()),
                    Some("Performance upgrade")
                );
                assert_eq!(*change_type, from_version.compare(&to_version));
            }
            _ => panic!("Expected VersionChanged event"),
        }

        let to_events = store
            .read_stream::<DomainEvent>(command.to_stream.clone())
            .await
            .unwrap();

        assert_eq!(to_events.len(), 1);
    }

    #[tokio::test]
    async fn test_deactivate_version() {
        let session_id = SessionId::generate();
        let model_version = ModelVersion {
            provider: crate::domain::llm::LlmProvider::OpenAI,
            model_id: ModelId::try_new("gpt-4-deprecated".to_string()).unwrap(),
        };

        let store = InMemoryEventStore::new();

        let record_command = RecordVersionUsage::new(session_id, model_version.clone()).unwrap();
        eventcore::execute(&store, record_command.clone(), RetryPolicy::default())
            .await
            .unwrap();

        let deactivate_command = DeactivateVersion::new(
            model_version.clone(),
            Some(ChangeReason::try_new("Model deprecated".to_string()).unwrap()),
        )
        .unwrap();
        let result =
            eventcore::execute(&store, deactivate_command.clone(), RetryPolicy::default()).await;
        assert!(result.is_ok());

        let events = store
            .read_stream::<DomainEvent>(record_command.version_stream.clone())
            .await
            .unwrap();

        assert_eq!(events.len(), 3);
        assert!(matches!(
            events.iter().nth(2).unwrap(),
            DomainEvent::VersionDeactivated { .. }
        ));
    }

    #[tokio::test]
    async fn test_deactivate_nonexistent_version() {
        let model_version = ModelVersion {
            provider: crate::domain::llm::LlmProvider::OpenAI,
            model_id: ModelId::try_new("gpt-4-nonexistent".to_string()).unwrap(),
        };

        let command = DeactivateVersion::new(model_version, None).unwrap();
        let store = InMemoryEventStore::new();

        let result = eventcore::execute(&store, command, RetryPolicy::default()).await;
        assert!(result.is_err());
    }
}
