//! EventCore event definitions for Union Square
//!
//! This module defines all domain events that are stored
//! in the event store using EventCore.

use eventcore::StreamId;
use serde::{Deserialize, Serialize};

use crate::domain::{
    audit_types::RequestUri,
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
        stream_id: StreamId,
        session_id: SessionId,
        user_id: UserId,
        application_id: ApplicationId,
        started_at: Timestamp,
    },
    SessionEnded {
        stream_id: StreamId,
        session_id: SessionId,
        ended_at: Timestamp,
        final_status: SessionStatus,
    },
    SessionTagged {
        stream_id: StreamId,
        session_id: SessionId,
        tag: Tag,
        tagged_at: Timestamp,
    },

    // LLM Request Events
    LlmRequestDeferred {
        stream_id: StreamId,
        request_id: RequestId,
        session_id: SessionId,
        received_at: Timestamp,
    },
    LlmRequestReceived {
        stream_id: StreamId,
        request_id: RequestId,
        session_id: SessionId,
        model_version: ModelVersion,
        prompt: Prompt,
        parameters: LlmParameters,
        received_at: Timestamp,
    },
    LlmRequestStarted {
        stream_id: StreamId,
        request_id: RequestId,
        started_at: Timestamp,
    },
    LlmResponseReceived {
        stream_id: StreamId,
        request_id: RequestId,
        response_text: ResponseText,
        metadata: ResponseMetadata,
        received_at: Timestamp,
    },
    LlmRequestFailed {
        stream_id: StreamId,
        request_id: RequestId,
        error_message: ErrorMessage,
        failed_at: Timestamp,
    },
    LlmRequestCancelled {
        stream_id: StreamId,
        request_id: RequestId,
        cancelled_at: Timestamp,
    },

    // Audit Error Events
    LlmRequestParsingFailed {
        stream_id: StreamId,
        request_id: RequestId,
        session_id: SessionId,
        parsing_error: ErrorMessage,
        raw_uri: RequestUri,
        occurred_at: Timestamp,
    },
    InvalidStateTransition {
        stream_id: StreamId,
        request_id: RequestId,
        session_id: SessionId,
        from_state: String,
        event_type: String,
        reason: ErrorMessage,
        occurred_at: Timestamp,
    },
    AuditEventProcessingFailed {
        stream_id: StreamId,
        request_id: RequestId,
        session_id: SessionId,
        event_type: String,
        error_message: ErrorMessage,
        occurred_at: Timestamp,
    },

    // Version Tracking Events
    VersionFirstSeen {
        stream_id: StreamId,
        model_version: ModelVersion,
        session_id: SessionId,
        first_seen_at: Timestamp,
    },
    VersionChanged {
        stream_id: StreamId,
        change_id: VersionChangeId,
        session_id: SessionId,
        from_version: ModelVersion,
        to_version: ModelVersion,
        change_type: VersionComparison,
        reason: Option<ChangeReason>,
        changed_at: Timestamp,
    },
    VersionUsageRecorded {
        stream_id: StreamId,
        model_version: ModelVersion,
        session_id: SessionId,
        recorded_at: Timestamp,
    },
    VersionDeactivated {
        stream_id: StreamId,
        model_version: ModelVersion,
        reason: Option<ChangeReason>,
        deactivated_at: Timestamp,
    },

    // F-score and Metrics Events
    FScoreCalculated {
        stream_id: StreamId,
        session_id: SessionId,
        model_version: ModelVersion,
        f_score: crate::domain::metrics::FScore,
        precision: Option<crate::domain::metrics::Precision>,
        recall: Option<crate::domain::metrics::Recall>,
        sample_count: SampleCount,
        calculated_at: Timestamp,
    },
    ApplicationFScoreCalculated {
        stream_id: StreamId,
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
        stream_id: StreamId,
        user_id: UserId,
        email: EmailAddress,
        display_name: Option<DisplayName>,
        created_at: Timestamp,
    },
    UserActivated {
        stream_id: StreamId,
        user_id: UserId,
        activated_at: Timestamp,
    },
    UserDeactivated {
        stream_id: StreamId,
        user_id: UserId,
        reason: Option<ChangeReason>,
        deactivated_at: Timestamp,
    },
}

impl eventcore::Event for DomainEvent {
    fn stream_id(&self) -> &StreamId {
        match self {
            DomainEvent::SessionStarted { stream_id, .. } => stream_id,
            DomainEvent::SessionEnded { stream_id, .. } => stream_id,
            DomainEvent::SessionTagged { stream_id, .. } => stream_id,
            DomainEvent::LlmRequestDeferred { stream_id, .. } => stream_id,
            DomainEvent::LlmRequestReceived { stream_id, .. } => stream_id,
            DomainEvent::LlmRequestStarted { stream_id, .. } => stream_id,
            DomainEvent::LlmResponseReceived { stream_id, .. } => stream_id,
            DomainEvent::LlmRequestFailed { stream_id, .. } => stream_id,
            DomainEvent::LlmRequestCancelled { stream_id, .. } => stream_id,
            DomainEvent::LlmRequestParsingFailed { stream_id, .. } => stream_id,
            DomainEvent::InvalidStateTransition { stream_id, .. } => stream_id,
            DomainEvent::AuditEventProcessingFailed { stream_id, .. } => stream_id,
            DomainEvent::VersionFirstSeen { stream_id, .. } => stream_id,
            DomainEvent::VersionChanged { stream_id, .. } => stream_id,
            DomainEvent::VersionUsageRecorded { stream_id, .. } => stream_id,
            DomainEvent::VersionDeactivated { stream_id, .. } => stream_id,
            DomainEvent::FScoreCalculated { stream_id, .. } => stream_id,
            DomainEvent::ApplicationFScoreCalculated { stream_id, .. } => stream_id,
            DomainEvent::UserCreated { stream_id, .. } => stream_id,
            DomainEvent::UserActivated { stream_id, .. } => stream_id,
            DomainEvent::UserDeactivated { stream_id, .. } => stream_id,
        }
    }

    fn event_type_name() -> &'static str
    where
        Self: Sized,
    {
        "DomainEvent"
    }
}

impl DomainEvent {
    /// Get the timestamp of when this event occurred
    pub fn occurred_at(&self) -> Timestamp {
        match self {
            DomainEvent::SessionStarted { started_at, .. } => *started_at,
            DomainEvent::SessionEnded { ended_at, .. } => *ended_at,
            DomainEvent::SessionTagged { tagged_at, .. } => *tagged_at,
            DomainEvent::LlmRequestDeferred { received_at, .. } => *received_at,
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
            stream_id: StreamId::try_new("session-test".to_string()).unwrap(),
            session_id,
            user_id,
            application_id: ApplicationId::try_new("test-app".to_string()).unwrap(),
            started_at: now,
        };

        assert_eq!(event.occurred_at(), now);
    }
}
