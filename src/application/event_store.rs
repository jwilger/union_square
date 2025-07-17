//! EventCore integration for storing and retrieving events
//!
//! This module provides the event store abstraction for Union Square,
//! handling all EventCore interactions for audit logging and event sourcing.

use async_trait::async_trait;
use eventcore::StreamId;

use crate::{
    domain::{DomainEvent, SessionId, VersionChangeEvent},
    error::Result,
};

/// Union Square specific event store interface
///
/// This provides domain-specific methods while using EventCore's EventStore internally.
/// We maintain this interface for domain-specific operations while leveraging EventCore's
/// multi-stream capabilities.
#[async_trait]
pub trait UnionSquareEventStore: Send + Sync {
    /// Store a domain event
    async fn store_event(&self, event: DomainEvent) -> Result<()>;

    /// Get all events for a session
    async fn get_session_events(&self, session_id: &SessionId) -> Result<Vec<DomainEvent>>;

    /// Get version change events for a session
    async fn get_version_changes(&self, session_id: &SessionId) -> Result<Vec<VersionChangeEvent>>;
}

/// Convert SessionId to EventCore StreamId
impl From<&SessionId> for StreamId {
    fn from(session_id: &SessionId) -> Self {
        StreamId::try_new(format!("session-{session_id}"))
            .expect("Session ID should always be valid for stream ID")
    }
}

/// In-memory event store for testing
#[cfg(test)]
pub mod test_support {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    /// Simple in-memory event store for Union Square domain events
    ///
    /// This is a temporary implementation that provides a domain-specific interface
    /// while we work on proper EventCore integration.
    pub struct InMemoryEventStore {
        events: Arc<RwLock<HashMap<String, Vec<DomainEvent>>>>,
    }

    impl Default for InMemoryEventStore {
        fn default() -> Self {
            Self::new()
        }
    }

    impl InMemoryEventStore {
        pub fn new() -> Self {
            Self {
                events: Arc::new(RwLock::new(HashMap::new())),
            }
        }
    }

    #[async_trait]
    impl UnionSquareEventStore for InMemoryEventStore {
        async fn store_event(&self, event: DomainEvent) -> Result<()> {
            let session_id = event.session_id().clone();

            let mut events = self.events.write().await;
            events
                .entry(session_id.to_string())
                .or_insert_with(Vec::new)
                .push(event);

            Ok(())
        }

        async fn get_session_events(&self, session_id: &SessionId) -> Result<Vec<DomainEvent>> {
            let events = self.events.read().await;

            Ok(events
                .get(&session_id.to_string())
                .cloned()
                .unwrap_or_default())
        }

        async fn get_version_changes(
            &self,
            session_id: &SessionId,
        ) -> Result<Vec<VersionChangeEvent>> {
            let events = self.get_session_events(session_id).await?;

            let version_changes = events
                .into_iter()
                .filter_map(|event| match event {
                    DomainEvent::VersionChanged {
                        session_id,
                        from_version,
                        to_version,
                        reason,
                        occurred_at,
                    } => Some(VersionChangeEvent {
                        session_id,
                        from_version,
                        to_version,
                        reason,
                        occurred_at,
                    }),
                    _ => None,
                })
                .collect();

            Ok(version_changes)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        ApiVersion, ExtendedModelVersion, LlmProvider, ModelName, ModelVersionString,
        ProviderVersionInfo, SessionMetadata, VersionChangeReason,
    };
    use chrono::Utc;

    #[tokio::test]
    async fn test_store_and_retrieve_events() {
        let store = test_support::InMemoryEventStore::new();
        let session_id = SessionId::generate();

        // Store a session started event
        let event1 = DomainEvent::SessionStarted {
            session_id: session_id.clone(),
            user_id: None,
            metadata: SessionMetadata {
                application_name: Some("test".to_string()),
                environment: None,
                user_agent: None,
                ip_address: None,
                tags: vec![],
            },
            occurred_at: Utc::now(),
        };

        store.store_event(event1.clone()).await.unwrap();

        // Store a version changed event
        let version = ExtendedModelVersion::new(
            LlmProvider::OpenAI,
            ModelName::try_new("gpt-4").unwrap(),
            ProviderVersionInfo::OpenAI {
                model_version: Some(ModelVersionString::try_new("1106-preview").unwrap()),
                api_version: ApiVersion::try_new("2023-12-01").unwrap(),
                system_fingerprint: None,
            },
        );

        let event2 = DomainEvent::VersionChanged {
            session_id: session_id.clone(),
            from_version: None,
            to_version: version,
            reason: VersionChangeReason::InitialDetection,
            occurred_at: Utc::now(),
        };

        store.store_event(event2.clone()).await.unwrap();

        // Retrieve all events
        let events = store.get_session_events(&session_id).await.unwrap();
        assert_eq!(events.len(), 2);

        // Retrieve version changes
        let version_changes = store.get_version_changes(&session_id).await.unwrap();
        assert_eq!(version_changes.len(), 1);
        assert_eq!(
            version_changes[0].reason,
            VersionChangeReason::InitialDetection
        );
    }

    #[tokio::test]
    async fn test_get_empty_session_events() {
        let store = test_support::InMemoryEventStore::new();
        let session_id = SessionId::generate();

        let events = store.get_session_events(&session_id).await.unwrap();
        assert!(events.is_empty());

        let version_changes = store.get_version_changes(&session_id).await.unwrap();
        assert!(version_changes.is_empty());
    }
}
