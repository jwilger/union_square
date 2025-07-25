//! Simplified streaming implementation for zero-copy request/response forwarding

use crate::proxy::ring_buffer::RingBuffer;
use crate::proxy::types::*;
use axum::body::Body;
use hyper::{Request, Response};
use std::sync::Arc;
use std::time::Instant;

/// Streaming hot path service for zero-copy forwarding
#[derive(Clone)]
pub struct StreamingHotPathService {
    config: Arc<ProxyConfig>,
    ring_buffer: Arc<RingBuffer>,
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

        Self {
            config: Arc::new(config),
            ring_buffer,
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

        // Update the URI with the target URL
        let path_and_query = parts
            .uri
            .path_and_query()
            .map(|pq| pq.as_str())
            .unwrap_or("/");

        let full_uri = format!(
            "{}{}",
            target_url.as_ref().trim_end_matches('/'),
            path_and_query
        );
        parts.uri = full_uri
            .parse()
            .map_err(|_| ProxyError::InvalidTargetUrl(full_uri))?;

        // Record request metadata in ring buffer
        let headers_vec: Vec<(String, String)> = parts
            .headers
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("<binary>").to_string()))
            .collect();

        let request_event = AuditEvent {
            request_id,
            session_id: SessionId::new(),
            timestamp: chrono::Utc::now(),
            event_type: AuditEventType::RequestReceived {
                method: HttpMethod::try_new(parts.method.to_string())
                    .unwrap_or_else(|_| HttpMethod::try_new("UNKNOWN".to_string()).unwrap()),
                uri: RequestUri::try_new(parts.uri.to_string())
                    .unwrap_or_else(|_| RequestUri::try_new("/".to_string()).unwrap()),
                headers: Headers::from_vec(headers_vec).unwrap_or_default(),
                body_size: BodySize::from(0), // We don't know the size in streaming mode
            },
        };

        // Fire-and-forget write to ring buffer
        if let Ok(serialized) = serde_json::to_vec(&request_event) {
            let _ = self.ring_buffer.write(request_id, &serialized);
        }

        // Create outgoing request with the streaming body
        // For MVP, we forward the body as-is without capturing chunks
        let outgoing_request = Request::from_parts(parts, body);

        // Forward the request with timeout
        let response_future = self.client.request(outgoing_request);
        let timeout_duration = self.config.request_timeout;

        let response = tokio::time::timeout(timeout_duration, response_future)
            .await
            .map_err(|_| ProxyError::RequestTimeout(timeout_duration))?
            .map_err(|e| ProxyError::Internal(format!("Client error: {e}")))?;

        let duration_ms = start_time.elapsed().as_millis() as u64;

        // Extract response parts
        let (response_parts, response_body) = response.into_parts();

        // Record response metadata
        let headers_vec: Vec<(String, String)> = response_parts
            .headers
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("<binary>").to_string()))
            .collect();

        let response_event = AuditEvent {
            request_id,
            session_id: SessionId::new(),
            timestamp: chrono::Utc::now(),
            event_type: AuditEventType::ResponseReceived {
                status: HttpStatusCode::try_new(response_parts.status.as_u16())
                    .unwrap_or_else(|_| HttpStatusCode::try_new(500).unwrap()),
                headers: Headers::from_vec(headers_vec).unwrap_or_default(),
                body_size: BodySize::from(0), // We don't know the size in streaming mode
                duration_ms: DurationMillis::from(duration_ms),
            },
        };

        // Fire-and-forget write to ring buffer
        if let Ok(serialized) = serde_json::to_vec(&response_event) {
            let _ = self.ring_buffer.write(request_id, &serialized);
        }

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
