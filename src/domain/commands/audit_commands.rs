//! EventCore commands for audit events
//!
//! These commands map from the audit path events to EventCore commands,
//! enabling persistence of all proxy operations to the event store.

use eventcore::{CommandError, CommandLogic, NewEvents, StreamId};
use eventcore_macros::Command;
use serde::{Deserialize, Serialize};

use crate::domain::{
    audit_types, events::DomainEvent, llm, metrics::Timestamp, session::SessionId,
};

use super::llm_request_parser::ParsedLlmRequest;

use std::fmt;

/// Wrapper for parsed LLM request that includes any parsing error
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedLlmRequestWithError {
    pub parsed: ParsedLlmRequest,
    pub error: Option<String>,
    pub raw_uri: audit_types::RequestUri,
}

impl ParsedLlmRequestWithError {
    /// Create a new parsed request with error information
    pub const fn new(
        parsed: ParsedLlmRequest,
        error: Option<String>,
        raw_uri: audit_types::RequestUri,
    ) -> Self {
        Self {
            parsed,
            error,
            raw_uri,
        }
    }
}

/// State machine for request lifecycle
/// This ensures illegal states are unrepresentable at the type level
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequestLifecycle {
    /// Initial state - no request received yet
    NotStarted,
    /// Request headers received but body not yet parsed
    Deferred {
        request_id: crate::domain::llm::RequestId,
        received_at: Timestamp,
    },
    /// Request has been received and parsed from the client
    Received {
        request_id: crate::domain::llm::RequestId,
        received_at: Timestamp,
    },
    /// Request has been forwarded to the target
    Forwarded {
        request_id: crate::domain::llm::RequestId,
        received_at: Timestamp,
        forwarded_at: Timestamp,
    },
    /// Response has been received from the target
    ResponseReceived {
        request_id: crate::domain::llm::RequestId,
        received_at: Timestamp,
        forwarded_at: Timestamp,
        response_received_at: Timestamp,
    },
    /// Response has been returned to the client (completed)
    Completed {
        request_id: crate::domain::llm::RequestId,
        received_at: Timestamp,
        forwarded_at: Timestamp,
        response_received_at: Timestamp,
        completed_at: Timestamp,
    },
    /// Request failed at some point
    Failed {
        request_id: crate::domain::llm::RequestId,
        failed_at: Timestamp,
        reason: String,
    },
}

/// State for request tracking - wraps the lifecycle state machine
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestState {
    pub lifecycle: RequestLifecycle,
}

impl fmt::Display for RequestState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.lifecycle {
            RequestLifecycle::NotStarted => write!(f, "NotStarted"),
            RequestLifecycle::Deferred { request_id, .. } => write!(f, "Deferred({request_id:?})"),
            RequestLifecycle::Received { request_id, .. } => write!(f, "Received({request_id:?})"),
            RequestLifecycle::Forwarded { request_id, .. } => {
                write!(f, "Forwarded({request_id:?})")
            }
            RequestLifecycle::ResponseReceived { request_id, .. } => {
                write!(f, "ResponseReceived({request_id:?})")
            }
            RequestLifecycle::Completed { request_id, .. } => {
                write!(f, "Completed({request_id:?})")
            }
            RequestLifecycle::Failed {
                request_id, reason, ..
            } => write!(f, "Failed({request_id:?}, {reason})"),
        }
    }
}

impl Default for RequestState {
    fn default() -> Self {
        Self::new()
    }
}

impl RequestState {
    /// Create a new request state in the initial state
    pub const fn new() -> Self {
        Self {
            lifecycle: RequestLifecycle::NotStarted,
        }
    }

    /// Get the current request ID if available
    pub fn request_id(&self) -> Option<&crate::domain::llm::RequestId> {
        match &self.lifecycle {
            RequestLifecycle::NotStarted => None,
            RequestLifecycle::Deferred { request_id, .. }
            | RequestLifecycle::Received { request_id, .. }
            | RequestLifecycle::Forwarded { request_id, .. }
            | RequestLifecycle::ResponseReceived { request_id, .. }
            | RequestLifecycle::Completed { request_id, .. }
            | RequestLifecycle::Failed { request_id, .. } => Some(request_id),
        }
    }

    /// Apply an event to update the state
    /// This enforces valid state transitions
    pub fn apply(&mut self, event: &DomainEvent) {
        self.lifecycle = self.lifecycle.clone().transition(event);
    }

    /// Check if the request has been received
    pub const fn is_request_received(&self) -> bool {
        self.lifecycle.is_request_received()
    }

    /// Check if the request has been forwarded
    pub const fn is_request_forwarded(&self) -> bool {
        self.lifecycle.is_request_forwarded()
    }

    /// Check if the response has been received
    pub const fn is_response_received(&self) -> bool {
        self.lifecycle.is_response_received()
    }

    /// Check if the response has been returned (completed)
    pub const fn is_response_returned(&self) -> bool {
        self.lifecycle.is_response_returned()
    }

    /// Check if the request has failed
    pub const fn is_failed(&self) -> bool {
        self.lifecycle.is_failed()
    }
}

