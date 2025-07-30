//! ID extraction utilities for multi-stream queries
//!
//! This module provides utilities to extract related IDs from domain events,
//! enabling efficient multi-stream queries and projections.

use crate::domain::{
    events::DomainEvent,
    llm::{ModelVersion, RequestId},
    session::ApplicationId,
    session::SessionId,
    user::UserId,
};
use std::collections::HashSet;

/// Extracts all session IDs referenced in an event
pub fn extract_session_ids(event: &DomainEvent) -> HashSet<SessionId> {
    let mut session_ids = HashSet::new();

    match event {
        // Events that contain session_id
        DomainEvent::SessionStarted { session_id, .. }
        | DomainEvent::SessionEnded { session_id, .. }
        | DomainEvent::SessionTagged { session_id, .. }
        | DomainEvent::LlmRequestReceived { session_id, .. }
        | DomainEvent::LlmRequestParsingFailed { session_id, .. }
        | DomainEvent::InvalidStateTransition { session_id, .. }
        | DomainEvent::AuditEventProcessingFailed { session_id, .. }
        | DomainEvent::VersionFirstSeen { session_id, .. }
        | DomainEvent::VersionChanged { session_id, .. }
        | DomainEvent::VersionUsageRecorded { session_id, .. }
        | DomainEvent::FScoreCalculated { session_id, .. }
        | DomainEvent::ApplicationFScoreCalculated { session_id, .. } => {
            session_ids.insert(session_id.clone());
        }

        // Events that do NOT contain session_id - explicitly list all
        DomainEvent::LlmRequestStarted { .. }
        | DomainEvent::LlmResponseReceived { .. }
        | DomainEvent::LlmRequestFailed { .. }
        | DomainEvent::LlmRequestCancelled { .. }
        | DomainEvent::VersionDeactivated { .. }
        | DomainEvent::UserCreated { .. }
        | DomainEvent::UserActivated { .. }
        | DomainEvent::UserDeactivated { .. } => {
            // No session_id in these events
        }
    }

    session_ids
}

/// Extracts all user IDs referenced in an event
pub fn extract_user_ids(event: &DomainEvent) -> HashSet<UserId> {
    let mut user_ids = HashSet::new();

    match event {
        // Events that contain user_id
        DomainEvent::SessionStarted { user_id, .. }
        | DomainEvent::UserCreated { user_id, .. }
        | DomainEvent::UserActivated { user_id, .. }
        | DomainEvent::UserDeactivated { user_id, .. } => {
            user_ids.insert(user_id.clone());
        }

        // Events that do NOT contain user_id - explicitly list all
        DomainEvent::SessionEnded { .. }
        | DomainEvent::SessionTagged { .. }
        | DomainEvent::LlmRequestReceived { .. }
        | DomainEvent::LlmRequestStarted { .. }
        | DomainEvent::LlmResponseReceived { .. }
        | DomainEvent::LlmRequestFailed { .. }
        | DomainEvent::LlmRequestCancelled { .. }
        | DomainEvent::LlmRequestParsingFailed { .. }
        | DomainEvent::InvalidStateTransition { .. }
        | DomainEvent::AuditEventProcessingFailed { .. }
        | DomainEvent::VersionFirstSeen { .. }
        | DomainEvent::VersionChanged { .. }
        | DomainEvent::VersionUsageRecorded { .. }
        | DomainEvent::VersionDeactivated { .. }
        | DomainEvent::FScoreCalculated { .. }
        | DomainEvent::ApplicationFScoreCalculated { .. } => {
            // No user_id in these events
        }
    }

    user_ids
}

/// Extracts all application IDs referenced in an event
pub fn extract_application_ids(event: &DomainEvent) -> HashSet<ApplicationId> {
    let mut app_ids = HashSet::new();

    match event {
        // Events that contain application_id
        DomainEvent::SessionStarted { application_id, .. }
        | DomainEvent::ApplicationFScoreCalculated { application_id, .. } => {
            app_ids.insert(application_id.clone());
        }

        // Events that do NOT contain application_id - explicitly list all
        DomainEvent::SessionEnded { .. }
        | DomainEvent::SessionTagged { .. }
        | DomainEvent::LlmRequestReceived { .. }
        | DomainEvent::LlmRequestStarted { .. }
        | DomainEvent::LlmResponseReceived { .. }
        | DomainEvent::LlmRequestFailed { .. }
        | DomainEvent::LlmRequestCancelled { .. }
        | DomainEvent::LlmRequestParsingFailed { .. }
        | DomainEvent::InvalidStateTransition { .. }
        | DomainEvent::AuditEventProcessingFailed { .. }
        | DomainEvent::VersionFirstSeen { .. }
        | DomainEvent::VersionChanged { .. }
        | DomainEvent::VersionUsageRecorded { .. }
        | DomainEvent::VersionDeactivated { .. }
        | DomainEvent::FScoreCalculated { .. }
        | DomainEvent::UserCreated { .. }
        | DomainEvent::UserActivated { .. }
        | DomainEvent::UserDeactivated { .. } => {
            // No application_id in these events
        }
    }

    app_ids
}

