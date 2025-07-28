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
use crate::proxy::types::{
    AuditEvent, AuditEventType, BodySize, DurationMillis, Headers, HttpMethod, HttpStatusCode,
    RequestId, RequestUri, TargetUrl,
};

use super::llm_request_parser::{create_fallback_request, parse_llm_request, ParsedLlmRequest};

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
#[derive(Debug, Clone)]
pub struct RequestState {
    pub lifecycle: RequestLifecycle,
}

impl Default for RequestState {
    fn default() -> Self {
        Self {
            lifecycle: RequestLifecycle::NotStarted,
        }
    }
}

impl RequestState {
    /// Apply an event to update the state
    /// This enforces valid state transitions
    pub fn apply(&mut self, event: &DomainEvent) {
        use RequestLifecycle::*;

        self.lifecycle = match (&self.lifecycle, event) {
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
                reason: "Request cancelled".to_string(),
            },

            // No state change for other events or invalid transitions
            (current_state, _) => current_state.clone(),
        };
    }

    /// Check if the request has been received
    pub fn is_request_received(&self) -> bool {
        !matches!(self.lifecycle, RequestLifecycle::NotStarted)
    }

    /// Check if the request has been forwarded
    pub fn is_request_forwarded(&self) -> bool {
        matches!(
            self.lifecycle,
            RequestLifecycle::Forwarded { .. }
                | RequestLifecycle::ResponseReceived { .. }
                | RequestLifecycle::Completed { .. }
        )
    }

    /// Check if the response has been received
    pub fn is_response_received(&self) -> bool {
        matches!(
            self.lifecycle,
            RequestLifecycle::ResponseReceived { .. } | RequestLifecycle::Completed { .. }
        )
    }

    /// Check if the response has been returned (completed)
    pub fn is_response_returned(&self) -> bool {
        matches!(self.lifecycle, RequestLifecycle::Completed { .. })
    }
}

/// Command to record that an LLM request was received
#[derive(Debug, Clone, Serialize, Deserialize, Command)]
pub struct RecordRequestReceived {
    #[stream]
    pub session_stream: StreamId,
    #[stream]
    pub request_stream: StreamId,
    pub request_id: RequestId,
    pub session_id: SessionId,
    pub method: HttpMethod,
    pub uri: RequestUri,
    pub headers: Headers,
    pub body_size: BodySize,
    pub timestamp: Timestamp,
    /// Parsed LLM request data (if available)
    #[serde(skip)]
    pub parsed_request: Option<ParsedLlmRequest>,
}

impl RecordRequestReceived {
    /// Create from an audit event
    pub fn from_audit_event(audit_event: &AuditEvent) -> Result<Self, AuditCommandError> {
        match &audit_event.event_type {
            AuditEventType::RequestReceived {
                method,
                uri,
                headers,
                body_size,
            } => {
                let session_stream =
                    StreamId::try_new(format!("session-{}", audit_event.session_id))
                        .map_err(|e| AuditCommandError::InvalidStreamId(e.to_string()))?;
                let request_stream =
                    StreamId::try_new(format!("request-{}", audit_event.request_id))
                        .map_err(|e| AuditCommandError::InvalidStreamId(e.to_string()))?;

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
                    method: method.clone(),
                    uri: uri.clone(),
                    headers: headers.clone(),
                    body_size: *body_size,
                    timestamp,
                    parsed_request: None, // Will be set later when body is available
                })
            }
            _ => Err(AuditCommandError::WrongEventType {
                expected: "RequestReceived".to_string(),
                actual: format!("{:?}", audit_event.event_type),
            }),
        }
    }

    /// Set the parsed LLM request data from a request body
    pub fn with_body(mut self, body: &[u8]) -> Self {
        // Convert headers to the format expected by the parser
        let headers_vec: Vec<(String, String)> = self
            .headers
            .as_vec()
            .iter()
            .map(|(name, value)| (name.as_ref().to_string(), value.as_ref().to_string()))
            .collect();

        // Try to parse the request body
        match parse_llm_request(body, self.uri.as_ref(), &headers_vec) {
            Ok(parsed) => {
                self.parsed_request = Some(parsed);
            }
            Err(e) => {
                // Log the error and use fallback
                tracing::warn!(
                    "Failed to parse LLM request for {}: {}. Using fallback.",
                    self.request_id,
                    e
                );
                self.parsed_request = Some(create_fallback_request(&e));
            }
        }

        self
    }
}

