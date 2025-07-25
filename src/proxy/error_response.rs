//! Unified error response handling for the proxy service
//!
//! This module provides consistent error formatting across all middleware
//! and handlers, ensuring proper request ID correlation and standardized
//! error messages.

use crate::proxy::types::{ProxyError, REQUEST_ID_HEADER};
use axum::{
    http::{HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};

/// Standard error response format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    /// Unique error code for programmatic handling
    pub code: String,
    /// Human-readable error message
    pub message: String,
    /// Request ID for correlation
    pub request_id: Option<String>,
    /// Additional error details (only in debug mode)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl ErrorResponse {
    /// Create a new error response
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            request_id: None,
            details: None,
        }
    }

    /// Add request ID for correlation
    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }

    /// Add additional error details (only shown in debug mode)
    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }

    /// Convert to HTTP response with proper headers
    pub fn into_response_with_status(self, status: StatusCode) -> Response {
        let request_id = self.request_id.clone();
        let mut response = (status, Json(self)).into_response();

        // Add request ID header if available
        if let Some(id) = request_id {
            if let Ok(header_value) = HeaderValue::from_str(&id) {
                response
                    .headers_mut()
                    .insert(REQUEST_ID_HEADER, header_value);
            }
        }

        response
    }
}

/// Extension trait for consistent error formatting
pub trait ErrorResponseExt {
    /// Convert to standardized error response
    fn to_error_response(&self) -> ErrorResponse;

    /// Get the appropriate HTTP status code
    fn status_code(&self) -> StatusCode;
}

impl ErrorResponseExt for ProxyError {
    fn to_error_response(&self) -> ErrorResponse {
        use ProxyError::*;

        match self {
            RequestTooLarge { size, max_size } => ErrorResponse::new(
                "REQUEST_TOO_LARGE",
                format!("Request size {size} exceeds maximum {max_size}"),
            ),
            ResponseTooLarge { size, max_size } => ErrorResponse::new(
                "RESPONSE_TOO_LARGE",
                format!("Response size {size} exceeds maximum {max_size}"),
            ),
            RequestTimeout(duration) => ErrorResponse::new(
                "REQUEST_TIMEOUT",
                format!("Request timed out after {duration:?}"),
            ),
            InvalidTargetUrl(msg) => {
                ErrorResponse::new("INVALID_TARGET_URL", format!("Invalid target URL: {msg}"))
            }
            RingBufferOverflow { dropped } => {
                ErrorResponse::new("INTERNAL_ERROR", "Service temporarily unavailable")
                    .with_details(serde_json::json!({
                        "dropped_events": dropped.as_ref()
                    }))
            }
            HttpError(e) => ErrorResponse::new("HTTP_ERROR", format!("HTTP error: {e}")),
            HyperError(e) => {
                ErrorResponse::new("CONNECTION_ERROR", format!("Connection error: {e}"))
            }
            IoError(e) => ErrorResponse::new("IO_ERROR", format!("I/O error: {e}")),
            SerializationError(e) => {
                ErrorResponse::new("SERIALIZATION_ERROR", format!("Serialization error: {e}"))
            }
            Internal(msg) => {
                // Map connection errors to gateway errors
                if msg.starts_with("Connection error:") {
                    ErrorResponse::new("CONNECTION_ERROR", msg.clone())
                } else {
                    ErrorResponse::new("INTERNAL_ERROR", msg.clone())
                }
            }
            InvalidHttpMethod(method) => ErrorResponse::new(
                "INVALID_HTTP_METHOD",
                format!("Invalid HTTP method: {method}"),
            ),
            InvalidRequestUri(uri) => {
                ErrorResponse::new("INVALID_REQUEST_URI", format!("Invalid request URI: {uri}"))
            }
            InvalidHttpStatusCode(code) => ErrorResponse::new(
                "INVALID_STATUS_CODE",
                format!("Invalid HTTP status code: {code}"),
            ),
            InvalidHeader { name } => {
                ErrorResponse::new("INVALID_HEADER", format!("Invalid header: {name}"))
            }
            AuditEventCreationFailed(msg) => {
                ErrorResponse::new("AUDIT_ERROR", "Failed to record audit event").with_details(
                    serde_json::json!({
                        "error": msg
                    }),
                )
            }
        }
    }

