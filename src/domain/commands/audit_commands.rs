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

/// State for request tracking
#[derive(Debug, Default, Clone)]
pub struct RequestState {
    pub request_received: bool,
    pub request_forwarded: bool,
    pub response_received: bool,
    pub response_returned: bool,
}

impl RequestState {
    /// Apply an event to update the state
    pub fn apply(&mut self, event: &DomainEvent) {
        match event {
            DomainEvent::LlmRequestReceived { .. } => {
                self.request_received = true;
            }
            DomainEvent::LlmRequestStarted { .. } => {
                self.request_forwarded = true;
            }
            DomainEvent::LlmResponseReceived { .. } => {
                self.response_received = true;
                self.response_returned = true;
            }
            _ => {} // Ignore other events
        }
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
                let timestamp = Timestamp::try_new(audit_event.timestamp).map_err(|_| {
                    AuditCommandError::InvalidStreamId("Invalid timestamp".to_string())
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
                })
            }
            _ => Err(AuditCommandError::WrongEventType {
                expected: "RequestReceived".to_string(),
                actual: format!("{:?}", audit_event.event_type),
            }),
        }
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
        if state.request_received {
            // Already recorded, skip
            return Ok(events);
        }

        // Create the LlmRequestReceived event
        // Note: We need to extract the actual prompt and parameters from the request
        // For now, we'll use placeholder values
        let prompt = crate::domain::types::Prompt::try_new("Placeholder prompt".to_string())
            .expect("Valid prompt");
        let parameters = crate::domain::types::LlmParameters::new(Default::default());
        let model_version = crate::domain::llm::ModelVersion {
            provider: crate::domain::llm::LlmProvider::Other(
                crate::domain::config_types::ProviderName::try_new("placeholder".to_string())
                    .expect("Valid provider name"),
            ),
            model_id: crate::domain::types::ModelId::try_new("placeholder-model".to_string())
                .expect("Valid model ID"),
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
                let timestamp = Timestamp::try_new(*start_time).map_err(|_| {
                    AuditCommandError::InvalidStreamId("Invalid timestamp".to_string())
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
        if state.request_forwarded {
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
                let timestamp = Timestamp::try_new(audit_event.timestamp).map_err(|_| {
                    AuditCommandError::InvalidStreamId("Invalid timestamp".to_string())
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
        if state.response_received {
            return Ok(events);
        }

        // Create placeholder response metadata
        let response_text =
            crate::domain::types::ResponseText::try_new("Placeholder response".to_string())
                .expect("Valid response");
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
                let timestamp = Timestamp::try_new(audit_event.timestamp).map_err(|_| {
                    AuditCommandError::InvalidStreamId("Invalid timestamp".to_string())
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
        if state.response_returned {
            return Ok(events);
        }

        // For now, we don't emit any specific event for response returned
        // The LlmResponseReceived event already captures the completion

        Ok(events)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AuditCommandError {
    #[error("Invalid stream ID: {0}")]
    InvalidStreamId(String),

    #[error("Wrong event type: expected {expected}, got {actual}")]
    WrongEventType { expected: String, actual: String },
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
        };

        // This test will fail to compile until we implement CommandLogic
        // Let's assert that RecordRequestReceived implements CommandLogic
        fn assert_command_logic<T: CommandLogic>() {}
        assert_command_logic::<RecordRequestReceived>(); // This will fail until we implement CommandLogic
    }
}
