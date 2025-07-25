//! Shared audit recording functionality for streaming implementations

use crate::proxy::ring_buffer::RingBuffer;
use crate::proxy::types::*;
use bytes::Bytes;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Size of chunks for streaming capture
pub const CAPTURE_CHUNK_SIZE: usize = 16 * 1024; // 16KB chunks

/// Trait for common audit recording operations
pub trait AuditRecorder {
    /// Record a request received event
    fn record_request_event(
        &self,
        request_id: RequestId,
        method: Result<HttpMethod, String>,
        uri: Result<RequestUri, String>,
        headers: Vec<(String, String)>,
        body_size: BodySize,
    );

    /// Record a response received event
    fn record_response_event(
        &self,
        request_id: RequestId,
        status: Result<HttpStatusCode, u16>,
        headers: Vec<(String, String)>,
        body_size: BodySize,
        duration_ms: DurationMillis,
    );

    /// Record an error event
    fn record_error_event(&self, request_id: RequestId, error: String, phase: ErrorPhase);

    /// Record a chunk event (request or response)
    fn record_chunk_event(
        &self,
        request_id: RequestId,
        offset: ChunkOffset,
        data: Vec<u8>,
        is_request: bool,
    );
}

/// Default implementation of audit recording using ring buffer
pub struct RingBufferAuditRecorder {
    ring_buffer: Arc<RingBuffer>,
}

impl RingBufferAuditRecorder {
    pub fn new(ring_buffer: Arc<RingBuffer>) -> Self {
        Self { ring_buffer }
    }
}

impl AuditRecorder for RingBufferAuditRecorder {
    fn record_request_event(
        &self,
        request_id: RequestId,
        method: Result<HttpMethod, String>,
        uri: Result<RequestUri, String>,
        headers: Vec<(String, String)>,
        body_size: BodySize,
    ) {
        let event_type = match (method, uri) {
            (Ok(method), Ok(uri)) => AuditEventType::RequestReceived {
                method,
                uri,
                headers: Headers::from_vec(headers).unwrap_or_default(),
                body_size,
            },
            (Err(method_err), _) => AuditEventType::Error {
                error: method_err,
                phase: ErrorPhase::RequestParsing,
            },
            (_, Err(uri_err)) => AuditEventType::Error {
                error: uri_err,
                phase: ErrorPhase::RequestParsing,
            },
        };

        self.write_audit_event(request_id, event_type);
    }

    fn record_response_event(
        &self,
        request_id: RequestId,
        status: Result<HttpStatusCode, u16>,
        headers: Vec<(String, String)>,
        body_size: BodySize,
        duration_ms: DurationMillis,
    ) {
        let event_type = match status {
            Ok(status) => AuditEventType::ResponseReceived {
                status,
                headers: Headers::from_vec(headers).unwrap_or_default(),
                body_size,
                duration_ms,
            },
            Err(invalid_status) => AuditEventType::Error {
                error: format!(
                    "Invalid HTTP status code '{invalid_status}' received from upstream"
                ),
                phase: ErrorPhase::ResponseReceiving,
            },
        };

        self.write_audit_event(request_id, event_type);
    }

    fn record_error_event(&self, request_id: RequestId, error: String, phase: ErrorPhase) {
        let event_type = AuditEventType::Error { error, phase };
        self.write_audit_event(request_id, event_type);
    }

    fn record_chunk_event(
        &self,
        request_id: RequestId,
        offset: ChunkOffset,
        data: Vec<u8>,
        is_request: bool,
    ) {
        let event_type = if is_request {
            AuditEventType::RequestChunk { offset, data }
        } else {
            AuditEventType::ResponseChunk { offset, data }
        };

        self.write_audit_event(request_id, event_type);
    }
}

impl RingBufferAuditRecorder {
    /// Write an audit event to the ring buffer
    fn write_audit_event(&self, request_id: RequestId, event_type: AuditEventType) {
        let audit_event = AuditEvent {
            request_id,
            session_id: SessionId::new(),
            timestamp: chrono::Utc::now(),
            event_type,
        };

        // Fire-and-forget write to ring buffer
        if let Ok(serialized) = serde_json::to_vec(&audit_event) {
            let _ = self.ring_buffer.write(request_id, &serialized);
        }
    }
}