/// Extracts all model versions referenced in an event
pub fn extract_model_versions(event: &DomainEvent) -> HashSet<ModelVersion> {
    let mut versions = HashSet::new();

    match event {
        // Events that contain model_version
        DomainEvent::LlmRequestReceived { model_version, .. }
        | DomainEvent::VersionFirstSeen { model_version, .. }
        | DomainEvent::VersionUsageRecorded { model_version, .. }
        | DomainEvent::VersionDeactivated { model_version, .. }
        | DomainEvent::FScoreCalculated { model_version, .. }
        | DomainEvent::ApplicationFScoreCalculated { model_version, .. } => {
            versions.insert(model_version.clone());
        }

        // Special case: VersionChanged has two versions
        DomainEvent::VersionChanged {
            from_version,
            to_version,
            ..
        } => {
            versions.insert(from_version.clone());
            versions.insert(to_version.clone());
        }

        // Events that do NOT contain model_version - explicitly list all
        DomainEvent::SessionStarted { .. }
        | DomainEvent::SessionEnded { .. }
        | DomainEvent::SessionTagged { .. }
        | DomainEvent::LlmRequestStarted { .. }
        | DomainEvent::LlmResponseReceived { .. }
        | DomainEvent::LlmRequestFailed { .. }
        | DomainEvent::LlmRequestCancelled { .. }
        | DomainEvent::LlmRequestParsingFailed { .. }
        | DomainEvent::InvalidStateTransition { .. }
        | DomainEvent::AuditEventProcessingFailed { .. }
        | DomainEvent::UserCreated { .. }
        | DomainEvent::UserActivated { .. }
        | DomainEvent::UserDeactivated { .. } => {
            // No model_version in these events
        }
    }

    versions
}

/// Extracts all request IDs referenced in an event
pub fn extract_request_ids(event: &DomainEvent) -> HashSet<RequestId> {
    let mut request_ids = HashSet::new();

    match event {
        // Events that contain request_id
        DomainEvent::LlmRequestReceived { request_id, .. }
        | DomainEvent::LlmRequestStarted { request_id, .. }
        | DomainEvent::LlmResponseReceived { request_id, .. }
        | DomainEvent::LlmRequestFailed { request_id, .. }
        | DomainEvent::LlmRequestCancelled { request_id, .. }
        | DomainEvent::LlmRequestParsingFailed { request_id, .. }
        | DomainEvent::InvalidStateTransition { request_id, .. }
        | DomainEvent::AuditEventProcessingFailed { request_id, .. } => {
            request_ids.insert(request_id.clone());
        }

        // Events that do NOT contain request_id - explicitly list all
        DomainEvent::SessionStarted { .. }
        | DomainEvent::SessionEnded { .. }
        | DomainEvent::SessionTagged { .. }
        | DomainEvent::VersionFirstSeen { .. }
        | DomainEvent::VersionChanged { .. }
        | DomainEvent::VersionUsageRecorded { .. }
        | DomainEvent::VersionDeactivated { .. }
        | DomainEvent::FScoreCalculated { .. }
        | DomainEvent::ApplicationFScoreCalculated { .. }
        | DomainEvent::UserCreated { .. }
        | DomainEvent::UserActivated { .. }
        | DomainEvent::UserDeactivated { .. } => {
            // No request_id in these events
        }
    }

    request_ids
}

/// Represents all streams that an event might relate to
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelatedStreams {
    pub session_ids: HashSet<SessionId>,
    pub user_ids: HashSet<UserId>,
    pub application_ids: HashSet<ApplicationId>,
}