#[async_trait]
impl CommandLogic for RecordRequestReceived {
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
            // Already recorded, skip
            return Ok(events);
        }

        // Create the LlmRequestReceived event using parsed data or defaults
        let (model_version, prompt, parameters) = if let Some(parsed) = &self.parsed_request {
            (
                parsed.model_version.clone(),
                parsed.prompt.clone(),
                parsed.parameters.clone(),
            )
        } else {
            // Fallback when no parsed data is available
            tracing::warn!(
                "No parsed LLM data available for request {}. Using defaults.",
                self.request_id
            );

            // Create safe fallback values that should never fail validation
            let fallback_provider =
                crate::domain::config_types::ProviderName::try_new("unknown".to_string())
                    .map_err(|e| {
                        tracing::error!("Failed to create fallback provider name: {}", e);
                    })
                    .ok();

            let fallback_model_id =
                crate::domain::types::ModelId::try_new("unknown-model".to_string())
                    .map_err(|e| {
                        tracing::error!("Failed to create fallback model ID: {}", e);
                    })
                    .ok();

            let fallback_prompt =
                crate::domain::types::Prompt::try_new("Request body not available".to_string())
                    .map_err(|e| {
                        tracing::error!("Failed to create fallback prompt: {}", e);
                    })
                    .ok();

            // If any of the fallbacks failed, we have a critical error
            match (fallback_provider, fallback_model_id, fallback_prompt) {
                (Some(provider), Some(model_id), Some(prompt)) => (
                    crate::domain::llm::ModelVersion {
                        provider: crate::domain::llm::LlmProvider::Other(provider),
                        model_id,
                    },
                    prompt,
                    crate::domain::types::LlmParameters::new(Default::default()),
                ),
                _ => {
                    // This should be extremely rare - it means our "safe" fallback values
                    // are failing validation, indicating a serious configuration issue
                    return Err(eventcore::CommandError::Internal(
                        "Critical error: cannot create fallback values for missing LLM data. This indicates a serious validation configuration issue.".to_string()
                    ));
                }
            }
        };

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

        Ok(events)
    }
}

/// Command to record that a request was forwarded to the target
#[derive(Debug, Clone, Serialize, Deserialize, Command)]
pub struct RecordRequestForwarded {
    #[stream]
    pub session_stream: StreamId,
    #[stream]
    pub request_stream: StreamId,
    pub request_id: RequestId,
    pub session_id: SessionId,
    pub target_url: TargetUrl,
    pub timestamp: Timestamp,
}

impl RecordRequestForwarded {
    /// Create from an audit event
    pub fn from_audit_event(audit_event: &AuditEvent) -> Result<Self, AuditCommandError> {
        match &audit_event.event_type {
            AuditEventType::RequestForwarded {
                target_url,
                start_time,
            } => {
                let session_stream =
                    StreamId::try_new(format!("session-{}", audit_event.session_id))
                        .map_err(|e| AuditCommandError::InvalidStreamId(e.to_string()))?;
                let request_stream =
                    StreamId::try_new(format!("request-{}", audit_event.request_id))
                        .map_err(|e| AuditCommandError::InvalidStreamId(e.to_string()))?;

                let session_id = SessionId::new(*audit_event.session_id.as_ref());
                let timestamp = Timestamp::try_new(*start_time).map_err(|e| {
                    AuditCommandError::InvalidTimestamp(format!(
                        "Failed to convert start_time: {e}"
                    ))
                })?;

                Ok(Self {
                    session_stream,
                    request_stream,
                    request_id: audit_event.request_id,
                    session_id,
                    target_url: target_url.clone(),
                    timestamp,
                })
            }
            _ => Err(AuditCommandError::WrongEventType {
                expected: "RequestForwarded".to_string(),
                actual: format!("{:?}", audit_event.event_type),
            }),
        }
    }
}

#[async_trait]
impl CommandLogic for RecordRequestForwarded {
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

        // Check if we've already forwarded this request
        if state.is_request_forwarded() {
            return Ok(events);
        }

        emit!(
            events,
            &_read_streams,
            self.request_stream.clone(),
            DomainEvent::LlmRequestStarted {
                request_id: crate::domain::llm::RequestId::new(*self.request_id.as_ref()),
                started_at: self.timestamp,
            }
        );

        Ok(events)
    }
}

