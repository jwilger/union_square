//! Type definitions for the proxy module

use nutype::nutype;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;
use uuid::Uuid;

/// Proxy configuration
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProxyConfig {
    /// Maximum request size in bytes
    pub max_request_size: usize,
    /// Maximum response size in bytes
    pub max_response_size: usize,
    /// Request timeout
    pub request_timeout: Duration,
    /// Ring buffer configuration
    pub ring_buffer: RingBufferConfig,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            max_request_size: 10 * 1024 * 1024,  // 10MB
            max_response_size: 10 * 1024 * 1024, // 10MB
            request_timeout: Duration::from_secs(30),
            ring_buffer: RingBufferConfig::default(),
        }
    }
}

/// Ring buffer configuration
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RingBufferConfig {
    /// Total buffer size in bytes
    pub buffer_size: usize,
    /// Size of each slot in bytes
    pub slot_size: usize,
}

impl Default for RingBufferConfig {
    fn default() -> Self {
        Self {
            buffer_size: 1024 * 1024 * 1024, // 1GB
            slot_size: 64 * 1024,            // 64KB
        }
    }
}

/// Request ID for correlation between hot and audit paths
#[nutype(
    derive(Clone, Copy, Debug, Display, Deserialize, Serialize, TryFrom, AsRef),
    validate(predicate = |id: &Uuid| id.get_version_num() == 7),
    new_unchecked,
)]
pub struct RequestId(Uuid);

/// Target URL for proxying
#[nutype(
    derive(Clone, Debug, Display, Deserialize, Serialize, TryFrom, AsRef),
    validate(predicate = |s: &str| s.starts_with("http://") || s.starts_with("https://")),
    new_unchecked,
)]
pub struct TargetUrl(String);

/// API key for authentication
#[nutype(
    derive(Clone, Debug, Deserialize, Serialize, TryFrom, AsRef),
    validate(predicate = |s: &str| !s.is_empty()),
    new_unchecked,
)]
pub struct ApiKey(String);

/// Session ID for tracking related requests
#[nutype(
    derive(Clone, Copy, Debug, Display, Deserialize, Serialize, TryFrom, AsRef),
    validate(predicate = |id: &Uuid| id.get_version_num() == 7),
    new_unchecked,
)]
pub struct SessionId(Uuid);

/// Errors that can occur in the proxy
#[derive(Error, Debug)]
pub enum ProxyError {
    #[error("Request too large: {size} bytes (max: {max_size} bytes)")]
    RequestTooLarge { size: usize, max_size: usize },

    #[error("Response too large: {size} bytes (max: {max_size} bytes)")]
    ResponseTooLarge { size: usize, max_size: usize },

    #[error("Request timeout after {0:?}")]
    RequestTimeout(Duration),

    #[error("Invalid target URL: {0}")]
    InvalidTargetUrl(String),

    #[error("Ring buffer overflow: {dropped} events dropped")]
    RingBufferOverflow { dropped: u64 },

    #[error("HTTP error: {0}")]
    HttpError(#[from] http::Error),

    #[error("Hyper error: {0}")]
    HyperError(#[from] hyper::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Internal error: {0}")]
    Internal(String),
}

/// Result type for proxy operations
pub type ProxyResult<T> = Result<T, ProxyError>;

/// Event captured for audit logging
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuditEvent {
    pub request_id: RequestId,
    pub session_id: SessionId,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub event_type: AuditEventType,
}

/// Types of audit events
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AuditEventType {
    RequestReceived {
        method: String,
        uri: String,
        headers: Vec<(String, String)>,
        body_size: usize,
    },
    RequestForwarded {
        target_url: String,
        start_time: chrono::DateTime<chrono::Utc>,
    },
    ResponseReceived {
        status: u16,
        headers: Vec<(String, String)>,
        body_size: usize,
        duration_ms: u64,
    },
    ResponseReturned {
        duration_ms: u64,
    },
    RequestBody {
        content: Vec<u8>,
        truncated: bool,
    },
    ResponseBody {
        content: Vec<u8>,
        truncated: bool,
    },
    Error {
        error: String,
        phase: ErrorPhase,
    },
}

/// Phase where an error occurred
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ErrorPhase {
    RequestParsing,
    RequestForwarding,
    ResponseReceiving,
    ResponseReturning,
    AuditRecording,
}
