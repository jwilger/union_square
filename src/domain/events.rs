//! EventCore event definitions for Union Square
//!
//! This module defines all domain events that are stored
//! in the event store using EventCore.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::{
    llm::{ModelVersion, RequestId, ResponseMetadata},
    session::{ApplicationId, SessionId, SessionStatus},
    types::{ChangeReason, ErrorMessage, LlmParameters, Prompt, ResponseText, Tag},
    user::{DisplayName, EmailAddress, UserId},
    version::{VersionChangeId, VersionComparison},
};

/// All domain events in the Union Square system
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DomainEvent {
    // Session Events
    SessionStarted {
        session_id: SessionId,
        user_id: UserId,
        application_id: ApplicationId,
        started_at: DateTime<Utc>,
    },
    SessionEnded {
        session_id: SessionId,
        ended_at: DateTime<Utc>,
        final_status: SessionStatus,
    },
    SessionTagged {
        session_id: SessionId,
        tag: Tag,
        tagged_at: DateTime<Utc>,
    },

    // LLM Request Events
    LlmRequestReceived {
        request_id: RequestId,
        session_id: SessionId,
        model_version: ModelVersion,
        prompt: Prompt,
        parameters: LlmParameters,
        received_at: DateTime<Utc>,
    },
    LlmRequestStarted {
        request_id: RequestId,
        started_at: DateTime<Utc>,
    },
    LlmResponseReceived {
        request_id: RequestId,
        response_text: ResponseText,
        metadata: ResponseMetadata,
        received_at: DateTime<Utc>,
    },
    LlmRequestFailed {
        request_id: RequestId,
        error_message: ErrorMessage,
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
        reason: Option<ChangeReason>,
        changed_at: DateTime<Utc>,
    },
    VersionUsageRecorded {
        model_version: ModelVersion,
        session_id: SessionId,
        recorded_at: DateTime<Utc>,
    },
    VersionDeactivated {
        model_version: ModelVersion,
        reason: Option<ChangeReason>,
        deactivated_at: DateTime<Utc>,
    },

    // User Events
    UserCreated {
        user_id: UserId,
        email: EmailAddress,
        display_name: Option<DisplayName>,
        created_at: DateTime<Utc>,
    },
    UserActivated {
        user_id: UserId,
        activated_at: DateTime<Utc>,
    },
    UserDeactivated {
        user_id: UserId,
        reason: Option<ChangeReason>,
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
            DomainEvent::VersionUsageRecorded { recorded_at, .. } => *recorded_at,
            DomainEvent::VersionDeactivated { deactivated_at, .. } => *deactivated_at,
            DomainEvent::UserCreated { created_at, .. } => *created_at,
            DomainEvent::UserActivated { activated_at, .. } => *activated_at,
            DomainEvent::UserDeactivated { deactivated_at, .. } => *deactivated_at,
        }
    }
}

// EventCore requires TryFrom<&'a ES::Event> for CommandExecutor
impl<'a> TryFrom<&'a DomainEvent> for DomainEvent {
    type Error = &'static str;

    fn try_from(event: &'a DomainEvent) -> Result<Self, Self::Error> {
        Ok(event.clone())
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_timestamp_extraction() {
        let now = Utc::now();
        let session_id = SessionId::generate();
        let user_id = UserId::generate();

        let event = DomainEvent::SessionStarted {
            session_id,
            user_id,
            application_id: ApplicationId::try_new("test-app".to_string()).unwrap(),
            started_at: now,
        };

        assert_eq!(event.occurred_at(), now);
    }
}
