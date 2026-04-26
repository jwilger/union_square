//! Domain audit event types
//!
//! This module provides semantic domain types for audit events, replacing
//! raw proxy types that previously leaked into the domain core. These types
//! represent business facts rather than transport structures.

use crate::domain::{llm::RequestId, session::SessionId, types::ErrorMessage};
use serde::{Deserialize, Serialize};

/// Size of HTTP body in bytes
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BodySize(usize);

impl BodySize {
    pub fn from(size: usize) -> Self {
        Self(size)
    }
}

impl AsRef<usize> for BodySize {
    fn as_ref(&self) -> &usize {
        &self.0
    }
}

/// Audit event captured at the proxy boundary, converted to domain facts
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditEvent {
    pub request_id: RequestId,
    pub session_id: SessionId,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub event_type: AuditEventType,
}

/// Types of audit events as semantic domain facts.
///
/// Raw bytes, headers, and URI strings from the proxy boundary are converted
/// to semantic types at the adapter layer before entering the domain.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditEventType {
    /// Request received from client
    RequestReceived {
        method: HttpMethod,
        uri: RequestUri,
        headers: HttpHeaders,
        body_size: BodySize,
    },
    /// Request forwarded to target provider
    RequestForwarded {
        target_url: TargetUrl,
        start_time: chrono::DateTime<chrono::Utc>,
    },
    /// Response received from provider
    ResponseReceived {
        status: HttpStatusCode,
        headers: HttpHeaders,
        body_size: BodySize,
        duration_ms: u64,
    },
    /// Response returned to client
    ResponseReturned { duration_ms: u64 },
    /// Error during processing
    Error {
        error: ErrorMessage,
        phase: ErrorPhase,
    },
    /// Complete request body received
    RequestBody { content: Vec<u8>, truncated: bool },
    /// Complete response body received
    ResponseBody { content: Vec<u8>, truncated: bool },
    /// Chunk of request body
    RequestChunk { offset: usize, data: Vec<u8> },
    /// Chunk of response body
    ResponseChunk { offset: usize, data: Vec<u8> },
}

/// HTTP method as a semantic domain type
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct HttpMethod(String);

impl HttpMethod {
    pub fn try_new(method: impl Into<String>) -> Result<Self, crate::domain::types::DomainError> {
        let method = method.into();
        if method.is_empty() {
            return Err(crate::domain::types::DomainError::InvalidHttpMethod(method));
        }
        Ok(Self(method))
    }
}

impl AsRef<str> for HttpMethod {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// HTTP request URI as a semantic domain type
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RequestUri(String);

impl RequestUri {
    pub fn try_new(uri: impl Into<String>) -> Result<Self, crate::domain::types::DomainError> {
        let uri = uri.into();
        if uri.is_empty() {
            return Err(crate::domain::types::DomainError::InvalidRequestUri(uri));
        }
        Ok(Self(uri))
    }
}

impl AsRef<str> for RequestUri {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// HTTP header name as a semantic domain type
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct HeaderName(String);

impl HeaderName {
    pub fn try_new(name: impl Into<String>) -> Result<Self, crate::domain::types::DomainError> {
        let name = name.into();
        if name.is_empty() {
            return Err(crate::domain::types::DomainError::InvalidHeaderName(name));
        }
        Ok(Self(name))
    }
}

impl AsRef<str> for HeaderName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// HTTP header value as a semantic domain type
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct HeaderValue(String);

impl From<String> for HeaderValue {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl AsRef<str> for HeaderValue {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// Collection of HTTP headers as semantic domain facts
#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct HttpHeaders(Vec<(HeaderName, HeaderValue)>);

impl HttpHeaders {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn try_from_pairs(
        pairs: Vec<(String, String)>,
    ) -> Result<Self, crate::domain::types::DomainError> {
        let mut headers = Vec::with_capacity(pairs.len());
        for (name, value) in pairs {
            headers.push((
                HeaderName::try_new(name.clone())
                    .map_err(|_| crate::domain::types::DomainError::InvalidHeaderName(name))?,
                HeaderValue::from(value),
            ));
        }
        Ok(Self(headers))
    }

    pub fn as_pairs(&self) -> &Vec<(HeaderName, HeaderValue)> {
        &self.0
    }

    pub fn into_pairs(self) -> Vec<(HeaderName, HeaderValue)> {
        self.0
    }
}

/// Target URL for proxying
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TargetUrl(String);

impl TargetUrl {
    pub fn try_new(url: impl Into<String>) -> Result<Self, crate::domain::types::DomainError> {
        let url = url.into();
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(crate::domain::types::DomainError::InvalidTargetUrl(url));
        }
        Ok(Self(url))
    }
}

impl AsRef<str> for TargetUrl {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// HTTP status code
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct HttpStatusCode(u16);

impl HttpStatusCode {
    pub fn try_new(code: u16) -> Result<Self, crate::domain::types::DomainError> {
        if !(100..=599).contains(&code) {
            return Err(crate::domain::types::DomainError::InvalidHttpStatusCode(
                code,
            ));
        }
        Ok(Self(code))
    }
}

impl AsRef<u16> for HttpStatusCode {
    fn as_ref(&self) -> &u16 {
        &self.0
    }
}

/// Phase where an error occurred
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ErrorPhase {
    RequestParsing,
    RequestForwarding,
    ResponseReceiving,
    ResponseReturning,
    AuditRecording,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http_method_rejects_empty() {
        assert!(HttpMethod::try_new("").is_err());
        assert!(HttpMethod::try_new("GET").is_ok());
    }

    #[test]
    fn request_uri_rejects_empty() {
        assert!(RequestUri::try_new("").is_err());
        assert!(RequestUri::try_new("/api/v1/chat").is_ok());
    }

    #[test]
    fn target_url_rejects_invalid() {
        assert!(TargetUrl::try_new("/api/v1").is_err());
        assert!(TargetUrl::try_new("https://api.openai.com").is_ok());
    }

    #[test]
    fn http_status_code_rejects_invalid() {
        assert!(HttpStatusCode::try_new(0).is_err());
        assert!(HttpStatusCode::try_new(99).is_err());
        assert!(HttpStatusCode::try_new(600).is_err());
        assert!(HttpStatusCode::try_new(200).is_ok());
    }

    #[test]
    fn http_headers_from_pairs() {
        let pairs = vec![
            ("Content-Type".to_string(), "application/json".to_string()),
            ("Authorization".to_string(), "Bearer token".to_string()),
        ];
        let headers = HttpHeaders::try_from_pairs(pairs).unwrap();
        assert_eq!(headers.as_pairs().len(), 2);
    }

    #[test]
    fn http_headers_rejects_empty_name() {
        let pairs = vec![("".to_string(), "value".to_string())];
        assert!(HttpHeaders::try_from_pairs(pairs).is_err());
    }

    #[test]
    fn audit_event_type_roundtrip() {
        let event = AuditEventType::RequestReceived {
            method: HttpMethod::try_new("POST").unwrap(),
            uri: RequestUri::try_new("/v1/chat/completions").unwrap(),
            headers: HttpHeaders::new(),
            body_size: BodySize::from(1024),
        };
        let json = serde_json::to_string(&event).unwrap();
        let decoded: AuditEventType = serde_json::from_str(&json).unwrap();
        assert_eq!(event, decoded);
    }
}
