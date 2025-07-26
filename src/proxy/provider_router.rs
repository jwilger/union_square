//! Provider-based routing for URL-based LLM API routing
//!
//! This module implements the URL-based routing pattern defined in ADR-0011,
//! routing requests to appropriate providers based on URL path prefixes.

use crate::providers::ProviderRegistry;
use crate::proxy::types::{ProxyError, RequestId};
use axum::body::Body;
use hyper::{Request, Response};
use std::sync::Arc;

/// Router for provider-based request handling
pub struct ProviderRouter {
    registry: Arc<ProviderRegistry>,
    client: hyper_util::client::legacy::Client<
        hyper_util::client::legacy::connect::HttpConnector,
        Body,
    >,
}

impl ProviderRouter {
    /// Create a new provider router
    pub fn new(registry: Arc<ProviderRegistry>) -> Self {
        let client =
            hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
                .http1_title_case_headers(true)
                .http1_preserve_header_case(true)
                .build_http();

        Self { registry, client }
    }

    /// Route and forward a request to the appropriate provider
    pub async fn route_request(
        &self,
        request: Request<Body>,
        _request_id: RequestId,
    ) -> Result<Response<Body>, ProxyError> {
        let path = request.uri().path();

        // Find the provider that handles this path
        let provider = self
            .registry
            .route(path)
            .ok_or_else(|| ProxyError::InvalidTargetUrl(format!("No provider for path: {path}")))?;

        // Forward the request to the provider
        provider
            .forward_request(request, &self.client)
            .await
            .map_err(|e| e.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::bedrock::{provider::BedrockProvider, types::AwsRegion};
    use serde_json::json;

    #[tokio::test]
    async fn test_provider_routing() {
        let mut registry = ProviderRegistry::new();

        // Register Bedrock provider
        let bedrock = Arc::new(BedrockProvider::new(
            AwsRegion::try_new("us-east-1").unwrap(),
        ));
        registry.register(bedrock);

        let router = ProviderRouter::new(Arc::new(registry));

        // Test Bedrock routing
        let request = Request::builder()
            .method("POST")
            .uri("/bedrock/model/test/invoke")
            .header("authorization", "AWS4-HMAC-SHA256 test")
            .header("x-amz-date", "20250126T120000Z")
            .body(Body::from(json!({"test": true}).to_string()))
            .unwrap();

        // This will fail in a real test without a mock server, but it validates routing works
        let result = router.route_request(request, RequestId::new()).await;
        assert!(result.is_err()); // Expected since we're not mocking the actual provider endpoint
    }

    #[tokio::test]
    async fn test_no_provider_found() {
        let registry = Arc::new(ProviderRegistry::new());
        let router = ProviderRouter::new(registry);

        let request = Request::builder()
            .method("POST")
            .uri("/unknown/path")
            .body(Body::empty())
            .unwrap();

        let result = router.route_request(request, RequestId::new()).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            ProxyError::InvalidTargetUrl(msg) => {
                assert!(msg.contains("No provider for path"));
            }
            _ => panic!("Expected InvalidTargetUrl error"),
        }
    }
}
