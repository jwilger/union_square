//! EventCore commands for audit events
//!
//! These commands map from the audit path events to EventCore commands,
//! enabling persistence of all proxy operations to the event store.

use async_trait::async_trait;
use eventcore::{
    emit, CommandLogic, CommandResult, ReadStreams, StoredEvent, StreamId, StreamResolver,
    StreamWrite,
};
use eventcore_macros::Command;
use serde::{Deserialize, Serialize};

use crate::domain::{events::DomainEvent, metrics::Timestamp, session::SessionId};
use crate::proxy::types::{AuditEvent, AuditEventType, Headers, HttpMethod, RequestId, RequestUri};

use super::llm_request_parser::{create_fallback_request, parse_llm_request, ParsedLlmRequest};
use crate::domain::types::ErrorMessage;

use std::fmt;

/// Wrapper for parsed LLM request that includes any parsing error
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedLlmRequestWithError {
    pub parsed: ParsedLlmRequest,
    pub error: Option<String>,
    pub raw_uri: String,
}

impl ParsedLlmRequestWithError {
    /// Create a new parsed request with error information
    pub const fn new(parsed: ParsedLlmRequest, error: Option<String>, raw_uri: String) -> Self {
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
    /// Request has been received from the client
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
            RequestLifecycle::Received { request_id, .. }
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
                DomainEvent::LlmRequestReceived {
                    request_id,
                    received_at,
                    ..
                },
            ) => Received {
                request_id: request_id.clone(),
                received_at: *received_at,
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

    /// Check if the request has been received
    pub const fn is_request_received(&self) -> bool {
        !matches!(self, RequestLifecycle::NotStarted)
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
    pub request_id: RequestId,
    pub session_id: SessionId,
    pub audit_event: AuditEventType,
    pub timestamp: Timestamp,
    /// Optional parsed request data (only used for RequestReceived events with body)
    #[serde(skip)]
    pub parsed_request: Option<ParsedLlmRequestWithError>,
}

impl TryFrom<&AuditEvent> for RecordAuditEvent {
    type Error = AuditCommandError;

    fn try_from(audit_event: &AuditEvent) -> Result<Self, Self::Error> {
        let session_stream =
            Self::session_stream_id(&SessionId::new(*audit_event.session_id.as_ref()))?;
        let request_stream = Self::request_stream_id(&audit_event.request_id)?;

        // Convert proxy SessionId to domain SessionId by extracting the inner UUID
        let session_id = SessionId::new(*audit_event.session_id.as_ref());

        // Convert chrono DateTime to domain Timestamp
        let timestamp = Timestamp::try_new(audit_event.timestamp).map_err(|e| {
            AuditCommandError::InvalidTimestamp(format!("Failed to convert timestamp: {e}"))
        })?;

        Ok(Self {
            session_stream,
            request_stream,
            request_id: audit_event.request_id,
            session_id,
            audit_event: audit_event.event_type.clone(),
            timestamp,
            parsed_request: None,
        })
    }
}

impl RecordAuditEvent {
    /// Create from an audit event (convenience method that delegates to TryFrom)
    pub fn from_audit_event(audit_event: &AuditEvent) -> Result<Self, AuditCommandError> {
        Self::try_from(audit_event)
    }

    /// Builder method to create a new instance
    pub fn builder() -> RecordAuditEventBuilder {
        RecordAuditEventBuilder::default()
    }

    /// Create stream ID for a session
    pub fn session_stream_id(session_id: &SessionId) -> Result<StreamId, AuditCommandError> {
        StreamId::try_new(format!("session-{}", session_id.clone().into_inner()))
            .map_err(|e| AuditCommandError::InvalidStreamId(e.to_string()))
    }

    /// Create stream ID for a request
    pub fn request_stream_id(request_id: &RequestId) -> Result<StreamId, AuditCommandError> {
        StreamId::try_new(format!("request-{request_id}"))
            .map_err(|e| AuditCommandError::InvalidStreamId(e.to_string()))
    }

    /// Set the parsed LLM request data from a request body
    pub fn with_body(mut self, body: &[u8]) -> Self {
        // Only parse body for RequestReceived events
        if let AuditEventType::RequestReceived { uri, headers, .. } = &self.audit_event {
            // Convert headers to the format expected by the parser
            let headers_vec = headers
                .as_vec()
                .iter()
                .map(|(name, value)| (name.as_ref().to_string(), value.as_ref().to_string()))
                .collect::<Vec<_>>();

            // Try to parse the request body
            match parse_llm_request(body, uri.as_ref(), &headers_vec) {
                Ok(parsed) => {
                    self.parsed_request = Some(ParsedLlmRequestWithError::new(
                        parsed,
                        None,
                        uri.as_ref().to_string(),
                    ));
                }
                Err(e) => {
                    // Log the error and use fallback
                    tracing::warn!(
                        "Failed to parse LLM request for {}: {}. Using fallback.",
                        self.request_id,
                        e
                    );
                    // Store both the fallback and the error for later emission
                    self.parsed_request = Some(ParsedLlmRequestWithError::new(
                        create_fallback_request(&e),
                        Some(e.to_string()),
                        uri.as_ref().to_string(),
                    ));
                }
            }
        }
        self
    }
}

/// Error messages as constants for compile-time validation
mod error_messages {
    use crate::domain::types::ErrorMessage;

    pub const REQUEST_ALREADY_RECEIVED: &str = "Request already received";
    pub const REQUEST_ALREADY_FORWARDED: &str = "Request already forwarded";
    pub const RESPONSE_ALREADY_RECEIVED: &str = "Response already received";
    pub const CANNOT_FORWARD_UNRECEIVED: &str = "Cannot forward request that hasn't been received";
    pub const CANNOT_RECEIVE_RESPONSE_UNFORWARDED: &str =
        "Cannot receive response for request that hasn't been forwarded";
    pub const AUDIT_EVENT_NOT_IMPLEMENTED: &str = "Audit event type not yet implemented";
    pub const UNKNOWN_PARSING_ERROR: &str = "Unknown parsing error";
    pub const REQUEST_CANCELLED: &str = "Request cancelled";

    /// Create an ErrorMessage from a static string - this is safe because we control all the strings
    #[inline]
    pub fn static_error(msg: &'static str) -> ErrorMessage {
        ErrorMessage::try_new(msg.to_string()).expect("static error message should be valid")
    }
}

/// Pure functions to transform audit events into domain events
mod transformers {
    use super::*;

    /// Transform RequestReceived audit event to domain event
    pub fn request_received_to_domain(
        request_id: RequestId,
        session_id: SessionId,
        timestamp: Timestamp,
        parsed_request: Option<&ParsedLlmRequest>,
    ) -> Result<DomainEvent, eventcore::CommandError> {
        let (model_version, prompt, parameters) = if let Some(parsed) = parsed_request {
            (
                parsed.model_version.clone(),
                parsed.prompt.clone(),
                parsed.parameters.clone(),
            )
        } else {
            create_fallback_llm_data(request_id)?
        };

        Ok(DomainEvent::LlmRequestReceived {
            request_id: crate::domain::llm::RequestId::new(*request_id.as_ref()),
            session_id,
            model_version,
            prompt,
            parameters,
            received_at: timestamp,
        })
    }

    /// Transform RequestForwarded audit event to domain event
    pub fn request_forwarded_to_domain(request_id: RequestId, timestamp: Timestamp) -> DomainEvent {
        DomainEvent::LlmRequestStarted {
            request_id: crate::domain::llm::RequestId::new(*request_id.as_ref()),
            started_at: timestamp,
        }
    }

    /// Transform ResponseReceived audit event to domain event
    pub fn response_received_to_domain(
        request_id: RequestId,
        timestamp: Timestamp,
    ) -> Result<DomainEvent, eventcore::CommandError> {
        // For now, we don't have the response body here
        // TODO: Implement response body parsing similar to request parsing
        let response_text = crate::domain::types::ResponseText::try_new(
            "Response body parsing not yet implemented".to_string(),
        )
        .map_err(|e| {
            eventcore::CommandError::Internal(format!(
                "Failed to create response text placeholder: {e}"
            ))
        })?;

        let metadata = crate::domain::llm::ResponseMetadata::default();

        Ok(DomainEvent::LlmResponseReceived {
            request_id: crate::domain::llm::RequestId::new(*request_id.as_ref()),
            response_text,
            metadata,
            received_at: timestamp,
        })
    }

    /// Helper to create error message safely
    #[allow(dead_code)]
    pub fn create_error_message(msg: &str) -> ErrorMessage {
        ErrorMessage::try_new(msg.to_string())
            .expect("error message creation should not fail for valid strings")
    }

    /// Create fallback LLM data when parsing fails
    fn create_fallback_llm_data(
        request_id: RequestId,
    ) -> Result<
        (
            crate::domain::llm::ModelVersion,
            crate::domain::types::Prompt,
            crate::domain::types::LlmParameters,
        ),
        eventcore::CommandError,
    > {
        tracing::warn!(
            "No parsed LLM data available for request {}. Using defaults.",
            request_id
        );

        // Create safe fallback values that should never fail validation
        let fallback_provider = crate::domain::config_types::ProviderName::try_new(
            "unknown".to_string(),
        )
        .map_err(|e| {
            eventcore::CommandError::Internal(format!(
                "Failed to create fallback provider name: {e}"
            ))
        })?;

        let fallback_model_id = crate::domain::types::ModelId::try_new("unknown-model".to_string())
            .map_err(|e| {
                eventcore::CommandError::Internal(format!(
                    "Failed to create fallback model ID: {e}"
                ))
            })?;

        let fallback_prompt =
            crate::domain::types::Prompt::try_new("Request body not available".to_string())
                .map_err(|e| {
                    eventcore::CommandError::Internal(format!(
                        "Failed to create fallback prompt: {e}"
                    ))
                })?;

        Ok((
            crate::domain::llm::ModelVersion {
                provider: crate::domain::llm::LlmProvider::Other(fallback_provider),
                model_id: fallback_model_id,
            },
            fallback_prompt,
            crate::domain::types::LlmParameters::new(Default::default()),
        ))
    }
}

#[async_trait]
impl CommandLogic for RecordAuditEvent {
    type State = RequestState;
    type Event = DomainEvent;

    fn apply(&self, state: &mut Self::State, event: &StoredEvent<Self::Event>) {
        state.apply(&event.payload);
    }

    async fn handle(
        &self,
        _read_streams: ReadStreams<Self::StreamSet>,
        state: Self::State,
        _stream_resolver: &mut StreamResolver,
    ) -> CommandResult<Vec<StreamWrite<Self::StreamSet, Self::Event>>> {
        let mut events = Vec::new();

        use AuditEventType::*;

        // Determine which event to emit based on audit event type and current state
        match &self.audit_event {
            RequestReceived { .. } => {
                if !state.is_request_received() {
                    // Emit the request received event
                    let event = transformers::request_received_to_domain(
                        self.request_id,
                        self.session_id.clone(),
                        self.timestamp,
                        self.parsed_request.as_ref().map(|p| &p.parsed),
                    )?;

                    emit!(events, &_read_streams, self.session_stream.clone(), event);

                    // If there was a parsing error, emit an error event
                    if let Some(_parsed_with_error) = &self.parsed_request {
                        if let Some(ParsedLlmRequestWithError {
                            error: Some(error_msg),
                            raw_uri,
                            ..
                        }) = &self.parsed_request
                        {
                            let error_message = ErrorMessage::try_new(error_msg.clone())
                                .unwrap_or_else(|_| {
                                    error_messages::static_error(
                                        error_messages::UNKNOWN_PARSING_ERROR,
                                    )
                                });

                            emit!(
                                events,
                                &_read_streams,
                                self.request_stream.clone(),
                                DomainEvent::LlmRequestParsingFailed {
                                    request_id: crate::domain::llm::RequestId::new(
                                        *self.request_id.as_ref()
                                    ),
                                    session_id: self.session_id.clone(),
                                    parsing_error: error_message,
                                    raw_uri: raw_uri.clone(),
                                    occurred_at: self.timestamp,
                                }
                            );
                        }
                    }
                } else {
                    // Invalid state transition - request already received
                    emit!(
                        events,
                        &_read_streams,
                        self.request_stream.clone(),
                        DomainEvent::InvalidStateTransition {
                            request_id: crate::domain::llm::RequestId::new(
                                *self.request_id.as_ref()
                            ),
                            session_id: self.session_id.clone(),
                            from_state: state.to_string(),
                            event_type: "RequestReceived".to_string(),
                            reason: error_messages::static_error(
                                error_messages::REQUEST_ALREADY_RECEIVED
                            ),
                            occurred_at: self.timestamp,
                        }
                    );
                }
            }
            RequestForwarded { start_time, .. } => {
                if !state.is_request_forwarded() {
                    // Check if we're in a valid state to forward
                    if !state.is_request_received() {
                        // Invalid transition - trying to forward before receiving
                        emit!(
                            events,
                            &_read_streams,
                            self.request_stream.clone(),
                            DomainEvent::InvalidStateTransition {
                                request_id: crate::domain::llm::RequestId::new(
                                    *self.request_id.as_ref()
                                ),
                                session_id: self.session_id.clone(),
                                from_state: state.to_string(),
                                event_type: "RequestForwarded".to_string(),
                                reason: error_messages::static_error(
                                    error_messages::CANNOT_FORWARD_UNRECEIVED
                                ),
                                occurred_at: self.timestamp,
                            }
                        );
                    } else {
                        let timestamp = Timestamp::try_new(*start_time).map_err(|e| {
                            eventcore::CommandError::Internal(format!(
                                "Failed to convert start_time: {e}"
                            ))
                        })?;

                        let event =
                            transformers::request_forwarded_to_domain(self.request_id, timestamp);

                        emit!(events, &_read_streams, self.request_stream.clone(), event);
                    }
                } else {
                    // Invalid state transition - request already forwarded
                    emit!(
                        events,
                        &_read_streams,
                        self.request_stream.clone(),
                        DomainEvent::InvalidStateTransition {
                            request_id: crate::domain::llm::RequestId::new(
                                *self.request_id.as_ref()
                            ),
                            session_id: self.session_id.clone(),
                            from_state: state.to_string(),
                            event_type: "RequestForwarded".to_string(),
                            reason: error_messages::static_error(
                                error_messages::REQUEST_ALREADY_FORWARDED
                            ),
                            occurred_at: self.timestamp,
                        }
                    );
                }
            }
            ResponseReceived { .. } => {
                // Only emit response if request has been forwarded and response not yet received
                if state.is_request_forwarded() && !state.is_response_received() {
                    let event =
                        transformers::response_received_to_domain(self.request_id, self.timestamp)?;

                    emit!(events, &_read_streams, self.request_stream.clone(), event);
                } else if !state.is_request_forwarded() {
                    // Invalid transition - response received before forwarding
                    emit!(
                        events,
                        &_read_streams,
                        self.request_stream.clone(),
                        DomainEvent::InvalidStateTransition {
                            request_id: crate::domain::llm::RequestId::new(
                                *self.request_id.as_ref()
                            ),
                            session_id: self.session_id.clone(),
                            from_state: state.to_string(),
                            event_type: "ResponseReceived".to_string(),
                            reason: error_messages::static_error(
                                error_messages::CANNOT_RECEIVE_RESPONSE_UNFORWARDED
                            ),
                            occurred_at: self.timestamp,
                        }
                    );
                } else {
                    // Response already received
                    emit!(
                        events,
                        &_read_streams,
                        self.request_stream.clone(),
                        DomainEvent::InvalidStateTransition {
                            request_id: crate::domain::llm::RequestId::new(
                                *self.request_id.as_ref()
                            ),
                            session_id: self.session_id.clone(),
                            from_state: state.to_string(),
                            event_type: "ResponseReceived".to_string(),
                            reason: error_messages::static_error(
                                error_messages::RESPONSE_ALREADY_RECEIVED
                            ),
                            occurred_at: self.timestamp,
                        }
                    );
                }
            }
            ResponseReturned { .. } => {
                // For now, we don't emit any specific event for response returned
                // The LlmResponseReceived event already captures the completion
            }
            _ => {
                // Other audit event types not yet handled
                tracing::debug!("Unhandled audit event type: {:?}", self.audit_event);

                // Emit an error event for unhandled audit event types
                let event_type_str = match &self.audit_event {
                    RequestBody { .. } => "RequestBody",
                    ResponseBody { .. } => "ResponseBody",
                    RequestChunk { .. } => "RequestChunk",
                    ResponseChunk { .. } => "ResponseChunk",
                    Error { .. } => "Error",
                    _ => "Unknown",
                };

                emit!(
                    events,
                    &_read_streams,
                    self.request_stream.clone(),
                    DomainEvent::AuditEventProcessingFailed {
                        request_id: crate::domain::llm::RequestId::new(*self.request_id.as_ref()),
                        session_id: self.session_id.clone(),
                        event_type: event_type_str.to_string(),
                        error_message: error_messages::static_error(
                            error_messages::AUDIT_EVENT_NOT_IMPLEMENTED
                        ),
                        occurred_at: self.timestamp,
                    }
                );
            }
        }

        Ok(events)
    }
}

// The redundant command structs have been removed in favor of the unified RecordAuditEvent command

/// Command to process buffered request body and emit LlmRequestReceived event
#[derive(Debug, Clone, Serialize, Deserialize, Command)]
pub struct ProcessRequestBody {
    #[stream]
    pub session_stream: StreamId,
    #[stream]
    pub request_stream: StreamId,
    pub request_id: RequestId,
    pub session_id: SessionId,
    pub method: HttpMethod,
    pub uri: RequestUri,
    pub headers: Headers,
    pub body: Vec<u8>,
    pub timestamp: Timestamp,
}

#[async_trait]
impl CommandLogic for ProcessRequestBody {
    type State = RequestState;
    type Event = DomainEvent;

    fn apply(&self, state: &mut Self::State, event: &StoredEvent<Self::Event>) {
        state.apply(&event.payload);
    }

    async fn handle(
        &self,
        _read_streams: ReadStreams<Self::StreamSet>,
        state: Self::State,
        _stream_resolver: &mut StreamResolver,
    ) -> CommandResult<Vec<StreamWrite<Self::StreamSet, Self::Event>>> {
        let mut events = Vec::new();

        // Check if we've already recorded this request
        if state.is_request_received() {
            return Ok(events);
        }

        // Convert headers for parser
        let headers_vec = self
            .headers
            .as_vec()
            .iter()
            .map(|(name, value)| (name.as_ref().to_string(), value.as_ref().to_string()))
            .collect::<Vec<_>>();

        // Parse the LLM request
        let parsed_result = parse_llm_request(&self.body, self.uri.as_ref(), &headers_vec);

        let (model_version, prompt, parameters, parsing_error) = parsed_result
            .map(|parsed| (parsed.model_version, parsed.prompt, parsed.parameters, None))
            .unwrap_or_else(|e| {
                tracing::warn!(
                    "Failed to parse LLM request {}: {}. Using fallback.",
                    self.request_id,
                    e
                );
                let fallback = create_fallback_request(&e);
                (
                    fallback.model_version,
                    fallback.prompt,
                    fallback.parameters,
                    Some(e.to_string()),
                )
            });

        emit!(
            events,
            &_read_streams,
            self.session_stream.clone(),
            DomainEvent::LlmRequestReceived {
                request_id: crate::domain::llm::RequestId::new(*self.request_id.as_ref()),
                session_id: self.session_id.clone(),
                model_version,
                prompt,
                parameters,
                received_at: self.timestamp,
            }
        );

        // If there was a parsing error, emit an error event
        if let Some(error_msg) = parsing_error {
            let error_message = ErrorMessage::try_new(error_msg).unwrap_or_else(|_| {
                error_messages::static_error(error_messages::UNKNOWN_PARSING_ERROR)
            });

            emit!(
                events,
                &_read_streams,
                self.request_stream.clone(),
                DomainEvent::LlmRequestParsingFailed {
                    request_id: crate::domain::llm::RequestId::new(*self.request_id.as_ref()),
                    session_id: self.session_id.clone(),
                    parsing_error: error_message,
                    raw_uri: self.uri.as_ref().to_string(),
                    occurred_at: self.timestamp,
                }
            );
        }

        Ok(events)
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
}

impl fmt::Display for RequestLifecycle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotStarted => write!(f, "NotStarted"),
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
    use crate::proxy::types::{
        BodySize, DurationMillis, HttpStatusCode, SessionId as ProxySessionId, TargetUrl,
    };
    use chrono::Utc;

    #[test]
    fn test_audit_event_to_unified_command() {
        let audit_event = AuditEvent {
            request_id: RequestId::new(),
            session_id: ProxySessionId::new(),
            timestamp: Utc::now(),
            event_type: AuditEventType::RequestReceived {
                method: HttpMethod::try_new("GET".to_string()).unwrap(),
                uri: RequestUri::try_new("/test".to_string()).unwrap(),
                headers: Headers::new(),
                body_size: BodySize::from(0),
            },
        };

        let command = RecordAuditEvent::from_audit_event(&audit_event);
        assert!(command.is_ok());
        let cmd = command.unwrap();
        assert_eq!(cmd.request_id, audit_event.request_id);
        // Can't directly compare different SessionId types, so check they wrap the same UUID
        assert_eq!(
            cmd.session_id.into_inner(),
            *audit_event.session_id.as_ref()
        );

        // Verify the audit event type was preserved
        assert!(matches!(
            cmd.audit_event,
            AuditEventType::RequestReceived { .. }
        ));
    }

    #[test]
    fn test_unified_command_handles_all_event_types() {
        // Test that the unified command can handle any audit event type
        let event_types = vec![
            AuditEventType::RequestReceived {
                method: HttpMethod::try_new("GET".to_string()).unwrap(),
                uri: RequestUri::try_new("/test".to_string()).unwrap(),
                headers: Headers::new(),
                body_size: BodySize::from(0),
            },
            AuditEventType::RequestForwarded {
                target_url: TargetUrl::try_new(
                    "https://api.openai.com/v1/chat/completions".to_string(),
                )
                .unwrap(),
                start_time: Utc::now(),
            },
            AuditEventType::ResponseReceived {
                status: HttpStatusCode::try_new(200).unwrap(),
                headers: Headers::new(),
                body_size: BodySize::from(1024),
                duration_ms: DurationMillis::from(100),
            },
            AuditEventType::ResponseReturned {
                duration_ms: DurationMillis::from(200),
            },
        ];

        for event_type in event_types {
            let audit_event = AuditEvent {
                request_id: RequestId::new(),
                session_id: ProxySessionId::new(),
                timestamp: Utc::now(),
                event_type: event_type.clone(),
            };

            let command = RecordAuditEvent::from_audit_event(&audit_event);
            assert!(command.is_ok(), "Failed for event type: {event_type:?}");
        }
    }

    #[tokio::test]
    async fn test_unified_command_logic() {
        // Create a command
        let session_id = SessionId::generate();
        let request_id = RequestId::new();
        let _command = RecordAuditEvent {
            session_stream: StreamId::try_new(format!(
                "session-{}",
                session_id.clone().into_inner()
            ))
            .unwrap(),
            request_stream: StreamId::try_new(format!("request-{request_id}")).unwrap(),
            request_id,
            session_id,
            audit_event: AuditEventType::RequestReceived {
                method: HttpMethod::try_new("POST".to_string()).unwrap(),
                uri: RequestUri::try_new("/api/test".to_string()).unwrap(),
                headers: Headers::new(),
                body_size: BodySize::from(1024),
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
        let request_id = RequestId::new();

        // Create a sample OpenAI request body
        let body = serde_json::json!({
            "model": "gpt-4",
            "messages": [
                {"role": "user", "content": "Hello, world!"}
            ],
            "temperature": 0.7
        });

        let command = RecordAuditEvent {
            session_stream: StreamId::try_new(format!(
                "session-{}",
                session_id.clone().into_inner()
            ))
            .unwrap(),
            request_stream: StreamId::try_new(format!("request-{request_id}")).unwrap(),
            request_id,
            session_id,
            audit_event: AuditEventType::RequestReceived {
                method: HttpMethod::try_new("POST".to_string()).unwrap(),
                uri: RequestUri::try_new("/v1/chat/completions".to_string()).unwrap(),
                headers: Headers::new(),
                body_size: BodySize::from(body.to_string().len()),
            },
            timestamp: Timestamp::now(),
            parsed_request: None,
        };

        // Apply body parsing
        let command_with_body = command.with_body(body.to_string().as_bytes());

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
        let request_id = RequestId::new();

        let body = serde_json::json!({
            "model": "claude-3-opus-20240229",
            "messages": [
                {"role": "user", "content": "What is 2+2?"}
            ],
            "max_tokens": 100
        });

        let _command = ProcessRequestBody {
            session_stream: StreamId::try_new(format!(
                "session-{}",
                session_id.clone().into_inner()
            ))
            .unwrap(),
            request_stream: StreamId::try_new(format!("request-{request_id}")).unwrap(),
            request_id,
            session_id,
            method: HttpMethod::try_new("POST".to_string()).unwrap(),
            uri: RequestUri::try_new("/v1/messages".to_string()).unwrap(),
            headers: Headers::new(),
            body: body.to_string().as_bytes().to_vec(),
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
            request_id: request_id.clone(),
            started_at: timestamp,
        };
        state.apply(&event);

        assert!(matches!(state.lifecycle, RequestLifecycle::NotStarted));

        // Try to receive response without forwarding - should stay in NotStarted
        let event = DomainEvent::LlmResponseReceived {
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
        use eventcore::{CommandExecutor, EventStore, ExecutionOptions, ReadOptions};
        use eventcore_memory::InMemoryEventStore;

        let event_store = InMemoryEventStore::new();
        let executor = CommandExecutor::new(event_store.clone());
        let session_id = SessionId::generate();
        let request_id = RequestId::new();

        // Create a command with invalid JSON body
        let command = RecordAuditEvent {
            session_stream: StreamId::try_new(format!(
                "session-{}",
                session_id.clone().into_inner()
            ))
            .unwrap(),
            request_stream: StreamId::try_new(format!("request-{request_id}")).unwrap(),
            request_id,
            session_id: session_id.clone(),
            audit_event: AuditEventType::RequestReceived {
                method: HttpMethod::try_new("POST".to_string()).unwrap(),
                uri: RequestUri::try_new("/v1/chat/completions".to_string()).unwrap(),
                headers: Headers::new(),
                body_size: BodySize::from(15),
            },
            timestamp: Timestamp::now(),
            parsed_request: None,
        }
        .with_body(b"invalid json {"); // Invalid JSON

        // Execute the command
        let result = executor.execute(command, ExecutionOptions::default()).await;
        assert!(result.is_ok());

        // Read events from the request stream
        let request_stream = StreamId::try_new(format!("request-{request_id}")).unwrap();
        let stream_data = event_store
            .read_streams(&[request_stream], &ReadOptions::default())
            .await
            .unwrap();
        let events = stream_data.events;

        // Should have emitted a parsing error event
        let has_parsing_error = events
            .iter()
            .any(|e| matches!(&e.payload, DomainEvent::LlmRequestParsingFailed { .. }));

        assert!(has_parsing_error, "Should emit parsing error event");
    }

    #[tokio::test]
    async fn test_invalid_state_transition_events() {
        use eventcore::{CommandExecutor, EventStore, ExecutionOptions, ReadOptions};
        use eventcore_memory::InMemoryEventStore;

        let event_store = InMemoryEventStore::new();
        let executor = CommandExecutor::new(event_store.clone());
        let session_id = SessionId::generate();
        let request_id = RequestId::new();
        let request_stream = StreamId::try_new(format!("request-{request_id}")).unwrap();

        // Try to forward a request that hasn't been received
        let command = RecordAuditEvent {
            session_stream: StreamId::try_new(format!(
                "session-{}",
                session_id.clone().into_inner()
            ))
            .unwrap(),
            request_stream: request_stream.clone(),
            request_id,
            session_id: session_id.clone(),
            audit_event: AuditEventType::RequestForwarded {
                target_url: TargetUrl::try_new(
                    "https://api.openai.com/v1/chat/completions".to_string(),
                )
                .unwrap(),
                start_time: Utc::now(),
            },
            timestamp: Timestamp::now(),
            parsed_request: None,
        };

        // Execute the command
        let result = executor.execute(command, ExecutionOptions::default()).await;
        assert!(result.is_ok());

        // Read events from the request stream
        let stream_data = event_store
            .read_streams(&[request_stream], &ReadOptions::default())
            .await
            .unwrap();
        let events = stream_data.events;

        // Should have emitted an invalid state transition event
        let has_invalid_transition = events.iter().any(|e| matches!(
            &e.payload,
            DomainEvent::InvalidStateTransition { event_type, .. } if event_type == "RequestForwarded"
        ));

        assert!(
            has_invalid_transition,
            "Should emit invalid state transition event"
        );
    }

    #[tokio::test]
    async fn test_duplicate_request_received_event() {
        use eventcore::{CommandExecutor, EventStore, ExecutionOptions, ReadOptions};
        use eventcore_memory::InMemoryEventStore;

        let event_store = InMemoryEventStore::new();
        let executor = CommandExecutor::new(event_store.clone());
        let session_id = SessionId::generate();
        let request_id = RequestId::new();
        let session_stream =
            StreamId::try_new(format!("session-{}", session_id.clone().into_inner())).unwrap();
        let request_stream = StreamId::try_new(format!("request-{request_id}")).unwrap();

        // First request received
        let command = RecordAuditEvent {
            session_stream: session_stream.clone(),
            request_stream: request_stream.clone(),
            request_id,
            session_id: session_id.clone(),
            audit_event: AuditEventType::RequestReceived {
                method: HttpMethod::try_new("POST".to_string()).unwrap(),
                uri: RequestUri::try_new("/v1/chat/completions".to_string()).unwrap(),
                headers: Headers::new(),
                body_size: BodySize::from(0),
            },
            timestamp: Timestamp::now(),
            parsed_request: None,
        };

        // Execute first time
        let result = executor
            .execute(command.clone(), ExecutionOptions::default())
            .await;
        assert!(result.is_ok());

        // Try to receive again (duplicate)
        let result = executor
            .execute(command.clone(), ExecutionOptions::default())
            .await;
        assert!(result.is_ok());

        // Read events from the request stream
        let stream_data = event_store
            .read_streams(&[request_stream], &ReadOptions::default())
            .await
            .unwrap();
        let events = stream_data.events;

        // Should have emitted an invalid state transition event for the duplicate
        let has_duplicate_error = events.iter().any(|e| matches!(
            &e.payload,
            DomainEvent::InvalidStateTransition { reason, .. } if reason.as_ref().contains("already received")
        ));

        assert!(
            has_duplicate_error,
            "Should emit error for duplicate request received"
        );
    }

    #[tokio::test]
    async fn test_unhandled_audit_event_error() {
        use eventcore::{CommandExecutor, EventStore, ExecutionOptions, ReadOptions};
        use eventcore_memory::InMemoryEventStore;

        let event_store = InMemoryEventStore::new();
        let executor = CommandExecutor::new(event_store.clone());
        let session_id = SessionId::generate();
        let request_id = RequestId::new();
        let request_stream = StreamId::try_new(format!("request-{request_id}")).unwrap();

        // Create a command with an unhandled event type
        let command = RecordAuditEvent {
            session_stream: StreamId::try_new(format!(
                "session-{}",
                session_id.clone().into_inner()
            ))
            .unwrap(),
            request_stream: request_stream.clone(),
            request_id,
            session_id: session_id.clone(),
            audit_event: AuditEventType::RequestBody {
                content: vec![1, 2, 3],
                truncated: false,
            },
            timestamp: Timestamp::now(),
            parsed_request: None,
        };

        // Execute the command
        let result = executor.execute(command, ExecutionOptions::default()).await;
        assert!(result.is_ok());

        // Read events from the request stream
        let stream_data = event_store
            .read_streams(&[request_stream], &ReadOptions::default())
            .await
            .unwrap();
        let events = stream_data.events;

        // Should have emitted an audit event processing failed event
        let has_processing_error = events.iter().any(|e| matches!(
            &e.payload,
            DomainEvent::AuditEventProcessingFailed { event_type, .. } if event_type == "RequestBody"
        ));

        assert!(
            has_processing_error,
            "Should emit error for unhandled audit event type"
        );
    }

    #[tokio::test]
    async fn test_process_request_body_with_parsing_error() {
        use eventcore::{CommandExecutor, EventStore, ExecutionOptions, ReadOptions};
        use eventcore_memory::InMemoryEventStore;

        let event_store = InMemoryEventStore::new();
        let executor = CommandExecutor::new(event_store.clone());
        let session_id = SessionId::generate();
        let request_id = RequestId::new();
        let request_stream = StreamId::try_new(format!("request-{request_id}")).unwrap();

        // Create command with invalid JSON body
        let command = ProcessRequestBody {
            session_stream: StreamId::try_new(format!(
                "session-{}",
                session_id.clone().into_inner()
            ))
            .unwrap(),
            request_stream: request_stream.clone(),
            request_id,
            session_id,
            method: HttpMethod::try_new("POST".to_string()).unwrap(),
            uri: RequestUri::try_new("/v1/messages".to_string()).unwrap(),
            headers: Headers::new(),
            body: b"not valid json at all".to_vec(),
            timestamp: Timestamp::now(),
        };

        // Execute the command
        let result = executor.execute(command, ExecutionOptions::default()).await;
        assert!(result.is_ok());

        // Read events from the request stream
        let stream_data = event_store
            .read_streams(&[request_stream], &ReadOptions::default())
            .await
            .unwrap();
        let events = stream_data.events;

        // Should have emitted a parsing error event
        let has_parsing_error = events
            .iter()
            .any(|e| matches!(&e.payload, DomainEvent::LlmRequestParsingFailed { .. }));

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
    request_id: Option<RequestId>,
    session_id: Option<SessionId>,
    audit_event: Option<AuditEventType>,
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

    pub fn request_id(mut self, id: RequestId) -> Self {
        self.request_id = Some(id);
        self
    }

    pub fn session_id(mut self, id: SessionId) -> Self {
        self.session_id = Some(id);
        self
    }

    pub fn audit_event(mut self, event: AuditEventType) -> Self {
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
            request_id: self.request_id.ok_or_else(|| {
                AuditCommandError::InvalidStreamId("Missing request ID".to_string())
            })?,
            session_id: self.session_id.ok_or_else(|| {
                AuditCommandError::InvalidStreamId("Missing session ID".to_string())
            })?,
            audit_event: self.audit_event.ok_or_else(|| {
                AuditCommandError::InvalidStreamId("Missing audit event".to_string())
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