/// Extracts all stream IDs that an event relates to
pub fn extract_related_streams(event: &DomainEvent) -> RelatedStreams {
    RelatedStreams {
        session_ids: extract_session_ids(event),
        user_ids: extract_user_ids(event),
        application_ids: extract_application_ids(event),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        llm::LlmProvider,
        metrics::Timestamp,
        types::{LlmParameters, ModelId, Prompt},
        version::VersionChangeId,
    };

    #[test]
    fn test_extract_session_ids_from_session_started() {
        let session_id = SessionId::generate();
        let event = DomainEvent::SessionStarted {
            session_id: session_id.clone(),
            user_id: UserId::generate(),
            application_id: ApplicationId::try_new("test-app".to_string()).unwrap(),
            started_at: Timestamp::now(),
        };

        let session_ids = extract_session_ids(&event);
        assert_eq!(session_ids.len(), 1);
        assert!(session_ids.contains(&session_id));
    }

    #[test]
    fn test_extract_session_ids_from_llm_request() {
        let session_id = SessionId::generate();
        let event = DomainEvent::LlmRequestReceived {
            request_id: RequestId::generate(),
            session_id: session_id.clone(),
            model_version: ModelVersion {
                provider: LlmProvider::Anthropic,
                model_id: ModelId::try_new("claude-3".to_string()).unwrap(),
            },
            prompt: Prompt::try_new("test prompt".to_string()).unwrap(),
            parameters: LlmParameters::new(serde_json::json!({})),
            received_at: Timestamp::now(),
        };

        let session_ids = extract_session_ids(&event);
        assert_eq!(session_ids.len(), 1);
        assert!(session_ids.contains(&session_id));
    }

    #[test]
    fn test_extract_user_ids_from_session_started() {
        let user_id = UserId::generate();
        let event = DomainEvent::SessionStarted {
            session_id: SessionId::generate(),
            user_id: user_id.clone(),
            application_id: ApplicationId::try_new("test-app".to_string()).unwrap(),
            started_at: Timestamp::now(),
        };

        let user_ids = extract_user_ids(&event);
        assert_eq!(user_ids.len(), 1);
        assert!(user_ids.contains(&user_id));
    }

    #[test]
    fn test_extract_application_ids() {
        let app_id = ApplicationId::try_new("test-app".to_string()).unwrap();
        let event = DomainEvent::SessionStarted {
            session_id: SessionId::generate(),
            user_id: UserId::generate(),
            application_id: app_id.clone(),
            started_at: Timestamp::now(),
        };

        let app_ids = extract_application_ids(&event);
        assert_eq!(app_ids.len(), 1);
        assert!(app_ids.contains(&app_id));
    }

    #[test]
    fn test_extract_model_versions() {
        let model_version = ModelVersion {
            provider: LlmProvider::Anthropic,
            model_id: ModelId::try_new("claude-3".to_string()).unwrap(),
        };
        let event = DomainEvent::LlmRequestReceived {
            request_id: RequestId::generate(),
            session_id: SessionId::generate(),
            model_version: model_version.clone(),
            prompt: Prompt::try_new("test".to_string()).unwrap(),
            parameters: LlmParameters::new(serde_json::json!({})),
            received_at: Timestamp::now(),
        };

        let versions = extract_model_versions(&event);
        assert_eq!(versions.len(), 1);
        assert!(versions.contains(&model_version));
    }

    #[test]
    fn test_extract_request_ids() {
        let request_id = RequestId::generate();
        let event = DomainEvent::LlmRequestStarted {
            request_id: request_id.clone(),
            started_at: Timestamp::now(),
        };

        let request_ids = extract_request_ids(&event);
        assert_eq!(request_ids.len(), 1);
        assert!(request_ids.contains(&request_id));
    }

    #[test]
    fn test_extract_related_streams() {
        let session_id = SessionId::generate();
        let user_id = UserId::generate();
        let app_id = ApplicationId::try_new("test-app".to_string()).unwrap();

        let event = DomainEvent::SessionStarted {
            session_id: session_id.clone(),
            user_id: user_id.clone(),
            application_id: app_id.clone(),
            started_at: Timestamp::now(),
        };

        let related = extract_related_streams(&event);
        assert!(related.session_ids.contains(&session_id));
        assert!(related.user_ids.contains(&user_id));
        assert!(related.application_ids.contains(&app_id));
    }

    #[test]
    fn test_version_changed_extracts_multiple_versions() {
        let from_version = ModelVersion {
            provider: LlmProvider::Anthropic,
            model_id: ModelId::try_new("claude-2".to_string()).unwrap(),
        };
        let to_version = ModelVersion {
            provider: LlmProvider::Anthropic,
            model_id: ModelId::try_new("claude-3".to_string()).unwrap(),
        };

        let event = DomainEvent::VersionChanged {
            change_id: VersionChangeId::generate(),
            session_id: SessionId::generate(),
            from_version: from_version.clone(),
            to_version: to_version.clone(),
            change_type: crate::domain::version::VersionComparison::Changed {
                from_provider: from_version.provider.clone(),
                from_model_id: from_version.model_id.clone(),
                to_provider: to_version.provider.clone(),
                to_model_id: to_version.model_id.clone(),
            },
            reason: None,
            changed_at: Timestamp::now(),
        };

        let versions = extract_model_versions(&event);
        assert_eq!(versions.len(), 2);
        assert!(versions.contains(&from_version));
        assert!(versions.contains(&to_version));
    }

    #[test]
    fn test_exhaustive_matching_user_created() {
        // Test that UserCreated event doesn't have session_id
        let event = DomainEvent::UserCreated {
            user_id: UserId::generate(),
            email: crate::domain::user::EmailAddress::try_new("test@example.com".to_string())
                .unwrap(),
            display_name: None,
            created_at: Timestamp::now(),
        };

        let session_ids = extract_session_ids(&event);
        assert!(session_ids.is_empty());

        let user_ids = extract_user_ids(&event);
        assert_eq!(user_ids.len(), 1);
    }

    #[test]
    fn test_exhaustive_matching_llm_response() {
        // Test that LlmResponseReceived doesn't have session_id
        let event = DomainEvent::LlmResponseReceived {
            request_id: RequestId::generate(),
            response_text: crate::domain::types::ResponseText::try_new("response".to_string())
                .unwrap(),
            metadata: crate::domain::llm::ResponseMetadata::default(),
            received_at: Timestamp::now(),
        };

        let session_ids = extract_session_ids(&event);
        assert!(session_ids.is_empty());

        let request_ids = extract_request_ids(&event);
        assert_eq!(request_ids.len(), 1);
    }
}
