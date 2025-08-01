//! Type definitions for the proxy module
//!
//! This module defines all the domain types used throughout the proxy service.
//! All types use the `nutype` crate for validation, ensuring that invalid states
//! are impossible to represent.
//!
//! ## Type Categories
//!
//! ### Size and Capacity Types
//! Types for representing various sizes and capacities with validation:
//! - `RequestSizeLimit`, `ResponseSizeLimit`: Maximum sizes for HTTP payloads
//! - `BufferSize`, `SlotSize`: Ring buffer dimensions
//! - `BodySize`, `DataSize`: Actual data sizes
//!
//! ### Identifier Types
//! Unique identifiers with specific formats:
//! - `RequestId`: V7 UUID for request correlation
//! - `SessionId`: V7 UUID for session tracking
//! - `ApiKey`: Non-empty string for authentication
//!
//! ### HTTP Types
//! HTTP-specific types with validation:
//! - `HttpMethod`, `HttpStatusCode`: Standard HTTP elements
//! - `TargetUrl`: Validated URL for proxying
//! - `RequestUri`: Valid URI path
//!
//! ### Audit Types
//! Types for the audit system:
//! - `AuditEvent`: Events captured during request processing
//! - `AuditEventType`: Different types of audit events
//! - `ErrorPhase`: When errors occurred in processing
//!
//! ## Example Usage
//!
//! ```rust,ignore
//! use union_square::proxy::types::*;
//!
//! // Create validated types
//! let request_size = RequestSizeLimit::try_new(1024 * 1024)?; // 1MB
//! let request_id = RequestId::new(); // Generates V7 UUID
//! let api_key = ApiKey::try_new("sk-123456")?;
//!
//! // Types ensure validation at compile time
//! let config = ProxyConfig {
//!     max_request_size: request_size,
//!     max_response_size: ResponseSizeLimit::try_new(10 * 1024 * 1024)?,
//!     request_timeout: Duration::from_secs(30),
//!     ring_buffer: RingBufferConfig::default(),
//! };
//! ```

use crate::providers::bedrock::types::AwsRegion;
use nutype::nutype;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;
use uuid::Uuid;

// ========== Size and Capacity Types ==========

/// Maximum size for HTTP requests in bytes
#[nutype(
    derive(Clone, Copy, Debug, Display, Deserialize, Serialize, TryFrom, AsRef),
    validate(predicate = |size: &usize| *size > 0),
)]
pub struct RequestSizeLimit(usize);

/// Maximum size for HTTP responses in bytes
#[nutype(
    derive(Clone, Copy, Debug, Display, Deserialize, Serialize, TryFrom, AsRef),
    validate(predicate = |size: &usize| *size > 0),
)]
pub struct ResponseSizeLimit(usize);

/// Total buffer size for ring buffer in bytes
#[nutype(
    derive(Clone, Copy, Debug, Display, Deserialize, Serialize, TryFrom, AsRef),
    validate(predicate = |size: &usize| *size > 0 && size.is_power_of_two()),
)]
pub struct BufferSize(usize);

/// Size of individual slots in ring buffer in bytes
#[nutype(
    derive(Clone, Copy, Debug, Display, Deserialize, Serialize, TryFrom, AsRef),
    validate(predicate = |size: &usize| *size > 0),
)]
pub struct SlotSize(usize);

/// Actual size of data in a buffer slot
#[nutype(
    derive(Clone, Copy, Debug, Display, Deserialize, Serialize, TryFrom, AsRef),
    validate(predicate = |size: &usize| *size > 0),
)]
pub struct DataSize(usize);

/// Size of HTTP body in bytes
#[nutype(derive(Clone, Copy, Debug, Display, Deserialize, Serialize, From, AsRef))]
pub struct BodySize(usize);

// ========== Count Types ==========

/// Number of events dropped due to buffer overflow
#[nutype(derive(Clone, Copy, Debug, Display, Deserialize, Serialize, From, AsRef))]
pub struct DroppedEventCount(u64);

/// Number of slots in the ring buffer
#[nutype(
    derive(Clone, Copy, Debug, Display, Deserialize, Serialize, TryFrom, AsRef),
    validate(predicate = |count: &usize| *count > 0 && count.is_power_of_two()),
)]
pub struct SlotCount(usize);

// ========== Time Types ==========

/// Duration in milliseconds
#[nutype(derive(Clone, Copy, Debug, Display, Deserialize, Serialize, From, AsRef))]
pub struct DurationMillis(u64);

/// Timestamp in nanoseconds since epoch
#[nutype(derive(Clone, Copy, Debug, Display, Deserialize, Serialize, From, AsRef))]
pub struct TimestampNanos(u64);

// ========== HTTP Types ==========

/// HTTP method as a string (for serialization)
#[nutype(
    derive(Clone, Debug, Display, Deserialize, Serialize, TryFrom, AsRef),
    validate(predicate = |s: &str| !s.is_empty()),
)]
pub struct HttpMethod(String);

