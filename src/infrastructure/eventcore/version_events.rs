//! EventCore-ready version tracking events and utilities
//!
//! This module provides event definitions and utilities that are ready
//! for EventCore integration when the full event sourcing is implemented.

use std::sync::Arc;
use tokio::sync::RwLock;

use crate::application::version_tracking::VersionTrackingService;
use crate::domain::{
    events::DomainEvent, llm::ModelVersion, session::SessionId,
    version::VersionChangeEvent as DomainVersionChangeEvent,
};
use crate::error::ApplicationError;

/// EventCore integration adapter for version tracking
///
/// This adapter bridges the gap between the application's version tracking
/// service and EventCore event storage. It converts domain operations into
/// events that can be stored in EventCore.
pub struct VersionEventAdapter {
    tracking_service: Arc<VersionTrackingService>,
    /// Temporary in-memory event storage until EventCore is fully integrated
    events: Arc<RwLock<Vec<DomainEvent>>>,
}

impl VersionEventAdapter {
    pub fn new(tracking_service: Arc<VersionTrackingService>) -> Self {
        Self {
            tracking_service,
            events: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Record version usage and emit appropriate events
    pub async fn record_version_usage(
        &self,
        session_id: &SessionId,
        version: &ModelVersion,
    ) -> Result<(), ApplicationError> {
        // First check if this is the first time we've seen this version
        let existing_versions = self.tracking_service.get_tracked_versions().await;
        let version_key = version.to_version_string();
        let is_first_seen = !existing_versions
            .iter()
            .any(|v| v.version.to_version_string() == version_key);

        // Record in the tracking service
        self.tracking_service
            .record_version_usage(session_id, version)
            .await?;

        // Emit event if this is the first time
        if is_first_seen {
            let event = DomainEvent::VersionFirstSeen {
                model_version: version.clone(),
                session_id: session_id.clone(),
                first_seen_at: chrono::Utc::now(),
            };
            self.events.write().await.push(event);
        }

        Ok(())
    }

    /// Record a version change and emit events
    pub async fn record_version_change(
        &self,
        session_id: &SessionId,
        from_version: ModelVersion,
        to_version: ModelVersion,
        reason: Option<String>,
    ) -> Result<DomainVersionChangeEvent, ApplicationError> {
        // Record in the tracking service
        let change_event = self
            .tracking_service
            .record_version_change(
                session_id,
                from_version.clone(),
                to_version.clone(),
                reason.clone(),
            )
            .await?;

        // Emit event
        let event = DomainEvent::VersionChanged {
            change_id: change_event.id.clone(),
            session_id: session_id.clone(),
            from_version,
            to_version,
            change_type: change_event.change_type.clone(),
            reason,
            changed_at: change_event.occurred_at,
        };
        self.events.write().await.push(event);

        Ok(change_event)
    }

    /// Deactivate a version and emit events
    pub async fn deactivate_version(
        &self,
        version: &ModelVersion,
        reason: Option<String>,
    ) -> Result<(), ApplicationError> {
        // Deactivate in the tracking service
        self.tracking_service.deactivate_version(version).await?;

        // Emit event
        let event = DomainEvent::VersionDeactivated {
            model_version: version.clone(),
            reason,
            deactivated_at: chrono::Utc::now(),
        };
        self.events.write().await.push(event);

        Ok(())
    }

    /// Get all emitted events (for testing)
    pub async fn get_events(&self) -> Vec<DomainEvent> {
        self.events.read().await.clone()
    }

    /// Clear all events (for testing)
    pub async fn clear_events(&self) {
        self.events.write().await.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::llm::LlmProvider;

    #[tokio::test]
    async fn test_version_first_seen_event() {
        let tracking_service = Arc::new(VersionTrackingService::new());
        let adapter = VersionEventAdapter::new(tracking_service);

        let session_id = SessionId::generate();
        let version = ModelVersion {
            provider: LlmProvider::OpenAI,
            model_name: "gpt-4".to_string(),
            version: Some("2024-01".to_string()),
            api_version: Some("v1".to_string()),
        };

        // First usage should emit VersionFirstSeen
        adapter
            .record_version_usage(&session_id, &version)
            .await
            .unwrap();
        let events = adapter.get_events().await;
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], DomainEvent::VersionFirstSeen { .. }));

        // Clear events
        adapter.clear_events().await;

        // Second usage should not emit event
        adapter
            .record_version_usage(&session_id, &version)
            .await
            .unwrap();
        let events = adapter.get_events().await;
        assert_eq!(events.len(), 0);
    }

    #[tokio::test]
    async fn test_version_change_event() {
        let tracking_service = Arc::new(VersionTrackingService::new());
        let adapter = VersionEventAdapter::new(tracking_service);

        let session_id = SessionId::generate();
        let v1 = ModelVersion {
            provider: LlmProvider::OpenAI,
            model_name: "gpt-3.5-turbo".to_string(),
            version: Some("2024-01".to_string()),
            api_version: Some("v1".to_string()),
        };
        let v2 = ModelVersion {
            provider: LlmProvider::OpenAI,
            model_name: "gpt-4".to_string(),
            version: Some("2024-01".to_string()),
            api_version: Some("v1".to_string()),
        };

        let change_event = adapter
            .record_version_change(&session_id, v1, v2, Some("Performance upgrade".to_string()))
            .await
            .unwrap();

        let events = adapter.get_events().await;
        assert_eq!(events.len(), 1);

        if let DomainEvent::VersionChanged {
            change_id, reason, ..
        } = &events[0]
        {
            assert_eq!(change_id, &change_event.id);
            assert_eq!(reason, &Some("Performance upgrade".to_string()));
        } else {
            panic!("Expected VersionChanged event");
        }
    }

    #[tokio::test]
    async fn test_deactivate_version_event() {
        let tracking_service = Arc::new(VersionTrackingService::new());
        let adapter = VersionEventAdapter::new(tracking_service.clone());

        let session_id = SessionId::generate();
        let version = ModelVersion {
            provider: LlmProvider::OpenAI,
            model_name: "gpt-4".to_string(),
            version: Some("2024-01".to_string()),
            api_version: Some("v1".to_string()),
        };

        // First record the version
        tracking_service
            .record_version_usage(&session_id, &version)
            .await
            .unwrap();

        // Then deactivate it
        adapter
            .deactivate_version(&version, Some("Deprecated".to_string()))
            .await
            .unwrap();

        let events = adapter.get_events().await;
        assert_eq!(events.len(), 1);

        if let DomainEvent::VersionDeactivated { reason, .. } = &events[0] {
            assert_eq!(reason, &Some("Deprecated".to_string()));
        } else {
            panic!("Expected VersionDeactivated event");
        }
    }
}