    fn status_code(&self) -> StatusCode {
        use ProxyError::*;

        match self {
            RequestTooLarge { .. } => StatusCode::PAYLOAD_TOO_LARGE,
            ResponseTooLarge { .. } => StatusCode::BAD_GATEWAY,
            RequestTimeout(_) => StatusCode::REQUEST_TIMEOUT,
            InvalidTargetUrl(_) => StatusCode::BAD_REQUEST,
            InvalidHttpMethod(_) => StatusCode::BAD_REQUEST,
            InvalidRequestUri(_) => StatusCode::BAD_REQUEST,
            InvalidHttpStatusCode(_) => StatusCode::BAD_GATEWAY,
            InvalidHeader { .. } => StatusCode::BAD_REQUEST,
            RingBufferOverflow { .. } => StatusCode::SERVICE_UNAVAILABLE,
            HttpError(_) | HyperError(_) => StatusCode::BAD_GATEWAY,
            Internal(msg) if msg.starts_with("Connection error:") => StatusCode::BAD_GATEWAY,
            IoError(_) | SerializationError(_) | Internal(_) | AuditEventCreationFailed(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }
}

/// Create an error response for common HTTP errors
pub fn standard_error_response(status: StatusCode, request_id: Option<&str>) -> Response {
    let (code, message) = match status {
        StatusCode::BAD_REQUEST => ("BAD_REQUEST", "Invalid request"),
        StatusCode::UNAUTHORIZED => ("UNAUTHORIZED", "Authentication required"),
        StatusCode::FORBIDDEN => ("FORBIDDEN", "Access denied"),
        StatusCode::NOT_FOUND => ("NOT_FOUND", "Resource not found"),
        StatusCode::METHOD_NOT_ALLOWED => ("METHOD_NOT_ALLOWED", "Method not allowed"),
        StatusCode::REQUEST_TIMEOUT => ("REQUEST_TIMEOUT", "Request timed out"),
        StatusCode::PAYLOAD_TOO_LARGE => ("PAYLOAD_TOO_LARGE", "Request too large"),
        StatusCode::INTERNAL_SERVER_ERROR => ("INTERNAL_ERROR", "Internal server error"),
        StatusCode::BAD_GATEWAY => ("BAD_GATEWAY", "Upstream service error"),
        StatusCode::SERVICE_UNAVAILABLE => {
            ("SERVICE_UNAVAILABLE", "Service temporarily unavailable")
        }
        StatusCode::GATEWAY_TIMEOUT => ("GATEWAY_TIMEOUT", "Upstream service timeout"),
        _ => ("ERROR", "An error occurred"),
    };

    let mut error = ErrorResponse::new(code, message);
    if let Some(id) = request_id {
        error = error.with_request_id(id);
    }

    error.into_response_with_status(status)
}

/// Helper to extract request ID from headers
pub fn extract_request_id(headers: &http::HeaderMap) -> Option<String> {
    headers
        .get(REQUEST_ID_HEADER)
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_response_creation() {
        let error = ErrorResponse::new("TEST_ERROR", "Test error message");
        assert_eq!(error.code, "TEST_ERROR");
        assert_eq!(error.message, "Test error message");
        assert!(error.request_id.is_none());
        assert!(error.details.is_none());
    }

    #[test]
    fn test_error_response_with_request_id() {
        let error = ErrorResponse::new("TEST_ERROR", "Test error").with_request_id("req-123");
        assert_eq!(error.request_id, Some("req-123".to_string()));
    }

    #[test]
    fn test_error_response_with_details() {
        let error = ErrorResponse::new("TEST_ERROR", "Test error")
            .with_details(serde_json::json!({ "field": "value" }));
        assert!(error.details.is_some());
    }

    #[test]
    fn test_proxy_error_to_response() {
        let error = ProxyError::InvalidTargetUrl("bad url".to_string());
        let response = error.to_error_response();
        assert_eq!(response.code, "INVALID_TARGET_URL");
        assert_eq!(error.status_code(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_standard_error_responses() {
        let response = standard_error_response(StatusCode::NOT_FOUND, Some("req-123"));
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert!(response.headers().contains_key(REQUEST_ID_HEADER));
    }
}