/// HTTP request URI
#[nutype(
    derive(Clone, Debug, Display, Deserialize, Serialize, TryFrom, AsRef),
    validate(predicate = |s: &str| !s.is_empty()),
)]
pub struct RequestUri(String);

/// HTTP status code
#[nutype(
    derive(Clone, Copy, Debug, Display, Deserialize, Serialize, TryFrom, AsRef),
    validate(predicate = |code: &u16| (100..=599).contains(code)),
)]
pub struct HttpStatusCode(u16);

/// HTTP header name
#[nutype(
    derive(Clone, Debug, Display, Deserialize, Serialize, TryFrom, AsRef),
    validate(predicate = |s: &str| !s.is_empty()),
)]
pub struct HeaderName(String);

/// HTTP header value
#[nutype(derive(Clone, Debug, Display, Deserialize, Serialize, From, AsRef))]
pub struct HeaderValue(String);

/// Collection of HTTP headers
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Headers(Vec<(HeaderName, HeaderValue)>);

impl Headers {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn from_vec(headers: Vec<(String, String)>) -> Result<Self, ProxyError> {
        let typed_headers = headers
            .into_iter()
            .map(|(name, value)| {
                Ok((
                    HeaderName::try_new(name)
                        .map_err(|e| ProxyError::Internal(format!("Invalid header name: {e}")))?,
                    HeaderValue::from(value),
                ))
            })
            .collect::<Result<Vec<_>, ProxyError>>()?;
        Ok(Self(typed_headers))
    }

    pub fn as_vec(&self) -> &Vec<(HeaderName, HeaderValue)> {
        &self.0
    }

    pub fn into_vec(self) -> Vec<(HeaderName, HeaderValue)> {
        self.0
    }
}

impl Default for Headers {
    fn default() -> Self {
        Self::new()
    }
}

// ========== Path Types ==========

/// Path that bypasses authentication
#[nutype(
    derive(Clone, Debug, Display, Hash, PartialEq, Eq, Deserialize, Serialize, TryFrom, AsRef),
    validate(predicate = |s: &str| s.starts_with('/')),
)]
pub struct BypassPath(String);

/// Offset for chunked data
#[nutype(derive(Clone, Copy, Debug, Display, Deserialize, Serialize, From, AsRef))]
pub struct ChunkOffset(usize);

// ========== Constants ==========

/// Size of UUID in bytes
pub const UUID_SIZE_BYTES: usize = 16;

/// Cache line size for alignment
pub const CACHE_LINE_SIZE: usize = 64;

/// Common HTTP methods
pub const METHOD_GET: &str = "GET";
pub const METHOD_POST: &str = "POST";

/// Common HTTP status codes
pub const STATUS_OK: u16 = 200;
pub const STATUS_INTERNAL_ERROR: u16 = 500;

// ========== Size Constants ==========

/// Common byte sizes
pub const BYTES_1KB: usize = 1024;
pub const BYTES_2KB: usize = 2 * BYTES_1KB;
pub const BYTES_16KB: usize = 16 * BYTES_1KB;
pub const BYTES_32KB: usize = 32 * BYTES_1KB;
pub const BYTES_64KB: usize = 64 * BYTES_1KB;
pub const BYTES_128KB: usize = 128 * BYTES_1KB;
pub const BYTES_1MB: usize = 1024 * BYTES_1KB;
pub const BYTES_2MB: usize = 2 * BYTES_1MB;
pub const BYTES_10MB: usize = 10 * BYTES_1MB;
pub const BYTES_512MB: usize = 512 * BYTES_1MB;
pub const BYTES_1GB: usize = 1024 * BYTES_1MB;

/// Buffer sizes for testing
pub const BUFFER_SIZE_SMALL: usize = 256; // For stress tests
pub const BUFFER_SIZE_TEST: usize = BYTES_1KB; // Standard test size
pub const BUFFER_SIZE_DEFAULT: usize = BYTES_1MB; // Default buffer

/// Slot sizes for testing
pub const SLOT_SIZE_TINY: usize = 64; // For stress tests
pub const SLOT_SIZE_SMALL: usize = 128; // Small test slots
pub const SLOT_SIZE_TEST: usize = BYTES_1KB; // Standard test slots

/// Thread and iteration counts for testing
pub const TEST_THREAD_COUNT: usize = 10;
pub const TEST_ITERATIONS_SMALL: usize = 100;
pub const TEST_ITERATIONS_LARGE: usize = 1000;

/// Network and timeout constants
pub const TEST_PORT_BASE: u16 = 8080;
pub const TIMEOUT_SHORT_MS: u64 = 100;
pub const TIMEOUT_DEFAULT_SECS: u64 = 30;
pub const TIMEOUT_LONG_SECS: u64 = 60;

/// Channel buffer sizes
pub const CHANNEL_BUFFER_SIZE: usize = 16;