/// Command to record that a response was received from the target
#[derive(Debug, Clone, Serialize, Deserialize, Command)]
pub struct RecordResponseReceived {
    #[stream]
    pub session_stream: StreamId,
    #[stream]
    pub request_stream: StreamId,
    pub request_id: RequestId,
    pub session_id: SessionId,
    pub status: HttpStatusCode,
    pub headers: Headers,
    pub body_size: BodySize,
    pub duration_ms: DurationMillis,
    pub timestamp: Timestamp,
}

impl RecordResponseReceived {
    /// Create from an audit event
    pub fn from_audit_event(audit_event: &AuditEvent) -> Result<Self, AuditCommandError> {
        match &audit_event.event_type {
            AuditEventType::ResponseReceived {
                status,
                headers,
                body_size,
                duration_ms,
            } => {
                let session_stream =
                    StreamId::try_new(format!("session-{}", audit_event.session_id))
                        .map_err(|e| AuditCommandError::InvalidStreamId(e.to_string()))?;
                let request_stream =
                    StreamId::try_new(format!("request-{}", audit_event.request_id))
                        .map_err(|e| AuditCommandError::InvalidStreamId(e.to_string()))?;

                let session_id = SessionId::new(*audit_event.session_id.as_ref());
                let timestamp = Timestamp::try_new(audit_event.timestamp).map_err(|e| {
                    AuditCommandError::InvalidTimestamp(format!("Failed to convert timestamp: {e}"))
                })?;

                Ok(Self {
                    session_stream,
                    request_stream,
                    request_id: audit_event.request_id,
                    session_id,
                    status: *status,
                    headers: headers.clone(),
                    body_size: *body_size,
                    duration_ms: *duration_ms,
                    timestamp,
                })
            }
            _ => Err(AuditCommandError::WrongEventType {
                expected: "ResponseReceived".to_string(),
                actual: format!("{:?}", audit_event.event_type),
            }),
        }
    }
}

#[async_trait]
impl CommandLogic for RecordResponseReceived {
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

        // Check if we've already received this response
        if state.is_response_received() {
            return Ok(events);
        }

        // For now, we don't have the response body here
        // TODO: Implement response body parsing similar to request parsing
        let response_text = crate::domain::types::ResponseText::try_new(
            "Response body parsing not yet implemented".to_string(),
        )
        .map_err(|e| {
            tracing::error!("Failed to create response text placeholder: {}", e);
            eventcore::CommandError::Internal(format!(
                "Failed to create response text placeholder: {e}. This should not happen with a valid static string."
            ))
        })?;
        let metadata = crate::domain::llm::ResponseMetadata::default();

        emit!(
            events,
            &_read_streams,
            self.request_stream.clone(),
            DomainEvent::LlmResponseReceived {
                request_id: crate::domain::llm::RequestId::new(*self.request_id.as_ref()),
                response_text,
                metadata,
                received_at: self.timestamp,
            }
        );

        Ok(events)
    }
}

/// Command to record that a response was returned to the client
#[derive(Debug, Clone, Serialize, Deserialize, Command)]
pub struct RecordResponseReturned {
    #[stream]
    pub session_stream: StreamId,
    #[stream]
    pub request_stream: StreamId,
    pub request_id: RequestId,
    pub session_id: SessionId,
    pub duration_ms: DurationMillis,
    pub timestamp: Timestamp,
}

impl RecordResponseReturned {
    /// Create from an audit event
    pub fn from_audit_event(audit_event: &AuditEvent) -> Result<Self, AuditCommandError> {
        match &audit_event.event_type {
            AuditEventType::ResponseReturned { duration_ms } => {
                let session_stream =
                    StreamId::try_new(format!("session-{}", audit_event.session_id))
                        .map_err(|e| AuditCommandError::InvalidStreamId(e.to_string()))?;
                let request_stream =
                    StreamId::try_new(format!("request-{}", audit_event.request_id))
                        .map_err(|e| AuditCommandError::InvalidStreamId(e.to_string()))?;

                let session_id = SessionId::new(*audit_event.session_id.as_ref());
                let timestamp = Timestamp::try_new(audit_event.timestamp).map_err(|e| {
                    AuditCommandError::InvalidTimestamp(format!("Failed to convert timestamp: {e}"))
                })?;

                Ok(Self {
                    session_stream,
                    request_stream,
                    request_id: audit_event.request_id,
                    session_id,
                    duration_ms: *duration_ms,
                    timestamp,
                })
            }
            _ => Err(AuditCommandError::WrongEventType {
                expected: "ResponseReturned".to_string(),
                actual: format!("{:?}", audit_event.event_type),
            }),
        }
    }
}