impl RequestLifecycle {
    /// Transition to a new state based on an event
    /// This is a pure function that returns the new state
    pub fn transition(self, event: &DomainEvent) -> Self {
        use RequestLifecycle::*;

        match (&self, event) {
            // Valid transitions from NotStarted
            (
                NotStarted,
                DomainEvent::LlmRequestDeferred {
                    request_id,
                    received_at,
                    ..
                },
            ) => Deferred {
                request_id: request_id.clone(),
                received_at: *received_at,
            },
            (
                NotStarted,
                DomainEvent::LlmRequestReceived {
                    request_id,
                    received_at,
                    ..
                },
            ) => Received {
                request_id: request_id.clone(),
                received_at: *received_at,
            },

            // Valid transitions from Deferred
            (
                Deferred {
                    request_id,
                    received_at,
                },
                DomainEvent::LlmRequestReceived { .. },
            ) => Received {
                request_id: request_id.clone(),
                received_at: *received_at,
            },
            (
                Deferred {
                    request_id,
                    received_at,
                },
                DomainEvent::LlmRequestStarted { started_at, .. },
            ) => Forwarded {
                request_id: request_id.clone(),
                received_at: *received_at,
                forwarded_at: *started_at,
            },

            // Valid transitions from Received
            (
                Received {
                    request_id,
                    received_at,
                },
                DomainEvent::LlmRequestStarted { started_at, .. },
            ) => Forwarded {
                request_id: request_id.clone(),
                received_at: *received_at,
                forwarded_at: *started_at,
            },

            // Valid transitions from Forwarded
            (
                Forwarded {
                    request_id,
                    received_at,
                    forwarded_at,
                },
                DomainEvent::LlmResponseReceived {
                    received_at: response_received_at,
                    ..
                },
            ) => ResponseReceived {
                request_id: request_id.clone(),
                received_at: *received_at,
                forwarded_at: *forwarded_at,
                response_received_at: *response_received_at,
            },

            // Response received implies completion in current model
            // This matches the original behavior where response_returned was set with response_received
            (
                ResponseReceived {
                    request_id,
                    received_at,
                    forwarded_at,
                    response_received_at,
                },
                _,
            ) => {
                Completed {
                    request_id: request_id.clone(),
                    received_at: *received_at,
                    forwarded_at: *forwarded_at,
                    response_received_at: *response_received_at,
                    completed_at: *response_received_at, // Using same timestamp as original behavior
                }
            }

            // Transitions to Failed state
            (
                _,
                DomainEvent::LlmRequestFailed {
                    request_id,
                    error_message,
                    failed_at,
                    ..
                },
            ) => Failed {
                request_id: request_id.clone(),
                failed_at: *failed_at,
                reason: error_message.as_ref().to_string(),
            },

            // Cancelled requests are considered failed
            (
                _,
                DomainEvent::LlmRequestCancelled {
                    request_id,
                    cancelled_at,
                    ..
                },
            ) => Failed {
                request_id: request_id.clone(),
                failed_at: *cancelled_at,
                reason: error_messages::REQUEST_CANCELLED.to_string(),
            },

            // No state change for other events or invalid transitions
            (current_state, _) => current_state.clone(),
        }
    }

    /// Check if the request has been received (including deferred)
    pub const fn is_request_received(&self) -> bool {
        matches!(
            self,
            RequestLifecycle::Deferred { .. }
                | RequestLifecycle::Received { .. }
                | RequestLifecycle::Forwarded { .. }
                | RequestLifecycle::ResponseReceived { .. }
                | RequestLifecycle::Completed { .. }
        )
    }

    /// Check if the request has been forwarded
    pub const fn is_request_forwarded(&self) -> bool {
        matches!(
            self,
            RequestLifecycle::Forwarded { .. }
                | RequestLifecycle::ResponseReceived { .. }
                | RequestLifecycle::Completed { .. }
        )
    }

    /// Check if the response has been received
    pub const fn is_response_received(&self) -> bool {
        matches!(
            self,
            RequestLifecycle::ResponseReceived { .. } | RequestLifecycle::Completed { .. }
        )
    }

    /// Check if the response has been returned (completed)
    pub const fn is_response_returned(&self) -> bool {
        matches!(self, RequestLifecycle::Completed { .. })
    }

    /// Check if the request has failed
    pub const fn is_failed(&self) -> bool {
        matches!(self, RequestLifecycle::Failed { .. })
    }
}

/// Unified command to record audit events
/// This single command handles all audit event types, simplifying the architecture
#[derive(Debug, Clone, Serialize, Deserialize, Command)]
pub struct RecordAuditEvent {
    #[stream]
    pub session_stream: StreamId,
    #[stream]
    pub request_stream: StreamId,
    pub request_id: llm::RequestId,
    pub session_id: SessionId,
    pub audit_event: audit_types::AuditEventType,
    pub timestamp: Timestamp,
    /// Optional parsed request data (only used for RequestReceived events with body)
    #[serde(skip)]
    pub parsed_request: Option<ParsedLlmRequestWithError>,
}

impl RecordAuditEvent {
    /// Builder method to create a new instance
    pub fn builder() -> RecordAuditEventBuilder {
        RecordAuditEventBuilder::default()
    }

    /// Create stream ID for a session
    pub fn session_stream_id(session_id: &SessionId) -> Result<StreamId, AuditCommandError> {
        crate::domain::streams::session_stream(session_id)
            .map_err(|e| AuditCommandError::InvalidStreamId(e.to_string()))
    }

    /// Create stream ID for a request
    pub fn request_stream_id(request_id: &llm::RequestId) -> Result<StreamId, AuditCommandError> {
        crate::domain::streams::request_stream(request_id)
            .map_err(|e| AuditCommandError::InvalidStreamId(e.to_string()))
    }

    /// Set pre-parsed LLM request data directly.
    ///
    /// Parsing must happen at the adapter boundary; the domain command
    /// only accepts already-parsed semantic facts.
    pub fn with_parsed_request(mut self, parsed: Option<ParsedLlmRequestWithError>) -> Self {
        // Only set parsed request for RequestReceived events
        if matches!(
            &self.audit_event,
            audit_types::AuditEventType::RequestReceived { .. }
        ) {
            self.parsed_request = parsed;
        }
        self
    }
}

/// Error messages as constants for compile-time validation
mod error_messages {
    use crate::domain::types::ErrorMessage;
    use eventcore::CommandError;