/// Proxy configuration
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProxyConfig {
    /// Maximum request size in bytes
    pub max_request_size: RequestSizeLimit,
    /// Maximum response size in bytes
    pub max_response_size: ResponseSizeLimit,
    /// Request timeout
    pub request_timeout: Duration,
    /// Ring buffer configuration
    pub ring_buffer: RingBufferConfig,
    /// AWS region for Bedrock provider
    pub bedrock_region: Option<AwsRegion>,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            max_request_size: RequestSizeLimit::try_new(BYTES_10MB).expect("10MB is valid"),
            max_response_size: ResponseSizeLimit::try_new(BYTES_10MB).expect("10MB is valid"),
            request_timeout: Duration::from_secs(TIMEOUT_DEFAULT_SECS),
            ring_buffer: RingBufferConfig::default(),
            bedrock_region: None,
        }
    }
}

/// Ring buffer configuration
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RingBufferConfig {
    /// Total buffer size in bytes
    pub buffer_size: BufferSize,
    /// Size of each slot in bytes
    pub slot_size: SlotSize,
}

impl Default for RingBufferConfig {
    fn default() -> Self {
        Self {
            buffer_size: BufferSize::try_new(BYTES_1GB).expect("1GB is valid power of 2"),
            slot_size: SlotSize::try_new(BYTES_64KB).expect("64KB is valid"),
        }
    }
}

/// Request ID for correlation between hot and audit paths
#[nutype(
    derive(Clone, Copy, Debug, Display, Hash, PartialEq, Eq, Deserialize, Serialize, TryFrom, AsRef),
    validate(predicate = |id: &Uuid| id.get_version_num() == 7),
)]
pub struct RequestId(Uuid);

impl RequestId {
    /// Create a new RequestId with a v7 UUID
    pub fn new() -> Self {
        // Uuid::now_v7() always creates a valid v7 UUID, so unwrap() is safe here
        Self::try_new(Uuid::now_v7()).unwrap()
    }
}

impl Default for RequestId {
    fn default() -> Self {
        Self::new()
    }
}

/// Target URL for proxying
#[nutype(
    derive(Clone, Debug, Display, Deserialize, Serialize, TryFrom, AsRef),
    validate(predicate = |s: &str| s.starts_with("http://") || s.starts_with("https://")),
)]
pub struct TargetUrl(String);

/// API key for authentication
#[nutype(
    derive(Clone, Debug, Hash, PartialEq, Eq, Deserialize, Serialize, TryFrom, AsRef),
    validate(predicate = |s: &str| !s.is_empty()),
)]
pub struct ApiKey(String);

/// Session ID for tracking related requests
#[nutype(
    derive(Clone, Copy, Debug, Display, Deserialize, Serialize, TryFrom, AsRef),
    validate(predicate = |id: &Uuid| id.get_version_num() == 7),
)]
pub struct SessionId(Uuid);

impl SessionId {
    /// Create a new SessionId with a v7 UUID
    pub fn new() -> Self {
        // Uuid::now_v7() always creates a valid v7 UUID, so unwrap() is safe here
        Self::try_new(Uuid::now_v7()).unwrap()
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors that can occur in the proxy
#[derive(Error, Debug)]
pub enum ProxyError {
    #[error("Request too large: {size} bytes (max: {max_size} bytes)")]
    RequestTooLarge {
        size: BodySize,
        max_size: RequestSizeLimit,
    },

    #[error("Response too large: {size} bytes (max: {max_size} bytes)")]
    ResponseTooLarge {
        size: BodySize,
        max_size: ResponseSizeLimit,
    },

    #[error("Request timeout after {0:?}")]
    RequestTimeout(Duration),

    #[error("Invalid target URL: {0}")]
    InvalidTargetUrl(String),

    #[error("Ring buffer overflow: {dropped} events dropped")]
    RingBufferOverflow { dropped: DroppedEventCount },

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

    #[error("Invalid HTTP method: {0}")]
    InvalidHttpMethod(String),

    #[error("Invalid request URI: {0}")]
    InvalidRequestUri(String),

    #[error("Invalid HTTP status code: {0}")]
    InvalidHttpStatusCode(u16),

    #[error("Invalid header: {name}")]
    InvalidHeader { name: String },

    #[error("Failed to create audit event: {0}")]
    AuditEventCreationFailed(String),
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
        method: HttpMethod,
        uri: RequestUri,
        headers: Headers,
        body_size: BodySize,
    },
    RequestForwarded {
        target_url: TargetUrl,
        start_time: chrono::DateTime<chrono::Utc>,
    },
    ResponseReceived {
        status: HttpStatusCode,
        headers: Headers,
        body_size: BodySize,
        duration_ms: DurationMillis,
    },
    ResponseReturned {
        duration_ms: DurationMillis,
    },
    RequestBody {
        content: Vec<u8>,
        truncated: bool,
    },
    ResponseBody {
        content: Vec<u8>,
        truncated: bool,
    },
    RequestChunk {
        offset: ChunkOffset,
        data: Vec<u8>,
    },
    ResponseChunk {
        offset: ChunkOffset,
        data: Vec<u8>,
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

#[cfg(test)]
#[path = "error_handling_tests.rs"]
mod error_handling_tests;
