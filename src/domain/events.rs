//! EventCore event definitions for Union Square
//!
//! This module defines all domain events that are stored
//! in the event store using EventCore.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::{
    llm::{ModelVersion, RequestId, ResponseMetadata},
    session::{SessionId, SessionStatus},
    user::{EmailAddress, UserId},
    version::{VersionChangeId, VersionComparison},
};

/// All domain events in the Union Square system
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DomainEvent {
    // Session Events
    SessionStarted {
        session_id: SessionId,
        user_id: UserId,
        application_name: String,
        started_at: DateTime<Utc>,
    },
    SessionEnded {
        session_id: SessionId,
        ended_at: DateTime<Utc>,
        final_status: SessionStatus,
    },
    SessionTagged {
        session_id: SessionId,
        tag: String,
        tagged_at: DateTime<Utc>,
    },

    // LLM Request Events
    LlmRequestReceived {
        request_id: RequestId,
        session_id: SessionId,
        model_version: ModelVersion,
        prompt: String,
        parameters: serde_json::Value,
        received_at: DateTime<Utc>,
    },
    LlmRequestStarted {
        request_id: RequestId,
        started_at: DateTime<Utc>,
    },
    LlmResponseReceived {
        request_id: RequestId,
        response_text: String,
        metadata: ResponseMetadata,
        received_at: DateTime<Utc>,
    },
    LlmRequestFailed {
        request_id: RequestId,
        error_message: String,
        failed_at: DateTime<Utc>,
    },
    LlmRequestCancelled {
        request_id: RequestId,
        cancelled_at: DateTime<Utc>,
    },

    // Version Tracking Events
    VersionFirstSeen {
        model_version: ModelVersion,
        session_id: SessionId,
        first_seen_at: DateTime<Utc>,
    },
    VersionChanged {
        change_id: VersionChangeId,
        session_id: SessionId,
        from_version: ModelVersion,
        to_version: ModelVersion,
        change_type: VersionComparison,
        reason: Option<String>,
        changed_at: DateTime<Utc>,
    },
    VersionDeactivated {
        model_version: ModelVersion,
        reason: Option<String>,
        deactivated_at: DateTime<Utc>,
    },

    // User Events
    UserCreated {
        user_id: UserId,
        email: EmailAddress,
        display_name: Option<String>,
        created_at: DateTime<Utc>,
    },
    UserActivated {
        user_id: UserId,
        activated_at: DateTime<Utc>,
    },
    UserDeactivated {
        user_id: UserId,
        reason: Option<String>,
        deactivated_at: DateTime<Utc>,
    },
}

impl DomainEvent {
    /// Get the timestamp of when this event occurred
    pub fn occurred_at(&self) -> DateTime<Utc> {
        match self {
            DomainEvent::SessionStarted { started_at, .. } => *started_at,
            DomainEvent::SessionEnded { ended_at, .. } => *ended_at,
            DomainEvent::SessionTagged { tagged_at, .. } => *tagged_at,
            DomainEvent::LlmRequestReceived { received_at, .. } => *received_at,
            DomainEvent::LlmRequestStarted { started_at, .. } => *started_at,
            DomainEvent::LlmResponseReceived { received_at, .. } => *received_at,
            DomainEvent::LlmRequestFailed { failed_at, .. } => *failed_at,
            DomainEvent::LlmRequestCancelled { cancelled_at, .. } => *cancelled_at,
            DomainEvent::VersionFirstSeen { first_seen_at, .. } => *first_seen_at,
            DomainEvent::VersionChanged { changed_at, .. } => *changed_at,
            DomainEvent::VersionDeactivated { deactivated_at, .. } => *deactivated_at,
            DomainEvent::UserCreated { created_at, .. } => *created_at,
            DomainEvent::UserActivated { activated_at, .. } => *activated_at,
            DomainEvent::UserDeactivated { deactivated_at, .. } => *deactivated_at,
        }
    }

    /// Get the primary entity ID associated with this event
    pub fn entity_id(&self) -> String {
        match self {
            DomainEvent::SessionStarted { session_id, .. } => {
                format!("session:{}", session_id.clone().into_inner())
            }
            DomainEvent::SessionEnded { session_id, .. } => {
                format!("session:{}", session_id.clone().into_inner())
            }
            DomainEvent::SessionTagged { session_id, .. } => {
                format!("session:{}", session_id.clone().into_inner())
            }
            DomainEvent::LlmRequestReceived { request_id, .. } => {
                format!("request:{}", request_id.clone().into_inner())
            }
            DomainEvent::LlmRequestStarted { request_id, .. } => {
                format!("request:{}", request_id.clone().into_inner())
            }
            DomainEvent::LlmResponseReceived { request_id, .. } => {
                format!("request:{}", request_id.clone().into_inner())
            }
            DomainEvent::LlmRequestFailed { request_id, .. } => {
                format!("request:{}", request_id.clone().into_inner())
            }
            DomainEvent::LlmRequestCancelled { request_id, .. } => {
                format!("request:{}", request_id.clone().into_inner())
            }
            DomainEvent::VersionFirstSeen { model_version, .. } => {
                format!("version:{}", model_version.to_version_string())
            }
            DomainEvent::VersionChanged { change_id, .. } => {
                format!("version_change:{}", change_id.clone().into_inner())
            }
            DomainEvent::VersionDeactivated { model_version, .. } => {
                format!("version:{}", model_version.to_version_string())
            }
            DomainEvent::UserCreated { user_id, .. } => {
                format!("user:{}", user_id.clone().into_inner())
            }
            DomainEvent::UserActivated { user_id, .. } => {
                format!("user:{}", user_id.clone().into_inner())
            }
            DomainEvent::UserDeactivated { user_id, .. } => {
                format!("user:{}", user_id.clone().into_inner())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::llm::LlmProvider;

    #[test]
    fn test_event_timestamp_extraction() {
        let now = Utc::now();
        let session_id = SessionId::generate();
        let user_id = UserId::generate();

        let event = DomainEvent::SessionStarted {
            session_id,
            user_id,
            application_name: "test-app".to_string(),
            started_at: now,
        };

        assert_eq!(event.occurred_at(), now);
    }

    #[test]
    fn test_event_entity_id() {
        let session_id = SessionId::generate();
        let request_id = RequestId::generate();
        let user_id = UserId::generate();

        let session_event = DomainEvent::SessionStarted {
            session_id: session_id.clone(),
            user_id,
            application_name: "test-app".to_string(),
            started_at: Utc::now(),
        };
        assert!(session_event.entity_id().starts_with("session:"));

        let request_event = DomainEvent::LlmRequestStarted {
            request_id: request_id.clone(),
            started_at: Utc::now(),
        };
        assert!(request_event.entity_id().starts_with("request:"));
    }

    #[test]
    fn test_version_events() {
        let session_id = SessionId::generate();
        let version = ModelVersion {
            provider: LlmProvider::OpenAI,
            model_id: "gpt-4-turbo-2024-01".to_string(),
        };

        let event = DomainEvent::VersionFirstSeen {
            model_version: version.clone(),
            session_id,
            first_seen_at: Utc::now(),
        };

        assert!(event.entity_id().contains("version:"));
        assert!(event.entity_id().contains("openai/gpt-4-turbo-2024-01"));
    }
}
