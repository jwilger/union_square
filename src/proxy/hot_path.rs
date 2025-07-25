//! Hot path implementation for minimal-latency request forwarding
//!
//! This module implements the streaming hot path that provides <5ms latency
//! for request forwarding while capturing audit data asynchronously.

use crate::proxy::audit_recorder::{
    extract_headers_vec, parse_http_method, parse_http_status, parse_request_uri, AuditRecorder,
    RingBufferAuditRecorder,
};
use crate::proxy::ring_buffer::RingBuffer;
use crate::proxy::types::*;
use crate::proxy::url_resolver::UrlResolver;
use axum::body::Body;
use http_body_util::BodyExt;
use hyper::{Request, Response};
use std::sync::Arc;
use std::time::Instant;

/// Streaming hot path service for zero-copy forwarding
#[derive(Clone)]
pub struct StreamingHotPathService {
    config: Arc<ProxyConfig>,
    audit_recorder: Arc<RingBufferAuditRecorder>,
    client: hyper_util::client::legacy::Client<
        hyper_util::client::legacy::connect::HttpConnector,
        Body,
    >,
}

impl StreamingHotPathService {
    /// Create a new streaming hot path service
    pub fn new(config: ProxyConfig, ring_buffer: Arc<RingBuffer>) -> Self {
        let client =
            hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
                .http1_title_case_headers(true)
                .http1_preserve_header_case(true)
                .build_http();

        let audit_recorder = Arc::new(RingBufferAuditRecorder::new(ring_buffer));

        Self {
            config: Arc::new(config),
            audit_recorder,
            client,
        }
    }

    /// Forward a request to the target URL with streaming
    pub async fn forward_request(
        &self,
        request: Request<Body>,
        target_url: TargetUrl,
        request_id: RequestId,
    ) -> ProxyResult<Response<Body>> {
        let start_time = Instant::now();

        // Extract parts from the incoming request
        let (mut parts, body) = request.into_parts();

        // Resolve the target URI using centralized strategy
        let resolved_uri = UrlResolver::resolve_target_uri(&target_url, &parts.uri)?;
        parts.uri = resolved_uri;

        // Record request metadata using shared audit recorder
        let headers_vec = extract_headers_vec(&parts.headers);
        let method_result = parse_http_method(&parts.method);
        let uri_result = parse_request_uri(&parts.uri);

        self.audit_recorder.record_request_event(
            request_id,
            method_result,
            uri_result,
            headers_vec,
            BodySize::from(0), // We don't know the size in streaming mode
        );

        // Apply request size limit by collecting the body first
        // TODO: This is a temporary implementation for MVP - we should implement true streaming size limits
        let body_bytes = http_body_util::Limited::new(body, *self.config.max_request_size.as_ref())
            .collect()
            .await
            .map_err(|e| {
                if e.is::<http_body_util::LengthLimitError>() {
                    ProxyError::RequestTooLarge {
                        size: BodySize::from(*self.config.max_request_size.as_ref() + 1),
                        max_size: self.config.max_request_size,
                    }
                } else {
                    ProxyError::Internal(format!("Body collection error: {e}"))
                }
            })?;

        // Create outgoing request with the collected body
        let outgoing_request = Request::from_parts(parts, Body::from(body_bytes.to_bytes()));

        // Forward the request with timeout
        let response_future = self.client.request(outgoing_request);
        let timeout_duration = self.config.request_timeout;

        let response = tokio::time::timeout(timeout_duration, response_future)
            .await
            .map_err(|_| ProxyError::RequestTimeout(timeout_duration))?
            .map_err(|e| ProxyError::Internal(format!("Connection error: {e}")))?;

        let duration_ms = start_time.elapsed().as_millis() as u64;

        // Extract response parts
        let (response_parts, response_body) = response.into_parts();

        // Record response metadata using shared audit recorder
        let headers_vec = extract_headers_vec(&response_parts.headers);
        let status_result = parse_http_status(response_parts.status);

        self.audit_recorder.record_response_event(
            request_id,
            status_result,
            headers_vec,
            BodySize::from(0), // We don't know the size in streaming mode
            DurationMillis::from(duration_ms),
        );

        // For MVP, return the streaming response as-is
        // TODO: Add chunked capture to ring buffer while streaming
        Ok(Response::from_parts(
            response_parts,
            Body::new(response_body),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proxy::types::ProxyConfig;

    #[tokio::test]
    async fn test_streaming_hot_path_creation() {
        let config = ProxyConfig::default();
        let ring_buffer = Arc::new(RingBuffer::new(&config.ring_buffer));
        let service = StreamingHotPathService::new(config, ring_buffer);

        // Service should be created successfully
        let _ = service;
    }
}