/// Chunked data capture functionality
pub struct ChunkCapture {
    recorder: Arc<dyn AuditRecorder + Send + Sync>,
    request_id: RequestId,
    is_request: bool,
}

impl ChunkCapture {
    pub fn new(
        recorder: Arc<dyn AuditRecorder + Send + Sync>,
        request_id: RequestId,
        is_request: bool,
    ) -> Self {
        Self {
            recorder,
            request_id,
            is_request,
        }
    }

    /// Start a chunk capture task that receives chunks via mpsc and writes to audit
    pub fn start_capture_task(&self) -> mpsc::Sender<Bytes> {
        let (tx, mut rx) = mpsc::channel::<Bytes>(16);
        let recorder = self.recorder.clone();
        let request_id = self.request_id;
        let is_request = self.is_request;

        tokio::spawn(async move {
            let mut buffer = Vec::with_capacity(CAPTURE_CHUNK_SIZE);
            let mut total_size = 0usize;

            while let Some(chunk) = rx.recv().await {
                buffer.extend_from_slice(&chunk);
                total_size += chunk.len();

                // Write to ring buffer when we have enough data
                if buffer.len() >= CAPTURE_CHUNK_SIZE {
                    let offset = ChunkOffset::from(total_size - buffer.len());
                    recorder.record_chunk_event(request_id, offset, buffer.clone(), is_request);
                    buffer.clear();
                }
            }

            // Write any remaining data when stream ends
            if !buffer.is_empty() {
                let offset = ChunkOffset::from(total_size - buffer.len());
                recorder.record_chunk_event(request_id, offset, buffer, is_request);
            }
        });

        tx
    }
}

/// Helper function to extract headers from HTTP headers map
pub fn extract_headers_vec(headers: &hyper::HeaderMap) -> Vec<(String, String)> {
    headers
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("<binary>").to_string()))
        .collect()
}

/// Helper function to parse HTTP method with detailed error message
pub fn parse_http_method(method: &hyper::Method) -> Result<HttpMethod, String> {
    HttpMethod::try_new(method.to_string())
        .map_err(|e| format!("Invalid HTTP method '{method}': {e}"))
}

/// Helper function to parse request URI with detailed error message
pub fn parse_request_uri(uri: &hyper::Uri) -> Result<RequestUri, String> {
    RequestUri::try_new(uri.to_string()).map_err(|e| format!("Invalid request URI '{uri}': {e}"))
}

/// Helper function to parse HTTP status code
pub fn parse_http_status(status: hyper::StatusCode) -> Result<HttpStatusCode, u16> {
    HttpStatusCode::try_new(status.as_u16()).map_err(|_| status.as_u16())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_recorder_creation() {
        let config = crate::proxy::types::RingBufferConfig::default();
        let ring_buffer = Arc::new(RingBuffer::new(&config));
        let recorder = RingBufferAuditRecorder::new(ring_buffer);

        // Test that we can record events without panicking
        recorder.record_error_event(
            RequestId::new(),
            "Test error".to_string(),
            ErrorPhase::RequestParsing,
        );
    }

    #[test]
    fn test_chunk_capture_creation() {
        let config = crate::proxy::types::RingBufferConfig::default();
        let ring_buffer = Arc::new(RingBuffer::new(&config));
        let recorder = Arc::new(RingBufferAuditRecorder::new(ring_buffer));

        let _capture = ChunkCapture::new(recorder, RequestId::new(), true);
        // Test successful creation
    }

    #[test]
    fn test_helper_functions() {
        let method = hyper::Method::GET;
        let uri: hyper::Uri = "https://example.com/test".parse().unwrap();
        let status = hyper::StatusCode::OK;

        // Test parsing functions
        assert!(parse_http_method(&method).is_ok());
        assert!(parse_request_uri(&uri).is_ok());
        assert!(parse_http_status(status).is_ok());
    }
}