#[async_trait]
impl CommandLogic for RecordResponseReturned {
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
        let events = Vec::new();

        // Check if we've already returned this response
        if state.is_response_returned() {
            return Ok(events);
        }

        // For now, we don't emit any specific event for response returned
        // The LlmResponseReceived event already captures the completion

        Ok(events)
    }
}

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
        let headers_vec: Vec<(String, String)> = self
            .headers
            .as_vec()
            .iter()
            .map(|(name, value)| (name.as_ref().to_string(), value.as_ref().to_string()))
            .collect();

        // Parse the LLM request
        let parsed_result = parse_llm_request(&self.body, self.uri.as_ref(), &headers_vec);

        let (model_version, prompt, parameters) = match parsed_result {
            Ok(parsed) => (parsed.model_version, parsed.prompt, parsed.parameters),
            Err(e) => {
                tracing::warn!(
                    "Failed to parse LLM request {}: {}. Using fallback.",
                    self.request_id,
                    e
                );
                let fallback = create_fallback_request(&e);
                (fallback.model_version, fallback.prompt, fallback.parameters)
            }
        };

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

        Ok(events)
    }
}

#[derive(Debug, thiserror::Error)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proxy::types::SessionId as ProxySessionId;
    use chrono::Utc;

    #[test]
    fn test_audit_event_to_request_received_command() {
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

        let command = RecordRequestReceived::from_audit_event(&audit_event);
        assert!(command.is_ok());
        let cmd = command.unwrap();
        assert_eq!(cmd.request_id, audit_event.request_id);
        // Can't directly compare different SessionId types, so check they wrap the same UUID
        assert_eq!(
            cmd.session_id.into_inner(),
            *audit_event.session_id.as_ref()
        );
    }

    #[test]
    fn test_wrong_event_type() {
        let audit_event = AuditEvent {
            request_id: RequestId::new(),
            session_id: ProxySessionId::new(),
            timestamp: Utc::now(),
            event_type: AuditEventType::Error {
                error: "test error".to_string(),
                phase: crate::proxy::types::ErrorPhase::RequestParsing,
            },
        };

        let command = RecordRequestReceived::from_audit_event(&audit_event);
        assert!(command.is_err());
        assert!(matches!(
            command.unwrap_err(),
            AuditCommandError::WrongEventType { .. }
        ));
    }

    #[tokio::test]
    async fn test_record_request_received_command_logic() {
        // Create a command
        let session_id = SessionId::generate();
        let request_id = RequestId::new();
        let _command = RecordRequestReceived {
            session_stream: StreamId::try_new(format!(
                "session-{}",
                session_id.clone().into_inner()
            ))
            .unwrap(),
            request_stream: StreamId::try_new(format!("request-{request_id}")).unwrap(),
            request_id,
            session_id,
            method: HttpMethod::try_new("POST".to_string()).unwrap(),
            uri: RequestUri::try_new("/api/test".to_string()).unwrap(),
            headers: Headers::new(),
            body_size: BodySize::from(1024),
            timestamp: Timestamp::now(),
            parsed_request: None,
        };

        // This test will fail to compile until we implement CommandLogic
        // Let's assert that RecordRequestReceived implements CommandLogic
        fn assert_command_logic<T: CommandLogic>() {}
        assert_command_logic::<RecordRequestReceived>(); // This will fail until we implement CommandLogic
    }

    #[test]
    fn test_record_request_with_body_parsing() {
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

        let command = RecordRequestReceived {
            session_stream: StreamId::try_new(format!(
                "session-{}",
                session_id.clone().into_inner()
            ))
            .unwrap(),
            request_stream: StreamId::try_new(format!("request-{request_id}")).unwrap(),
            request_id,
            session_id,
            method: HttpMethod::try_new("POST".to_string()).unwrap(),
            uri: RequestUri::try_new("/v1/chat/completions".to_string()).unwrap(),
            headers: Headers::new(),
            body_size: BodySize::from(body.to_string().len()),
            timestamp: Timestamp::now(),
            parsed_request: None,
        };

        // Apply body parsing
        let command_with_body = command.with_body(body.to_string().as_bytes());

        // Verify parsing succeeded
        assert!(command_with_body.parsed_request.is_some());
        let parsed = command_with_body.parsed_request.unwrap();
        assert_eq!(parsed.model_version.model_id.as_ref(), "gpt-4");
        assert!(parsed.prompt.as_ref().contains("user: Hello, world!"));
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
}