    pub const REQUEST_ALREADY_RECEIVED: &str = "Request already received";
    pub const REQUEST_ALREADY_FORWARDED: &str = "Request already forwarded";
    pub const RESPONSE_ALREADY_RECEIVED: &str = "Response already received";
    pub const CANNOT_FORWARD_UNRECEIVED: &str = "Cannot forward request that hasn't been received";
    pub const CANNOT_RECEIVE_RESPONSE_UNFORWARDED: &str =
        "Cannot receive response for request that hasn't been forwarded";
    pub const AUDIT_EVENT_NOT_IMPLEMENTED: &str = "Audit event type not yet implemented";
    pub const UNKNOWN_PARSING_ERROR: &str = "Unknown parsing error";
    pub const REQUEST_CANCELLED: &str = "Request cancelled";

    /// Create an ErrorMessage from a static string - all static strings are controlled and non-empty
    #[inline]
    pub fn static_error(msg: &'static str) -> Result<ErrorMessage, CommandError> {
        ErrorMessage::try_new(msg.to_string()).map_err(|e| {
            CommandError::ValidationError(format!("Invalid static error message: {e}"))
        })
    }

    /// Try to create an ErrorMessage from raw text; fall back to a static error if validation fails
    #[inline]
    pub fn parse_error(raw: String) -> Result<ErrorMessage, CommandError> {
        ErrorMessage::try_new(raw).or_else(|_| static_error(UNKNOWN_PARSING_ERROR))
    }
}

/// Pure functions to transform audit events into domain events
mod transformers {
    use super::*;

    /// Transform RequestReceived audit event to domain event
    pub fn request_received_to_domain(
        session_stream: StreamId,
        request_id: llm::RequestId,
        session_id: SessionId,
        timestamp: Timestamp,
        parsed_request: &ParsedLlmRequest,
    ) -> Result<DomainEvent, CommandError> {
        Ok(DomainEvent::LlmRequestReceived {
            stream_id: session_stream,
            request_id,
            session_id,
            model_version: parsed_request.model_version.clone(),
            prompt: parsed_request.prompt.clone(),
            parameters: parsed_request.parameters.clone(),
            received_at: timestamp,
        })
    }

    /// Transform RequestForwarded audit event to domain event
    pub fn request_forwarded_to_domain(
        request_stream: StreamId,
        request_id: llm::RequestId,
        timestamp: Timestamp,
    ) -> DomainEvent {
        DomainEvent::LlmRequestStarted {
            stream_id: request_stream,
            request_id,
            started_at: timestamp,
        }
    }

    /// Transform ResponseReceived audit event to domain event
    pub fn response_received_to_domain(
        request_stream: StreamId,
        request_id: llm::RequestId,
        timestamp: Timestamp,
    ) -> Result<DomainEvent, CommandError> {
        // For now, we don't have the response body here
        // TODO: Implement response body parsing similar to request parsing
        let response_text = crate::domain::types::ResponseText::try_new(
            "Response body parsing not yet implemented".to_string(),
        )
        .map_err(|e| {
            CommandError::ValidationError(format!(
                "Failed to create response text placeholder: {e}"
            ))
        })?;

        let metadata = crate::domain::llm::ResponseMetadata::default();

        Ok(DomainEvent::LlmResponseReceived {
            stream_id: request_stream,
            request_id,
            response_text,
            metadata,
            received_at: timestamp,
        })
    }
}

impl CommandLogic for RecordAuditEvent {
    type State = RequestState;
    type Event = DomainEvent;

    fn apply(&self, mut state: Self::State, event: &Self::Event) -> Self::State {
        state.apply(event);
        state
    }

