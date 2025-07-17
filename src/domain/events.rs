//! EventCore event definitions for Union Square
//!
//! This module defines all domain events that are published through EventCore
//! for audit logging, event sourcing, and system state tracking.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::{
    ExtendedModelVersion, LlmRequest, LlmResponse, ResponseMetadata, SessionId, SessionMetadata,
    SessionStatus, UserId, VersionChangeReason,
};

#[cfg(test)]
use crate::domain::LlmProvider;

/// Aggregate ID type for session-related events
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionAggregateId(pub SessionId);

impl From<SessionId> for SessionAggregateId {
    fn from(id: SessionId) -> Self {
        Self(id)
    }
}

impl std::fmt::Display for SessionAggregateId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "session:{}", self.0)
    }
}

/// All domain events in the Union Square system
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DomainEvent {
    /// A new session has been started
    SessionStarted {
        session_id: SessionId,
        user_id: Option<UserId>,
        metadata: SessionMetadata,
        occurred_at: DateTime<Utc>,
    },

    /// An LLM request has been recorded
    RequestRecorded {
        session_id: SessionId,
        request: LlmRequest,
        occurred_at: DateTime<Utc>,
    },

    /// An LLM response has been recorded
    ResponseRecorded {
        session_id: SessionId,
        response: LlmResponse,
        metadata: ResponseMetadata,
        occurred_at: DateTime<Utc>,
    },

    /// Session status has changed
    SessionStatusChanged {
        session_id: SessionId,
        from_status: SessionStatus,
        to_status: SessionStatus,
        occurred_at: DateTime<Utc>,
    },

    /// Model version has been detected or changed
    VersionChanged {
        session_id: SessionId,
        from_version: Option<ExtendedModelVersion>,
        to_version: ExtendedModelVersion,
        reason: VersionChangeReason,
        occurred_at: DateTime<Utc>,
    },

    /// An error occurred during session processing
    SessionError {
        session_id: SessionId,
        error_message: String,
        error_type: String,
        occurred_at: DateTime<Utc>,
    },
}

impl DomainEvent {
    /// Get the session ID associated with this event
    pub fn session_id(&self) -> &SessionId {
        match self {
            Self::SessionStarted { session_id, .. }
            | Self::RequestRecorded { session_id, .. }
            | Self::ResponseRecorded { session_id, .. }
            | Self::SessionStatusChanged { session_id, .. }
            | Self::VersionChanged { session_id, .. }
            | Self::SessionError { session_id, .. } => session_id,
        }
    }

    /// Get the timestamp when this event occurred
    pub fn occurred_at(&self) -> DateTime<Utc> {
        match self {
            Self::SessionStarted { occurred_at, .. }
            | Self::RequestRecorded { occurred_at, .. }
            | Self::ResponseRecorded { occurred_at, .. }
            | Self::SessionStatusChanged { occurred_at, .. }
            | Self::VersionChanged { occurred_at, .. }
            | Self::SessionError { occurred_at, .. } => *occurred_at,
        }
    }

    /// Get a human-readable description of the event
    pub fn description(&self) -> String {
        match self {
            Self::SessionStarted { .. } => "Session started".to_string(),
            Self::RequestRecorded { request, .. } => {
                format!(
                    "Request recorded for model {}",
                    request.model_version.model_name
                )
            }
            Self::ResponseRecorded { .. } => "Response recorded".to_string(),
            Self::SessionStatusChanged {
                from_status,
                to_status,
                ..
            } => {
                format!("Session status changed from {from_status:?} to {to_status:?}")
            }
            Self::VersionChanged {
                from_version,
                to_version,
                reason,
                ..
            } => match from_version {
                Some(from) => format!(
                    "Model version changed from {} to {} ({})",
                    from.display_string(),
                    to_version.display_string(),
                    match reason {
                        VersionChangeReason::InitialDetection => "initial detection",
                        VersionChangeReason::ProviderUpdate => "provider update",
                        VersionChangeReason::UserSelection => "user selection",
                        VersionChangeReason::Failover => "failover",
                        VersionChangeReason::Experiment { .. } => "experiment",
                        VersionChangeReason::Other(_) => "other",
                    }
                ),
                None => format!(
                    "Initial model version detected: {}",
                    to_version.display_string()
                ),
            },
            Self::SessionError { error_type, .. } => {
                format!("Session error: {error_type}")
            }
        }
    }
}

