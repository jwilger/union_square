//! Adapter layer for converting proxy boundary DTOs to domain types
//!
//! This module provides conversion functions that translate structural proxy types
//! into semantic domain facts. All parsing and validation happens at this boundary
//! before data enters the domain core.

use crate::domain::{
    audit_types,
    commands::audit_commands::{AuditCommandError, RecordAuditEvent},
    llm,
    metrics::Timestamp,
    session::SessionId,
};

/// Convert a proxy audit event into a domain `RecordAuditEvent` command.
///
/// This is the canonical conversion seam between the proxy boundary and the domain.
/// All proxy structural types are converted to semantic domain types here.
pub fn convert_audit_event(
    proxy_event: &crate::proxy::types::AuditEvent,
) -> Result<RecordAuditEvent, AuditCommandError> {
    let session_id = SessionId::new(*proxy_event.session_id.as_ref());
    let request_id = llm::RequestId::new(*proxy_event.request_id.as_ref());

    let session_stream = RecordAuditEvent::session_stream_id(&session_id)
        .map_err(|e| AuditCommandError::InvalidStreamId(format!("session stream: {e}")))?;
    let request_stream = RecordAuditEvent::request_stream_id(&request_id)
        .map_err(|e| AuditCommandError::InvalidStreamId(format!("request stream: {e}")))?;

    let timestamp = Timestamp::try_new(proxy_event.timestamp).map_err(|e| {
        AuditCommandError::InvalidTimestamp(format!("Failed to convert timestamp: {e}"))
    })?;

    let audit_event = convert_audit_event_type(&proxy_event.event_type)?;

    Ok(RecordAuditEvent {
        session_stream,
        request_stream,
        request_id,
        session_id,
        audit_event,
        timestamp,
        parsed_request: None,
    })
}

/// Convert proxy `AuditEventType` to domain `AuditEventType`.
fn convert_audit_event_type(
    proxy_type: &crate::proxy::types::AuditEventType,
) -> Result<audit_types::AuditEventType, AuditCommandError> {
    use crate::proxy::types::AuditEventType as ProxyType;

    match proxy_type {
        ProxyType::RequestReceived {
            method,
            uri,
            headers,
            body_size,
        } => {
            let method = audit_types::HttpMethod::try_new(method.as_ref().to_string())
                .map_err(|e| AuditCommandError::InvalidField(format!("method: {e}")))?;
            let uri = audit_types::RequestUri::try_new(uri.as_ref().to_string())
                .map_err(|e| AuditCommandError::InvalidField(format!("uri: {e}")))?;
            let headers = convert_headers(headers)?;
            let body_size = audit_types::BodySize::from(*body_size.as_ref());
            Ok(audit_types::AuditEventType::RequestReceived {
                method,
                uri,
                headers,
                body_size,
            })
        }
        ProxyType::RequestForwarded {
            target_url,
            start_time,
        } => {
            let target_url = audit_types::TargetUrl::try_new(target_url.as_ref().to_string())
                .map_err(|e| AuditCommandError::InvalidField(format!("target_url: {e}")))?;
            Ok(audit_types::AuditEventType::RequestForwarded {
                target_url,
                start_time: *start_time,
            })
        }
        ProxyType::ResponseReceived {
            status,
            headers,
            body_size,
            duration_ms,
        } => {
            let status = audit_types::HttpStatusCode::try_new(*status.as_ref())
                .map_err(|e| AuditCommandError::InvalidField(format!("status: {e}")))?;
            let headers = convert_headers(headers)?;
            let body_size = audit_types::BodySize::from(*body_size.as_ref());
            Ok(audit_types::AuditEventType::ResponseReceived {
                status,
                headers,
                body_size,
                duration_ms: *duration_ms.as_ref(),
            })
        }
        ProxyType::ResponseReturned { duration_ms } => {
            Ok(audit_types::AuditEventType::ResponseReturned {
                duration_ms: *duration_ms.as_ref(),
            })
        }
        ProxyType::RequestBody { .. } => Err(AuditCommandError::InvalidField(
            "RequestBody not supported at domain boundary".to_string(),
        )),
        ProxyType::ResponseBody { .. } => Err(AuditCommandError::InvalidField(
            "ResponseBody not supported at domain boundary".to_string(),
        )),
        ProxyType::RequestChunk { .. } => Err(AuditCommandError::InvalidField(
            "RequestChunk not supported at domain boundary".to_string(),
        )),
        ProxyType::ResponseChunk { .. } => Err(AuditCommandError::InvalidField(
            "ResponseChunk not supported at domain boundary".to_string(),
        )),
        ProxyType::Error { error, phase } => {
            let error = crate::domain::types::ErrorMessage::try_new(error.clone())
                .map_err(|e| AuditCommandError::InvalidField(format!("error: {e}")))?;
            let phase = convert_error_phase(phase);
            Ok(audit_types::AuditEventType::Error { error, phase })
        }
    }
}