    fn handle(&self, state: Self::State) -> Result<NewEvents<Self::Event>, CommandError> {
        let mut events = Vec::new();

        use audit_types::AuditEventType::*;

        // Determine which event to emit based on audit event type and current state
        match &self.audit_event {
            RequestReceived { .. } => {
                if !state.is_request_received() {
                    if let Some(parsed_request) = &self.parsed_request {
                        // Emit the request received event with parsed data
                        let event = transformers::request_received_to_domain(
                            self.session_stream.clone(),
                            self.request_id.clone(),
                            self.session_id.clone(),
                            self.timestamp,
                            &parsed_request.parsed,
                        )?;

                        events.push(event);

                        // If there was a parsing error, emit an error event
                        if let Some(error_msg) = &parsed_request.error {
                            let error_message = error_messages::parse_error(error_msg.clone())?;

                            events.push(DomainEvent::LlmRequestParsingFailed {
                                stream_id: self.request_stream.clone(),
                                request_id: self.request_id.clone(),
                                session_id: self.session_id.clone(),
                                parsing_error: error_message,
                                raw_uri: parsed_request.raw_uri.clone(),
                                occurred_at: self.timestamp,
                            });
                        }
                    } else {
                        // Body not yet available — defer parsing
                        events.push(DomainEvent::LlmRequestDeferred {
                            stream_id: self.session_stream.clone(),
                            request_id: self.request_id.clone(),
                            session_id: self.session_id.clone(),
                            received_at: self.timestamp,
                        });
                    }
                } else {
                    // Invalid state transition - request already received
                    events.push(DomainEvent::InvalidStateTransition {
                        stream_id: self.request_stream.clone(),
                        request_id: self.request_id.clone(),
                        session_id: self.session_id.clone(),
                        from_state: state.to_string(),
                        event_type: "RequestReceived".to_string(),
                        reason: error_messages::static_error(
                            error_messages::REQUEST_ALREADY_RECEIVED,
                        )?,
                        occurred_at: self.timestamp,
                    });
                }
            }
            RequestForwarded { start_time, .. } => {
                if !state.is_request_forwarded() {
                    // Check if we're in a valid state to forward
                    if !state.is_request_received() {
                        // Invalid transition - trying to forward before receiving
                        events.push(DomainEvent::InvalidStateTransition {
                            stream_id: self.request_stream.clone(),
                            request_id: self.request_id.clone(),
                            session_id: self.session_id.clone(),
                            from_state: state.to_string(),
                            event_type: "RequestForwarded".to_string(),
                            reason: error_messages::static_error(
                                error_messages::CANNOT_FORWARD_UNRECEIVED,
                            )?,
                            occurred_at: self.timestamp,
                        });
                    } else {
                        let event = transformers::request_forwarded_to_domain(
                            self.request_stream.clone(),
                            self.request_id.clone(),
                            *start_time,
                        );

                        events.push(event);
                    }
                } else {
                    // Invalid state transition - request already forwarded
                    events.push(DomainEvent::InvalidStateTransition {
                        stream_id: self.request_stream.clone(),
                        request_id: self.request_id.clone(),
                        session_id: self.session_id.clone(),
                        from_state: state.to_string(),
                        event_type: "RequestForwarded".to_string(),
                        reason: error_messages::static_error(
                            error_messages::REQUEST_ALREADY_FORWARDED,
                        )?,
                        occurred_at: self.timestamp,
                    });
                }
            }
            ResponseReceived { .. } => {
                // Only emit response if request has been forwarded and response not yet received
                if state.is_request_forwarded() && !state.is_response_received() {
                    let event = transformers::response_received_to_domain(
                        self.request_stream.clone(),
                        self.request_id.clone(),
                        self.timestamp,
                    )?;

                    events.push(event);
                } else if !state.is_request_forwarded() {
                    // Invalid transition - response received before forwarding
                    events.push(DomainEvent::InvalidStateTransition {
                        stream_id: self.request_stream.clone(),
                        request_id: self.request_id.clone(),
                        session_id: self.session_id.clone(),
                        from_state: state.to_string(),
                        event_type: "ResponseReceived".to_string(),
                        reason: error_messages::static_error(
                            error_messages::CANNOT_RECEIVE_RESPONSE_UNFORWARDED,
                        )?,
                        occurred_at: self.timestamp,
                    });
                } else {
                    // Response already received
                    events.push(DomainEvent::InvalidStateTransition {
                        stream_id: self.request_stream.clone(),
                        request_id: self.request_id.clone(),
                        session_id: self.session_id.clone(),
                        from_state: state.to_string(),
                        event_type: "ResponseReceived".to_string(),
                        reason: error_messages::static_error(
                            error_messages::RESPONSE_ALREADY_RECEIVED,
                        )?,
                        occurred_at: self.timestamp,
                    });
                }
            }
            ResponseReturned { .. } => {
                // For now, we don't emit any specific event for response returned
                // The LlmResponseReceived event already captures the completion
            }
            _ => {
                // Other audit event types not yet handled

                // Emit an error event for unhandled audit event types
                let event_type_str = match &self.audit_event {
                    Error { .. } => "Error",
                    _ => "Unknown",
                };

                events.push(DomainEvent::AuditEventProcessingFailed {
                    stream_id: self.request_stream.clone(),
                    request_id: self.request_id.clone(),
                    session_id: self.session_id.clone(),
                    event_type: event_type_str.to_string(),
                    error_message: error_messages::static_error(
                        error_messages::AUDIT_EVENT_NOT_IMPLEMENTED,
                    )?,
                    occurred_at: self.timestamp,
                });
            }
        }

        Ok(events.into())
    }
}

// The redundant command structs have been removed in favor of the unified RecordAuditEvent command

/// Command to process a pre-parsed LLM request and emit domain events.
///
/// All parsing happens at the adapter boundary; this command only accepts
/// already-parsed semantic facts.
#[derive(Debug, Clone, Serialize, Deserialize, Command)]
pub struct ProcessRequestBody {
    #[stream]
    pub session_stream: StreamId,
    #[stream]
    pub request_stream: StreamId,
    pub request_id: llm::RequestId,
    pub session_id: SessionId,
    #[serde(skip)]
    pub parsed_request: Option<ParsedLlmRequestWithError>,
    pub timestamp: Timestamp,
}

impl CommandLogic for ProcessRequestBody {
    type State = RequestState;
    type Event = DomainEvent;

    fn apply(&self, mut state: Self::State, event: &Self::Event) -> Self::State {
        state.apply(event);
        state
    }

