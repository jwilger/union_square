//! AWS Bedrock provider implementation

use crate::providers::bedrock::{
    auth::{extract_sigv4_headers, validate_sigv4_auth},
    models::extract_model_id,
    types::{AwsRegion, ModelFamily, ModelPricing},
};
use crate::providers::{HealthStatus, Provider, ProviderError, ProviderMetadata};
use async_trait::async_trait;
use axum::body::Body;
use hyper::{Request, Response, Uri};

/// AWS Bedrock provider
pub struct BedrockProvider {
    base_url: String,
}

impl BedrockProvider {
    /// Create a new Bedrock provider
    pub fn new(region: AwsRegion) -> Self {
        let base_url = format!("https://bedrock-runtime.{}.amazonaws.com", region.as_ref());
        Self { base_url }
    }

    /// Create a new Bedrock provider with a custom base URL (for testing)
    pub fn with_base_url(base_url: String) -> Self {
        Self { base_url }
    }

    /// Build the target URL for Bedrock API
    fn build_target_url(&self, path: &str) -> Result<Uri, ProviderError> {
        // Remove /bedrock prefix from path
        let bedrock_path = path
            .strip_prefix("/bedrock")
            .ok_or_else(|| ProviderError::InvalidPath("Missing /bedrock prefix".to_string()))?;

        let target_url = format!("{}{}", self.base_url, bedrock_path);

        target_url
            .parse()
            .map_err(|_| ProviderError::InvalidUrl(target_url))
    }
}

#[async_trait]
impl Provider for BedrockProvider {
    fn id(&self) -> &'static str {
        "bedrock"
    }

    fn matches_path(&self, path: &str) -> bool {
        path.starts_with("/bedrock/")
    }

    fn transform_url(&self, url: &Uri) -> Result<Uri, ProviderError> {
        self.build_target_url(url.path())
    }

    async fn forward_request(
        &self,
        request: Request<Body>,
        client: &hyper_util::client::legacy::Client<
            hyper_util::client::legacy::connect::HttpConnector,
            Body,
        >,
    ) -> Result<Response<Body>, ProviderError> {
        let (mut parts, body) = request.into_parts();

        // Validate and extract SigV4 headers
        validate_sigv4_auth(&parts.headers)?;
        let auth_headers = extract_sigv4_headers(&parts.headers)?;

        // Transform the URL
        let target_uri = self.transform_url(&parts.uri)?;
        parts.uri = target_uri;

        // Merge auth headers into request headers
        for (name, value) in auth_headers.iter() {
            parts.headers.insert(name.clone(), value.clone());
        }

        // Rebuild the request
        let request = Request::from_parts(parts, body);

        // Forward the request
        let response = client
            .request(request)
            .await
            .map_err(|e| ProviderError::RequestFailed(format!("Request failed: {e}")))?;

        // Convert response body from Incoming to Body
        let (parts, incoming_body) = response.into_parts();
        let body = Body::new(incoming_body);

        Ok(Response::from_parts(parts, body))
    }

    fn extract_metadata(
        &self,
        request: &Request<Body>,
        _response: &Response<Body>,
    ) -> ProviderMetadata {
        let mut metadata = ProviderMetadata {
            provider_id: self.id().to_string(),
            ..Default::default()
        };

        // Extract model ID from request path
        if let Some(model_id) = extract_model_id(request.uri().path()) {
            metadata.model_id = Some(model_id.clone());

            // Determine model family for token extraction
            let _model_family = ModelFamily::from_model_id(&model_id);

            // Note: Actual token extraction happens in process_response_body
            // This method just provides basic metadata

            // Calculate cost estimate if we have pricing info
            if let Some(_pricing) = ModelPricing::for_model(&model_id) {
                // Cost will be calculated when we have token counts
                // For now, just indicate that pricing is available
            }
        }

        metadata
    }

    async fn health_check(
        &self,
        _client: &hyper_util::client::legacy::Client<
            hyper_util::client::legacy::connect::HttpConnector,
            Body,
        >,
    ) -> HealthStatus {
        // For MVP, we assume Bedrock is healthy
        // Future enhancement: Implement actual health check
        HealthStatus::Healthy
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_id() {
        let provider = BedrockProvider::new(AwsRegion::try_new("us-east-1").unwrap());
        assert_eq!(provider.id(), "bedrock");
    }

    #[test]
    fn test_matches_path() {
        let provider = BedrockProvider::new(AwsRegion::try_new("us-east-1").unwrap());

        assert!(provider.matches_path("/bedrock/model/claude-3/invoke"));
        assert!(provider.matches_path("/bedrock/"));
        assert!(!provider.matches_path("/openai/v1/chat"));
        assert!(!provider.matches_path("/anthropic/v1/messages"));
    }

    #[test]
    fn test_build_target_url() {
        let provider = BedrockProvider::new(AwsRegion::try_new("us-west-2").unwrap());

        let url = provider
            .build_target_url("/bedrock/model/claude-3/invoke")
            .unwrap();
        assert_eq!(
            url.to_string(),
            "https://bedrock-runtime.us-west-2.amazonaws.com/model/claude-3/invoke"
        );
    }

    #[test]
    fn test_transform_url() {
        let provider = BedrockProvider::new(AwsRegion::try_new("eu-west-1").unwrap());

        let input_uri: Uri = "/bedrock/model/titan/invoke".parse().unwrap();
        let output_uri = provider.transform_url(&input_uri).unwrap();

        assert_eq!(
            output_uri.to_string(),
            "https://bedrock-runtime.eu-west-1.amazonaws.com/model/titan/invoke"
        );
    }
}
