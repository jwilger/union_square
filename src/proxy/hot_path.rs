//! Hot path implementation for minimal-latency request forwarding
//!
//! This module contains a non-streaming implementation used for testing
//! and performance comparisons. The production code uses StreamingHotPathService.

#[cfg(test)]
use crate::proxy::types::*;
#[cfg(test)]
use bytes::Bytes;
#[cfg(test)]
use http_body_util::{BodyExt, Full};
#[cfg(test)]
use hyper::{Request, Response};
#[cfg(test)]
use std::sync::Arc;

/// Hot path service for forwarding requests with minimal overhead
/// (Test implementation - production code uses StreamingHotPathService)
#[cfg(test)]
#[derive(Clone)]
pub struct HotPathService {
    config: Arc<ProxyConfig>,
    client: hyper_util::client::legacy::Client<
        hyper_util::client::legacy::connect::HttpConnector,
        Full<Bytes>,
    >,
}

#[cfg(test)]
impl HotPathService {
    /// Create a new hot path service
    pub fn new(config: ProxyConfig) -> Self {
        let client =
            hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
                .build_http();

        Self {
            config: Arc::new(config),
            client,
        }
    }

    /// Forward a request to the target URL
    pub async fn forward_request<B>(
        &self,
        request: Request<B>,
        target_url: TargetUrl,
    ) -> ProxyResult<Response<Full<Bytes>>>
    where
        B: http_body::Body + Send + 'static,
        B::Data: Send,
        B::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    {
        // Extract parts from the incoming request
        let (mut parts, body) = request.into_parts();

        // Update the URI with the target URL
        let _target_uri = target_url
            .as_ref()
            .parse::<hyper::Uri>()
            .map_err(|_| ProxyError::InvalidTargetUrl(target_url.as_ref().to_string()))?;

        // Preserve the path and query from the original request
        let path_and_query = parts
            .uri
            .path_and_query()
            .map(|pq| pq.as_str())
            .unwrap_or("/");

        // Build the full target URI
        let full_uri = format!(
            "{}{}",
            target_url.as_ref().trim_end_matches('/'),
            path_and_query
        );
        parts.uri = full_uri
            .parse()
            .map_err(|_| ProxyError::InvalidTargetUrl(full_uri))?;

        // Collect the body into bytes (with size limit)
        let body_bytes = http_body_util::Limited::new(body, self.config.max_request_size)
            .collect()
            .await
            .map_err(|e| {
                if e.is::<http_body_util::LengthLimitError>() {
                    ProxyError::RequestTooLarge {
                        size: self.config.max_request_size + 1,
                        max_size: self.config.max_request_size,
                    }
                } else {
                    ProxyError::Internal(format!("Failed to read request body: {e}"))
                }
            })?
            .to_bytes();

        // Create the outgoing request
        let outgoing_request = Request::from_parts(parts, Full::new(body_bytes));

        // Forward the request with timeout
        let response_future = self.client.request(outgoing_request);
        let timeout_duration = self.config.request_timeout;

        let response = tokio::time::timeout(timeout_duration, response_future)
            .await
            .map_err(|_| ProxyError::RequestTimeout(timeout_duration))?
            .map_err(|e| ProxyError::Internal(format!("Client error: {e}")))?;

        // Extract response parts
        let (response_parts, response_body) = response.into_parts();

        // Collect response body with size limit
        let response_bytes =
            http_body_util::Limited::new(response_body, self.config.max_response_size)
                .collect()
                .await
                .map_err(|e| {
                    if e.is::<http_body_util::LengthLimitError>() {
                        ProxyError::ResponseTooLarge {
                            size: self.config.max_response_size + 1,
                            max_size: self.config.max_response_size,
                        }
                    } else {
                        ProxyError::Internal(format!("Failed to read response body: {e}"))
                    }
                })?
                .to_bytes();

        // Return the response
        Ok(Response::from_parts(
            response_parts,
            Full::new(response_bytes),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proxy::types::{ProxyConfig, TargetUrl};
    use bytes::Bytes;
    use http_body_util::{BodyExt, Empty};

    #[tokio::test]
    async fn test_hot_path_service_creation() {
        let config = ProxyConfig::default();
        let service = HotPathService::new(config);

        // Service should be created successfully
        let _ = service;
    }

    #[tokio::test]
    async fn test_forward_request_basic() {
        let config = ProxyConfig::default();
        let service = HotPathService::new(config);

        // Create a mock request
        let empty_body = Empty::<Bytes>::new();
        let request = Request::builder()
            .method("GET")
            .uri("/test")
            .body(empty_body.boxed())
            .unwrap();

        // We can't test actual forwarding without a mock server
        // So we'll just test that the request is processed without panicking
        let target_url = TargetUrl::try_new("http://localhost:9999").unwrap();

        // This will fail with connection refused, which is expected
        let result = service.forward_request(request, target_url).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_forward_request_with_invalid_target_url() {
        let config = ProxyConfig::default();
        let _service = HotPathService::new(config);

        // Create a mock request
        let _request = Request::builder()
            .method("POST")
            .uri("/test")
            .body(Empty::<Bytes>::new().boxed())
            .unwrap();

        // Try with invalid URL (this should be caught at type level)
        let target_url_result = TargetUrl::try_new("not-a-url");
        assert!(target_url_result.is_err());
    }

    #[tokio::test]
    async fn test_request_size_limits() {
        let config = ProxyConfig {
            max_request_size: 1024, // 1KB limit
            ..Default::default()
        };
        let service = HotPathService::new(config);

        // TODO: Test request size validation once implemented
        let _ = service;
    }

    #[tokio::test]
    async fn test_response_size_limits() {
        let config = ProxyConfig {
            max_response_size: 1024, // 1KB limit
            ..Default::default()
        };
        let service = HotPathService::new(config);

        // TODO: Test response size validation once implemented
        let _ = service;
    }

    #[tokio::test]
    async fn test_request_timeout() {
        use std::time::Duration;

        let config = ProxyConfig {
            request_timeout: Duration::from_millis(100),
            ..Default::default()
        };
        let service = HotPathService::new(config);

        // TODO: Test timeout handling once implemented
        let _ = service;
    }

    // Performance tests
    #[tokio::test]
    async fn test_hot_path_latency_target() {
        use std::time::Instant;

        let config = ProxyConfig::default();
        let service = HotPathService::new(config);

        let request = Request::builder()
            .method("GET")
            .uri("/test")
            .body(Empty::<Bytes>::new().boxed())
            .unwrap();

        let target_url = TargetUrl::try_new("http://localhost:9999").unwrap();

        // Measure latency
        let start = Instant::now();
        let _ = service.forward_request(request, target_url).await;
        let duration = start.elapsed();

        // Even with connection refused, should fail quickly
        assert!(duration.as_millis() < 100); // Less than 100ms
    }

    #[tokio::test]
    async fn test_concurrent_requests() {
        use futures_util::future::join_all;

        let config = ProxyConfig::default();
        let service = HotPathService::new(config);

        // Create multiple concurrent requests
        let mut futures = vec![];

        for i in 0..10 {
            let service_clone = service.clone();
            let future = async move {
                let request = Request::builder()
                    .method("GET")
                    .uri(format!("/test/{i}"))
                    .body(Empty::<Bytes>::new().boxed())
                    .unwrap();

                let target_url = TargetUrl::try_new("http://localhost:9999").unwrap();
                service_clone.forward_request(request, target_url).await
            };
            futures.push(future);
        }

        // Execute all requests concurrently
        let results = join_all(futures).await;

        // All should fail with connection error (expected)
        for result in results {
            assert!(result.is_err());
        }
    }
}