    fn handle(&self, state: Self::State) -> Result<NewEvents<Self::Event>, CommandError> {
        let mut events = Vec::new();

        // Only process if request is deferred or not yet received
        if matches!(state.lifecycle, RequestLifecycle::Received { .. }) {
            return Ok(events.into());
        }

        let parsed_request = self
            .parsed_request
            .as_ref()
            .ok_or_else(|| CommandError::ValidationError("Missing parsed request".to_string()))?;
        let parsed = &parsed_request.parsed;

        events.push(DomainEvent::LlmRequestReceived {
            stream_id: self.session_stream.clone(),
            request_id: self.request_id.clone(),
            session_id: self.session_id.clone(),
            model_version: parsed.model_version.clone(),
            prompt: parsed.prompt.clone(),
            parameters: parsed.parameters.clone(),
            received_at: self.timestamp,
        });

        // If there was a parsing error, emit an error event
        if let Some(ref error_msg) = parsed_request.error {
            let error_message = error_messages::parse_error(error_msg.clone())?;

            events.push(DomainEvent::LlmRequestParsingFailed {
                stream_id: self.request_stream.clone(),
                request_id: self.request_id.clone(),
                session_id: self.session_id.clone(),
                parsing_error: error_message,
                raw_uri: parsed_request.raw_uri.clone(),
                occurred_at: self.timestamp,
            });
        }

        Ok(events.into())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AuditCommandError {
    #[error("Invalid stream ID: {0}")]
    InvalidStreamId(String),

    #[error("Wrong event type: expected {expected}, got {actual}")]
    WrongEventType { expected: String, actual: String },

    #[error("Invalid timestamp: {0}")]
    InvalidTimestamp(String),

    #[error("Invalid provider name: {0}")]
    InvalidProviderName(String),

    #[error("Invalid model ID: {0}")]
    InvalidModelId(String),

    #[error("Invalid prompt: {0}")]
    InvalidPrompt(String),

    #[error("Invalid response text: {0}")]
    InvalidResponseText(String),

    #[error("Invalid field: {0}")]
    InvalidField(String),
}

impl fmt::Display for RequestLifecycle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotStarted => write!(f, "NotStarted"),
            Self::Deferred { request_id, received_at } =>
                write!(f, "Deferred {{ request_id: {request_id:?}, received_at: {received_at:?} }}"),
            Self::Received { request_id, received_at } =>
                write!(f, "Received {{ request_id: {request_id:?}, received_at: {received_at:?} }}"),
            Self::Forwarded { request_id, received_at, forwarded_at } =>
                write!(f, "Forwarded {{ request_id: {request_id:?}, received_at: {received_at:?}, forwarded_at: {forwarded_at:?} }}"),
            Self::ResponseReceived { request_id, received_at, forwarded_at, response_received_at } =>
                write!(f, "ResponseReceived {{ request_id: {request_id:?}, received_at: {received_at:?}, forwarded_at: {forwarded_at:?}, response_received_at: {response_received_at:?} }}"),
            Self::Completed { request_id, received_at, forwarded_at, response_received_at, completed_at } =>
                write!(f, "Completed {{ request_id: {request_id:?}, received_at: {received_at:?}, forwarded_at: {forwarded_at:?}, response_received_at: {response_received_at:?}, completed_at: {completed_at:?} }}"),
            Self::Failed { request_id, failed_at, reason } =>
                write!(f, "Failed {{ request_id: {request_id:?}, failed_at: {failed_at:?}, reason: {reason} }}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::proxy_audit::convert_audit_event;
    use crate::domain::streams::{request_stream, session_stream};
    use chrono::Utc;
    use eventcore_types::EventStore;

    #[test]
    fn test_audit_event_to_unified_command() {
        let proxy_event = crate::proxy::types::AuditEvent {
            request_id: crate::proxy::types::RequestId::new(),
            session_id: crate::proxy::types::SessionId::new(),
            timestamp: Utc::now(),
            event_type: crate::proxy::types::AuditEventType::RequestReceived {
                method: crate::proxy::types::HttpMethod::try_new("GET".to_string()).unwrap(),
                uri: crate::proxy::types::RequestUri::try_new("/test".to_string()).unwrap(),
                headers: crate::proxy::types::Headers::new(),
                body_size: crate::proxy::types::BodySize::from(0),
            },
        };

        let command = convert_audit_event(&proxy_event);
        assert!(command.is_ok());
        let cmd = command.unwrap();
        // Verify the audit event type was preserved
        assert!(matches!(
            cmd.audit_event,
            audit_types::AuditEventType::RequestReceived { .. }
        ));
    }

    #[test]
    fn test_unified_command_handles_all_event_types() {
        // Test that the adapter can handle any audit event type
        let event_types = vec![
            crate::proxy::types::AuditEventType::RequestReceived {
                method: crate::proxy::types::HttpMethod::try_new("GET".to_string()).unwrap(),
                uri: crate::proxy::types::RequestUri::try_new("/test".to_string()).unwrap(),
                headers: crate::proxy::types::Headers::new(),
                body_size: crate::proxy::types::BodySize::from(0),
            },
            crate::proxy::types::AuditEventType::RequestForwarded {
                target_url: crate::proxy::types::TargetUrl::try_new(
                    "https://api.openai.com/v1/chat/completions".to_string(),
                )
                .unwrap(),
                start_time: Utc::now(),
            },
            crate::proxy::types::AuditEventType::ResponseReceived {
                status: crate::proxy::types::HttpStatusCode::try_new(200).unwrap(),
                headers: crate::proxy::types::Headers::new(),
                body_size: crate::proxy::types::BodySize::from(1024),
                duration_ms: crate::proxy::types::DurationMillis::from(100),
            },
            crate::proxy::types::AuditEventType::ResponseReturned {
                duration_ms: crate::proxy::types::DurationMillis::from(200),
            },
        ];

        for event_type in event_types {
            let proxy_event = crate::proxy::types::AuditEvent {
                request_id: crate::proxy::types::RequestId::new(),
                session_id: crate::proxy::types::SessionId::new(),
                timestamp: Utc::now(),
                event_type: event_type.clone(),
            };

            let command = convert_audit_event(&proxy_event);
            assert!(command.is_ok(), "Failed for event type: {event_type:?}");
        }
    }

    #[tokio::test]
    async fn test_unified_command_logic() {
        // Create a command
        let session_id = SessionId::generate();
        let request_id = llm::RequestId::generate();
        let _command = RecordAuditEvent {
            session_stream: session_stream(&session_id).unwrap(),
            request_stream: request_stream(&request_id).unwrap(),
            request_id: request_id.clone(),
            session_id,
            audit_event: audit_types::AuditEventType::RequestReceived {
                method: audit_types::HttpMethod::try_new("POST".to_string()).unwrap(),
                uri: audit_types::RequestUri::try_new("/api/test".to_string()).unwrap(),
                headers: audit_types::HttpHeaders::new(),
                body_size: audit_types::BodySize::from(1024),
            },
            timestamp: Timestamp::now(),
            parsed_request: None,
        };

        // Assert that RecordAuditEvent implements CommandLogic
        fn assert_command_logic<T: CommandLogic>() {}
        assert_command_logic::<RecordAuditEvent>();
    }

    #[test]
    fn test_unified_command_with_body_parsing() {
        let session_id = SessionId::generate();
        let request_id = llm::RequestId::generate();

        // Create a sample OpenAI request body
        let body = serde_json::json!({
            "model": "gpt-4",
            "messages": [
                {"role": "user", "content": "Hello, world!"}
            ],
            "temperature": 0.7
        });

        let command = RecordAuditEvent {
            session_stream: session_stream(&session_id).unwrap(),
            request_stream: request_stream(&request_id).unwrap(),
            request_id: request_id.clone(),
            session_id,
            audit_event: audit_types::AuditEventType::RequestReceived {
                method: audit_types::HttpMethod::try_new("POST".to_string()).unwrap(),
                uri: audit_types::RequestUri::try_new("/v1/chat/completions".to_string()).unwrap(),
                headers: audit_types::HttpHeaders::new(),
                body_size: audit_types::BodySize::from(body.to_string().len()),
            },
            timestamp: Timestamp::now(),
            parsed_request: None,
        };

        // Apply body parsing at adapter boundary
        let uri = audit_types::RequestUri::try_new("/v1/chat/completions".to_string()).unwrap();
        let headers = audit_types::HttpHeaders::new();
        let parsed = crate::adapters::proxy_audit::parse_request_body(
            body.to_string().as_bytes(),
            &uri,
            &headers,
        );
        let command_with_body = command.with_parsed_request(Some(parsed));

        // Verify parsing succeeded
        assert!(command_with_body.parsed_request.is_some());
        let parsed_with_error = command_with_body.parsed_request.unwrap();
        assert!(parsed_with_error.error.is_none()); // No error expected
        assert_eq!(
            parsed_with_error.parsed.model_version.model_id.as_ref(),
            "gpt-4"
        );
        assert!(parsed_with_error
            .parsed
            .prompt
            .as_ref()
            .contains("user: Hello, world!"));
    }

    #[tokio::test]
    async fn test_process_request_body_command() {
        let session_id = SessionId::generate();
        let request_id = llm::RequestId::generate();

        let body = serde_json::json!({
            "model": "claude-3-opus-20240229",
            "messages": [
                {"role": "user", "content": "What is 2+2?"}
            ],
            "max_tokens": 100
        });

        let uri = audit_types::RequestUri::try_new("/v1/messages".to_string()).unwrap();
        let headers = audit_types::HttpHeaders::new();
        let parsed_request = crate::adapters::proxy_audit::parse_request_body(
            body.to_string().as_bytes(),
            &uri,
            &headers,
        );

        let _command = ProcessRequestBody {
            session_stream: session_stream(&session_id).unwrap(),
            request_stream: request_stream(&request_id).unwrap(),
            request_id: request_id.clone(),
            session_id,
            parsed_request: Some(parsed_request),
            timestamp: Timestamp::now(),
        };

        // Assert that ProcessRequestBody implements CommandLogic
        fn assert_command_logic<T: CommandLogic>() {}
        assert_command_logic::<ProcessRequestBody>();
    }

    #[test]
    fn test_request_state_machine_transitions() {
        let mut state = RequestState::default();
        let request_id = crate::domain::llm::RequestId::generate();
        let session_id = SessionId::generate();
        let timestamp = Timestamp::now();

        // Initial state should be NotStarted
        assert!(matches!(state.lifecycle, RequestLifecycle::NotStarted));
        assert!(!state.is_request_received());
        assert!(!state.is_request_forwarded());
        assert!(!state.is_response_received());
        assert!(!state.is_response_returned());

        // Transition to Received
        let event = DomainEvent::LlmRequestReceived {
            stream_id: session_stream(&session_id).unwrap(),
            request_id: request_id.clone(),
            session_id: session_id.clone(),
            model_version: crate::domain::llm::ModelVersion {
                provider: crate::domain::llm::LlmProvider::Other(
                    crate::domain::config_types::ProviderName::try_new("test".to_string())
                        .expect("test is a valid provider name in tests"),
                ),
                model_id: crate::domain::types::ModelId::try_new("test-model".to_string())
                    .expect("test-model is a valid model ID in tests"),
            },
            prompt: crate::domain::types::Prompt::try_new("test prompt".to_string())
                .expect("test prompt is valid in tests"),
            parameters: crate::domain::types::LlmParameters::new(Default::default()),
            received_at: timestamp,
        };
        state.apply(&event);

        assert!(matches!(state.lifecycle, RequestLifecycle::Received { .. }));
        assert!(state.is_request_received());
        assert!(!state.is_request_forwarded());

        // Transition to Forwarded
        let event = DomainEvent::LlmRequestStarted {
            stream_id: request_stream(&request_id).unwrap(),
            request_id: request_id.clone(),
            started_at: timestamp,
        };
        state.apply(&event);

        assert!(matches!(
            state.lifecycle,
            RequestLifecycle::Forwarded { .. }
        ));
        assert!(state.is_request_received());
        assert!(state.is_request_forwarded());
        assert!(!state.is_response_received());

        // Transition to ResponseReceived
        let event = DomainEvent::LlmResponseReceived {
            stream_id: request_stream(&request_id).unwrap(),
            request_id: request_id.clone(),
            response_text: crate::domain::types::ResponseText::try_new("response".to_string())
                .expect("response is valid text in tests"),
            metadata: crate::domain::llm::ResponseMetadata::default(),
            received_at: timestamp,
        };
        state.apply(&event);

        assert!(matches!(
            state.lifecycle,
            RequestLifecycle::ResponseReceived { .. }
        ));
        assert!(state.is_response_received());
        assert!(!state.is_response_returned());

        // Auto-transition to Completed (matching original behavior)
        state.apply(&DomainEvent::SessionTagged {
            stream_id: session_stream(&session_id).unwrap(),
            session_id: session_id.clone(),
            tag: crate::domain::types::Tag::try_new("test".to_string())
                .expect("test is a valid tag in tests"),
            tagged_at: timestamp,
        });

        assert!(matches!(
            state.lifecycle,
            RequestLifecycle::Completed { .. }
        ));
        assert!(state.is_response_returned());
    }

    #[test]
    fn test_request_state_machine_failure_transitions() {
        let mut state = RequestState::default();
        let request_id = crate::domain::llm::RequestId::generate();
        let session_id = SessionId::generate();
        let timestamp = Timestamp::now();

        // Set up initial state
        let event = DomainEvent::LlmRequestReceived {
            stream_id: session_stream(&session_id).unwrap(),
            request_id: request_id.clone(),
            session_id: session_id.clone(),
            model_version: crate::domain::llm::ModelVersion {
                provider: crate::domain::llm::LlmProvider::Other(
                    crate::domain::config_types::ProviderName::try_new("test".to_string())
                        .expect("test is a valid provider name in tests"),
                ),
                model_id: crate::domain::types::ModelId::try_new("test-model".to_string())
                    .expect("test-model is a valid model ID in tests"),
            },
            prompt: crate::domain::types::Prompt::try_new("test prompt".to_string())
                .expect("test prompt is valid in tests"),
            parameters: crate::domain::types::LlmParameters::new(Default::default()),
            received_at: timestamp,
        };
        state.apply(&event);

        // Transition to Failed from any state
        let event = DomainEvent::LlmRequestFailed {
            stream_id: request_stream(&request_id).unwrap(),
            request_id: request_id.clone(),
            error_message: crate::domain::types::ErrorMessage::try_new("test error".to_string())
                .expect("test error is a valid error message in tests"),
            failed_at: timestamp,
        };
        state.apply(&event);

        assert!(matches!(state.lifecycle, RequestLifecycle::Failed { .. }));

        // Test cancelled transition
        let mut state2 = RequestState::default();
        let event = DomainEvent::LlmRequestCancelled {
            stream_id: request_stream(&request_id).unwrap(),
            request_id: request_id.clone(),
            cancelled_at: timestamp,
        };
        state2.apply(&event);

        assert!(matches!(
            state2.lifecycle,
            RequestLifecycle::Failed { reason, .. } if reason == "Request cancelled"
        ));
    }

    #[test]
    fn test_request_state_prevents_illegal_transitions() {
        let mut state = RequestState::default();
        let request_id = crate::domain::llm::RequestId::generate();
        let timestamp = Timestamp::now();

        // Try to forward without receiving first - should stay in NotStarted
        let event = DomainEvent::LlmRequestStarted {
            stream_id: request_stream(&request_id).unwrap(),
            request_id: request_id.clone(),
            started_at: timestamp,
        };
        state.apply(&event);

        assert!(matches!(state.lifecycle, RequestLifecycle::NotStarted));

        // Try to receive response without forwarding - should stay in NotStarted
        let event = DomainEvent::LlmResponseReceived {
            stream_id: request_stream(&request_id).unwrap(),
            request_id: request_id.clone(),
            response_text: crate::domain::types::ResponseText::try_new("response".to_string())
                .expect("response is valid text in tests"),
            metadata: crate::domain::llm::ResponseMetadata::default(),
            received_at: timestamp,
        };
        state.apply(&event);

        assert!(matches!(state.lifecycle, RequestLifecycle::NotStarted));
    }

    #[tokio::test]
    async fn test_parsing_error_event_emission() {
        use eventcore::RetryPolicy;
        use eventcore_memory::InMemoryEventStore;

        let store = InMemoryEventStore::new();
        let session_id = SessionId::generate();
        let request_id = llm::RequestId::generate();

        // Create a command with invalid JSON body (parsed at adapter boundary)
        let uri = audit_types::RequestUri::try_new("/v1/chat/completions".to_string()).unwrap();
        let headers = audit_types::HttpHeaders::new();
        let parsed =
            crate::adapters::proxy_audit::parse_request_body(b"invalid json {", &uri, &headers);
        let command = RecordAuditEvent {
            session_stream: session_stream(&session_id).unwrap(),
            request_stream: request_stream(&request_id).unwrap(),
            request_id: request_id.clone(),
            session_id: session_id.clone(),
            audit_event: audit_types::AuditEventType::RequestReceived {
                method: audit_types::HttpMethod::try_new("POST".to_string()).unwrap(),
                uri,
                headers,
                body_size: audit_types::BodySize::from(15),
            },
            timestamp: Timestamp::now(),
            parsed_request: None,
        }
        .with_parsed_request(Some(parsed));

        // Execute the command
        let result = eventcore::execute(&store, command, RetryPolicy::default()).await;
        assert!(result.is_ok());

        // Read events from the request stream
        let request_stream = request_stream(&request_id).unwrap();
        let stream_data = store
            .read_stream::<DomainEvent>(request_stream)
            .await
            .unwrap();
        let events = stream_data;

        // Should have emitted a parsing error event
        let has_parsing_error = events
            .iter()
            .any(|e| matches!(e, DomainEvent::LlmRequestParsingFailed { .. }));

        assert!(has_parsing_error, "Should emit parsing error event");
    }

    #[tokio::test]
    async fn test_invalid_state_transition_events() {
        use eventcore::RetryPolicy;
        use eventcore_memory::InMemoryEventStore;

        let store = InMemoryEventStore::new();
        let session_id = SessionId::generate();
        let request_id = llm::RequestId::generate();
        let request_stream = request_stream(&request_id).unwrap();

        // Try to forward a request that hasn't been received
        let command = RecordAuditEvent {
            session_stream: session_stream(&session_id).unwrap(),
            request_stream: request_stream.clone(),
            request_id: request_id.clone(),
            session_id: session_id.clone(),
            audit_event: audit_types::AuditEventType::RequestForwarded {
                target_url: audit_types::TargetUrl::try_new(
                    "https://api.openai.com/v1/chat/completions".to_string(),
                )
                .unwrap(),
                start_time: Timestamp::now(),
            },
            timestamp: Timestamp::now(),
            parsed_request: None,
        };

        // Execute the command
        let result = eventcore::execute(&store, command, RetryPolicy::default()).await;
        assert!(result.is_ok());

        // Read events from the request stream
        let stream_data = store
            .read_stream::<DomainEvent>(request_stream)
            .await
            .unwrap();
        let events = stream_data;

        // Should have emitted an invalid state transition event
        let has_invalid_transition = events.iter().any(|e| matches!(
            e,
            DomainEvent::InvalidStateTransition { event_type, .. } if event_type == "RequestForwarded"
        ));

        assert!(
            has_invalid_transition,
            "Should emit invalid state transition event"
        );
    }

    #[tokio::test]
    async fn test_duplicate_request_received_event() {
        use eventcore::RetryPolicy;
        use eventcore_memory::InMemoryEventStore;

        let store = InMemoryEventStore::new();
        let session_id = SessionId::generate();
        let request_id = llm::RequestId::generate();
        let session_stream = session_stream(&session_id).unwrap();
        let request_stream = request_stream(&request_id).unwrap();

        // First request received
        let command = RecordAuditEvent {
            session_stream: session_stream.clone(),
            request_stream: request_stream.clone(),
            request_id: request_id.clone(),
            session_id: session_id.clone(),
            audit_event: audit_types::AuditEventType::RequestReceived {
                method: audit_types::HttpMethod::try_new("POST".to_string()).unwrap(),
                uri: audit_types::RequestUri::try_new("/v1/chat/completions".to_string()).unwrap(),
                headers: audit_types::HttpHeaders::new(),
                body_size: audit_types::BodySize::from(0),
            },
            timestamp: Timestamp::now(),
            parsed_request: None,
        };

        // Execute first time
        let result = eventcore::execute(&store, command.clone(), RetryPolicy::default()).await;
        assert!(result.is_ok());

        // Try to receive again (duplicate)
        let result = eventcore::execute(&store, command.clone(), RetryPolicy::default()).await;
        assert!(result.is_ok());

        // Read events from the request stream
        let stream_data = store
            .read_stream::<DomainEvent>(request_stream)
            .await
            .unwrap();
        let events = stream_data;

        // Should have emitted an invalid state transition event for the duplicate
        let has_duplicate_error = events.iter().any(|e| matches!(
            e,
            DomainEvent::InvalidStateTransition { reason, .. } if reason.as_ref().contains("already received")
        ));

        assert!(
            has_duplicate_error,
            "Should emit error for duplicate request received"
        );
    }

    #[tokio::test]
    async fn test_process_request_body_with_parsing_error() {
        use eventcore::RetryPolicy;
        use eventcore_memory::InMemoryEventStore;

        let store = InMemoryEventStore::new();
        let session_id = SessionId::generate();
        let request_id = llm::RequestId::generate();
        let request_stream = request_stream(&request_id).unwrap();

        // Create command with invalid JSON body (parsed at adapter boundary)
        let uri = audit_types::RequestUri::try_new("/v1/messages".to_string()).unwrap();
        let headers = audit_types::HttpHeaders::new();
        let parsed_request = crate::adapters::proxy_audit::parse_request_body(
            b"not valid json at all",
            &uri,
            &headers,
        );

        let command = ProcessRequestBody {
            session_stream: session_stream(&session_id).unwrap(),
            request_stream: request_stream.clone(),
            request_id: request_id.clone(),
            session_id,
            parsed_request: Some(parsed_request),
            timestamp: Timestamp::now(),
        };

        // Execute the command
        let result = eventcore::execute(&store, command, RetryPolicy::default()).await;
        assert!(result.is_ok());

        // Read events from the request stream
        let stream_data = store
            .read_stream::<DomainEvent>(request_stream)
            .await
            .unwrap();
        let events = stream_data;

        // Should have emitted a parsing error event
        let has_parsing_error = events
            .iter()
            .any(|e| matches!(e, DomainEvent::LlmRequestParsingFailed { .. }));

        assert!(
            has_parsing_error,
            "Should emit parsing error event from ProcessRequestBody"
        );
    }
}

/// Builder for RecordAuditEvent
#[derive(Debug, Default)]
pub struct RecordAuditEventBuilder {
    session_stream: Option<StreamId>,
    request_stream: Option<StreamId>,
    request_id: Option<llm::RequestId>,
    session_id: Option<SessionId>,
    audit_event: Option<audit_types::AuditEventType>,
    timestamp: Option<Timestamp>,
    parsed_request: Option<ParsedLlmRequestWithError>,
}

impl RecordAuditEventBuilder {
    pub fn session_stream(mut self, stream: StreamId) -> Self {
        self.session_stream = Some(stream);
        self
    }

    pub fn request_stream(mut self, stream: StreamId) -> Self {
        self.request_stream = Some(stream);
        self
    }

    pub fn request_id(mut self, id: llm::RequestId) -> Self {
        self.request_id = Some(id);
        self
    }

    pub fn session_id(mut self, id: SessionId) -> Self {
        self.session_id = Some(id);
        self
    }

    pub fn audit_event(mut self, event: audit_types::AuditEventType) -> Self {
        self.audit_event = Some(event);
        self
    }

    pub fn timestamp(mut self, timestamp: Timestamp) -> Self {
        self.timestamp = Some(timestamp);
        self
    }

    pub fn parsed_request(mut self, parsed: ParsedLlmRequestWithError) -> Self {
        self.parsed_request = Some(parsed);
        self
    }

    pub fn build(self) -> Result<RecordAuditEvent, AuditCommandError> {
        Ok(RecordAuditEvent {
            session_stream: self.session_stream.ok_or_else(|| {
                AuditCommandError::InvalidStreamId("Missing session stream".to_string())
            })?,
            request_stream: self.request_stream.ok_or_else(|| {
                AuditCommandError::InvalidStreamId("Missing request stream".to_string())
            })?,
            request_id: self
                .request_id
                .ok_or_else(|| AuditCommandError::InvalidField("Missing request ID".to_string()))?,
            session_id: self
                .session_id
                .ok_or_else(|| AuditCommandError::InvalidField("Missing session ID".to_string()))?,
            audit_event: self.audit_event.ok_or_else(|| {
                AuditCommandError::InvalidField("Missing audit event".to_string())
            })?,
            timestamp: self.timestamp.ok_or_else(|| {
                AuditCommandError::InvalidTimestamp("Missing timestamp".to_string())
            })?,
            parsed_request: self.parsed_request,
        })
    }
}

#[cfg(test)]
#[path = "audit_commands_tests.rs"]
mod audit_commands_tests;