/// Get the event type name for EventCore
impl DomainEvent {
    pub fn event_type(&self) -> &'static str {
        match self {
            DomainEvent::SessionStarted { .. } => "session_started",
            DomainEvent::RequestRecorded { .. } => "request_recorded",
            DomainEvent::ResponseRecorded { .. } => "response_recorded",
            DomainEvent::SessionStatusChanged { .. } => "session_status_changed",
            DomainEvent::VersionChanged { .. } => "version_changed",
            DomainEvent::SessionError { .. } => "session_error",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{ApiVersion, ModelName, ModelVersionString, ProviderVersionInfo};

    #[test]
    fn test_session_started_event() {
        let session_id = SessionId::generate();
        let event = DomainEvent::SessionStarted {
            session_id: session_id.clone(),
            user_id: None,
            metadata: SessionMetadata {
                application_name: Some("test-app".to_string()),
                environment: Some("test".to_string()),
                user_agent: None,
                ip_address: None,
                tags: vec![],
            },
            occurred_at: Utc::now(),
        };

        assert_eq!(event.session_id(), &session_id);
        assert_eq!(event.description(), "Session started");
    }

    #[test]
    fn test_version_changed_event() {
        let session_id = SessionId::generate();
        let to_version = ExtendedModelVersion::new(
            LlmProvider::OpenAI,
            ModelName::try_new("gpt-4").unwrap(),
            ProviderVersionInfo::OpenAI {
                model_version: Some(ModelVersionString::try_new("1106-preview").unwrap()),
                api_version: ApiVersion::try_new("2023-12-01").unwrap(),
                system_fingerprint: None,
            },
        );

        let event = DomainEvent::VersionChanged {
            session_id: session_id.clone(),
            from_version: None,
            to_version: to_version.clone(),
            reason: VersionChangeReason::InitialDetection,
            occurred_at: Utc::now(),
        };

        assert_eq!(event.session_id(), &session_id);
        assert!(event
            .description()
            .contains("Initial model version detected"));
        assert!(event.description().contains("gpt-4/1106-preview"));
    }

    #[test]
    fn test_event_type_names() {
        let session_id = SessionId::generate();
        let event = DomainEvent::SessionStarted {
            session_id: session_id.clone(),
            user_id: None,
            metadata: SessionMetadata {
                application_name: Some("test-app".to_string()),
                environment: None,
                user_agent: None,
                ip_address: None,
                tags: vec![],
            },
            occurred_at: Utc::now(),
        };

        assert_eq!(event.event_type(), "session_started");

        let version_event = DomainEvent::VersionChanged {
            session_id,
            from_version: None,
            to_version: ExtendedModelVersion::new(
                LlmProvider::OpenAI,
                ModelName::try_new("gpt-4").unwrap(),
                ProviderVersionInfo::OpenAI {
                    model_version: Some(ModelVersionString::try_new("1106-preview").unwrap()),
                    api_version: ApiVersion::try_new("2023-12-01").unwrap(),
                    system_fingerprint: None,
                },
            ),
            reason: VersionChangeReason::InitialDetection,
            occurred_at: Utc::now(),
        };

        assert_eq!(version_event.event_type(), "version_changed");
    }

    #[test]
    fn test_version_change_with_previous_version() {
        let session_id = SessionId::generate();
        let from_version = ExtendedModelVersion::new(
            LlmProvider::OpenAI,
            ModelName::try_new("gpt-4").unwrap(),
            ProviderVersionInfo::OpenAI {
                model_version: Some(ModelVersionString::try_new("0613").unwrap()),
                api_version: ApiVersion::try_new("2023-11-01").unwrap(),
                system_fingerprint: None,
            },
        );

        let to_version = ExtendedModelVersion::new(
            LlmProvider::OpenAI,
            ModelName::try_new("gpt-4").unwrap(),
            ProviderVersionInfo::OpenAI {
                model_version: Some(ModelVersionString::try_new("1106-preview").unwrap()),
                api_version: ApiVersion::try_new("2023-12-01").unwrap(),
                system_fingerprint: None,
            },
        );

        let event = DomainEvent::VersionChanged {
            session_id,
            from_version: Some(from_version),
            to_version,
            reason: VersionChangeReason::ProviderUpdate,
            occurred_at: Utc::now(),
        };

        let description = event.description();
        assert!(description.contains("Model version changed from"));
        assert!(description.contains("gpt-4/0613"));
        assert!(description.contains("gpt-4/1106-preview"));
        assert!(description.contains("provider update"));
    }
}
