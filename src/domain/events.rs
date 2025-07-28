//! EventCore event definitions for Union Square
//!
//! This module defines all domain events that are stored
//! in the event store using EventCore.

use serde::{Deserialize, Serialize};

use crate::domain::{
    llm::{ModelVersion, RequestId, ResponseMetadata},
    metrics::{SampleCount, Timestamp},
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
        started_at: Timestamp,
    },
    SessionEnded {
        session_id: SessionId,
        ended_at: Timestamp,
        final_status: SessionStatus,
    },
    SessionTagged {
        session_id: SessionId,
        tag: Tag,
        tagged_at: Timestamp,
    },

    // LLM Request Events
    LlmRequestReceived {
        request_id: RequestId,
        session_id: SessionId,
        model_version: ModelVersion,
        prompt: Prompt,
        parameters: LlmParameters,
        received_at: Timestamp,
    },
    LlmRequestStarted {
        request_id: RequestId,
        started_at: Timestamp,
    },
    LlmResponseReceived {
        request_id: RequestId,
        response_text: ResponseText,
        metadata: ResponseMetadata,
        received_at: Timestamp,
    },
    LlmRequestFailed {
        request_id: RequestId,
        error_message: ErrorMessage,
        failed_at: Timestamp,
    },
    LlmRequestCancelled {
        request_id: RequestId,
        cancelled_at: Timestamp,
    },

    // Audit Error Events
    LlmRequestParsingFailed {
        request_id: RequestId,
        session_id: SessionId,
        parsing_error: ErrorMessage,
        raw_uri: String,
        occurred_at: Timestamp,
    },
    InvalidStateTransition {
        request_id: RequestId,
        session_id: SessionId,
        from_state: String,
        event_type: String,
        reason: ErrorMessage,
        occurred_at: Timestamp,
    },
    AuditEventProcessingFailed {
        request_id: RequestId,
        session_id: SessionId,
        event_type: String,
        error_message: ErrorMessage,
        occurred_at: Timestamp,
    },

    // Version Tracking Events
    VersionFirstSeen {
        model_version: ModelVersion,
        session_id: SessionId,
        first_seen_at: Timestamp,
    },
    VersionChanged {
        change_id: VersionChangeId,
        session_id: SessionId,
        from_version: ModelVersion,
        to_version: ModelVersion,
        change_type: VersionComparison,
        reason: Option<ChangeReason>,
        changed_at: Timestamp,
    },
    VersionUsageRecorded {
        model_version: ModelVersion,
        session_id: SessionId,
        recorded_at: Timestamp,
    },
    VersionDeactivated {
        model_version: ModelVersion,
        reason: Option<ChangeReason>,
        deactivated_at: Timestamp,
    },

    // F-score and Metrics Events
    FScoreCalculated {
        session_id: SessionId,
        model_version: ModelVersion,
        f_score: crate::domain::metrics::FScore,
        precision: Option<crate::domain::metrics::Precision>,
        recall: Option<crate::domain::metrics::Recall>,
        sample_count: SampleCount,
        calculated_at: Timestamp,
    },
    ApplicationFScoreCalculated {
        session_id: SessionId,
        application_id: ApplicationId,
        model_version: ModelVersion,
        f_score: crate::domain::metrics::FScore,
        precision: Option<crate::domain::metrics::Precision>,
        recall: Option<crate::domain::metrics::Recall>,
        sample_count: SampleCount,
        calculated_at: Timestamp,
    },

    // User Events
    UserCreated {
        user_id: UserId,
        email: EmailAddress,
        display_name: Option<DisplayName>,
        created_at: Timestamp,
    },
    UserActivated {
        user_id: UserId,
        activated_at: Timestamp,
    },
    UserDeactivated {
        user_id: UserId,
        reason: Option<ChangeReason>,
        deactivated_at: Timestamp,
    },
}

impl DomainEvent {
    /// Get the timestamp of when this event occurred
    pub fn occurred_at(&self) -> Timestamp {
        match self {
            DomainEvent::SessionStarted { started_at, .. } => *started_at,
            DomainEvent::SessionEnded { ended_at, .. } => *ended_at,
            DomainEvent::SessionTagged { tagged_at, .. } => *tagged_at,
            DomainEvent::LlmRequestReceived { received_at, .. } => *received_at,
            DomainEvent::LlmRequestStarted { started_at, .. } => *started_at,
            DomainEvent::LlmResponseReceived { received_at, .. } => *received_at,
            DomainEvent::LlmRequestFailed { failed_at, .. } => *failed_at,
            DomainEvent::LlmRequestCancelled { cancelled_at, .. } => *cancelled_at,
            DomainEvent::LlmRequestParsingFailed { occurred_at, .. } => *occurred_at,
            DomainEvent::InvalidStateTransition { occurred_at, .. } => *occurred_at,
            DomainEvent::AuditEventProcessingFailed { occurred_at, .. } => *occurred_at,
            DomainEvent::VersionFirstSeen { first_seen_at, .. } => *first_seen_at,
            DomainEvent::VersionChanged { changed_at, .. } => *changed_at,
            DomainEvent::VersionUsageRecorded { recorded_at, .. } => *recorded_at,
            DomainEvent::VersionDeactivated { deactivated_at, .. } => *deactivated_at,
            DomainEvent::FScoreCalculated { calculated_at, .. } => *calculated_at,
            DomainEvent::ApplicationFScoreCalculated { calculated_at, .. } => *calculated_at,
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
        let now = Timestamp::now();
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