/// Convert proxy `Headers` to domain `HttpHeaders`.
fn convert_headers(
    proxy_headers: &crate::proxy::types::Headers,
) -> Result<audit_types::HttpHeaders, AuditCommandError> {
    let pairs: Vec<(String, String)> = proxy_headers
        .as_vec()
        .iter()
        .map(|(name, value)| (name.as_ref().to_string(), value.as_ref().to_string()))
        .collect();

    audit_types::HttpHeaders::try_from_pairs(pairs)
        .map_err(|e| AuditCommandError::InvalidField(format!("headers: {e}")))
}

/// Convert proxy `ErrorPhase` to domain `ErrorPhase`.
fn convert_error_phase(proxy_phase: &crate::proxy::types::ErrorPhase) -> audit_types::ErrorPhase {
    match proxy_phase {
        crate::proxy::types::ErrorPhase::RequestParsing => audit_types::ErrorPhase::RequestParsing,
        crate::proxy::types::ErrorPhase::RequestForwarding => {
            audit_types::ErrorPhase::RequestForwarding
        }
        crate::proxy::types::ErrorPhase::ResponseReceiving => {
            audit_types::ErrorPhase::ResponseReceiving
        }
        crate::proxy::types::ErrorPhase::ResponseReturning => {
            audit_types::ErrorPhase::ResponseReturning
        }
        crate::proxy::types::ErrorPhase::AuditRecording => audit_types::ErrorPhase::AuditRecording,
    }
}

/// Parse a request body into domain semantic facts at the adapter boundary.
///
/// This keeps all transport-to-domain parsing out of the functional core.
pub fn parse_request_body(
    body: &[u8],
    uri: &audit_types::RequestUri,
    headers: &audit_types::HttpHeaders,
) -> crate::domain::commands::audit_commands::ParsedLlmRequestWithError {
    use crate::domain::commands::audit_commands::ParsedLlmRequestWithError;
    use crate::domain::commands::llm_request_parser::{create_fallback_request, parse_llm_request};

    let headers_vec = headers
        .as_pairs()
        .iter()
        .map(|(name, value)| (name.as_ref().to_string(), value.as_ref().to_string()))
        .collect::<Vec<_>>();

    match parse_llm_request(body, uri.as_ref(), &headers_vec) {
        Ok(parsed) => ParsedLlmRequestWithError::new(parsed, None, uri.as_ref().to_string()),
        Err(e) => ParsedLlmRequestWithError::new(
            create_fallback_request(&e),
            Some(e.to_string()),
            uri.as_ref().to_string(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_request_received() {
        let proxy_event = crate::proxy::types::AuditEvent {
            request_id: crate::proxy::types::RequestId::new(),
            session_id: crate::proxy::types::SessionId::new(),
            timestamp: chrono::Utc::now(),
            event_type: crate::proxy::types::AuditEventType::RequestReceived {
                method: crate::proxy::types::HttpMethod::try_new("GET".to_string()).unwrap(),
                uri: crate::proxy::types::RequestUri::try_new("/test".to_string()).unwrap(),
                headers: crate::proxy::types::Headers::new(),
                body_size: crate::proxy::types::BodySize::from(0),
            },
        };

        let result = convert_audit_event(&proxy_event);
        assert!(result.is_ok());

        let cmd = result.unwrap();
        assert_eq!(
            cmd.audit_event,
            audit_types::AuditEventType::RequestReceived {
                method: audit_types::HttpMethod::try_new("GET".to_string()).unwrap(),
                uri: audit_types::RequestUri::try_new("/test".to_string()).unwrap(),
                headers: audit_types::HttpHeaders::new(),
                body_size: audit_types::BodySize::from(0),
            }
        );
    }

    #[test]
    fn converts_error_event() {
        let proxy_event = crate::proxy::types::AuditEvent {
            request_id: crate::proxy::types::RequestId::new(),
            session_id: crate::proxy::types::SessionId::new(),
            timestamp: chrono::Utc::now(),
            event_type: crate::proxy::types::AuditEventType::Error {
                error: "something went wrong".to_string(),
                phase: crate::proxy::types::ErrorPhase::RequestForwarding,
            },
        };

        let result = convert_audit_event(&proxy_event);
        assert!(result.is_ok());
    }
}
