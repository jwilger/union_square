# Provider Integration Guide

This guide explains how to integrate new LLM providers with Union Square following the established provider abstraction pattern.

## Table of Contents

- [Overview](#overview)
- [Provider Architecture](#provider-architecture)
- [AWS Bedrock Integration](#aws-bedrock-integration)
- [Adding New Providers](#adding-new-providers)
- [Testing Providers](#testing-providers)
- [Performance Considerations](#performance-considerations)

## Overview

Union Square uses a provider abstraction layer to support multiple LLM APIs while maintaining:
- URL-based routing (per ADR-0011)
- Zero-copy streaming for minimal latency
- Authentication pass-through
- Request/response recording for analysis
- Token usage tracking and cost calculation

## Provider Architecture

### Core Traits

```rust
#[async_trait]
pub trait Provider: Send + Sync {
    /// Provider identifier (openai, anthropic, bedrock, etc.)
    fn id(&self) -> &'static str;

    /// Check if this provider handles the given path
    fn matches_path(&self, path: &str) -> bool;

    /// Transform Union Square URL to provider URL
    fn transform_url(&self, url: &hyper::Uri) -> Result<hyper::Uri, ProviderError>;

    /// Validate and forward the request
    async fn forward_request(
        &self,
        request: Request<Body>,
        client: &HttpClient,
    ) -> Result<Response<Body>, ProviderError>;

    /// Extract metadata for audit logging
    fn extract_metadata(
        &self,
        request: &Request<Body>,
        response: &Response<Body>,
    ) -> ProviderMetadata;

    /// Provider-specific health check
    async fn health_check(&self, client: &HttpClient) -> HealthStatus;
}
```

### Provider Registry

The `ProviderRegistry` manages all available providers and routes requests based on URL path:

```rust
let mut registry = ProviderRegistry::new();
registry.register(Arc::new(BedrockProvider::new(region)));
registry.register(Arc::new(OpenAIProvider::new()));
// ... register other providers

// Route request to appropriate provider
if let Some(provider) = registry.route(request.uri().path()) {
    let response = provider.forward_request(request, &client).await?;
}
```

## AWS Bedrock Integration

The Bedrock provider demonstrates the MVP implementation pattern for new providers.

### Key Features

1. **URL Routing**: Paths starting with `/bedrock/` are routed to this provider
2. **SigV4 Authentication**: Pass-through without storing credentials
3. **Model Support**: Claude, Titan, Llama, and other Bedrock models
4. **Streaming**: Zero-copy streaming for `/invoke-with-response-stream` endpoint
5. **Cost Tracking**: Automatic cost calculation based on token usage

### Implementation Example

```rust
// Provider implementation
pub struct BedrockProvider {
    base_url: String,
}

impl BedrockProvider {
    pub fn new(region: AwsRegion) -> Self {
        let base_url = format!("https://bedrock-runtime.{}.amazonaws.com", region.as_ref());
        Self { base_url }
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
        // Remove /bedrock prefix and build target URL
        let bedrock_path = path.strip_prefix("/bedrock")
            .ok_or_else(|| ProviderError::InvalidPath("Missing /bedrock prefix".to_string()))?;

        let target_url = format!("{}{}", self.base_url, bedrock_path);
        target_url.parse()
            .map_err(|_| ProviderError::InvalidUrl(target_url))
    }

    async fn forward_request(
        &self,
        request: Request<Body>,
        client: &HttpClient,
    ) -> Result<Response<Body>, ProviderError> {
        // Validate authentication
        validate_sigv4_auth(&request.headers())?;

        // Transform URL and forward
        let (mut parts, body) = request.into_parts();
        parts.uri = self.transform_url(&parts.uri)?;

        let request = Request::from_parts(parts, body);
        client.request(request).await
            .map_err(|e| ProviderError::RequestFailed(e.to_string()))
    }

    fn extract_metadata(
        &self,
        request: &Request<Body>,
        response: &Response<Body>,
    ) -> ProviderMetadata {
        // Extract model ID, request ID, etc.
        let mut metadata = ProviderMetadata {
            provider_id: self.id().to_string(),
            ..Default::default()
        };

        if let Some(model_id) = extract_model_id(request.uri().path()) {
            metadata.model_id = Some(model_id);
        }

        if let Some(request_id) = response.headers().get("x-amzn-requestid") {
            if let Ok(id) = request_id.to_str() {
                metadata.provider_request_id = Some(id.to_string());
            }
        }

        metadata
    }
}
```

### Response Processing

Extract token usage and calculate costs from response bodies:

```rust
let processor = ProviderResponseProcessor::new(base_metadata);
let final_metadata = processor.process_complete_body(&response_body);

// Metadata now includes:
// - request_tokens
// - response_tokens
// - total_tokens
// - cost_estimate
```

## Adding New Providers

To add a new provider (e.g., OpenAI):

### 1. Create Provider Module

```rust
// src/providers/openai/mod.rs
pub mod auth;
pub mod models;
pub mod provider;
pub mod types;

pub use provider::OpenAIProvider;
```

### 2. Define Types

```rust
// src/providers/openai/types.rs
#[derive(Debug, Clone)]
pub enum OpenAIModel {
    Gpt4,
    Gpt4Turbo,
    Gpt35Turbo,
}

#[derive(Debug, Clone)]
pub struct ModelPricing {
    pub input_price_per_1k_tokens: f64,
    pub output_price_per_1k_tokens: f64,
}
```

### 3. Implement Provider Trait

```rust
pub struct OpenAIProvider {
    api_key: Option<String>, // Optional if using header pass-through
}

#[async_trait]
impl Provider for OpenAIProvider {
    fn id(&self) -> &'static str {
        "openai"
    }

    fn matches_path(&self, path: &str) -> bool {
        path.starts_with("/openai/")
    }

    // ... implement other trait methods
}
```

### 4. Handle Authentication

```rust
fn validate_auth(headers: &HeaderMap) -> Result<(), ProviderError> {
    if !headers.contains_key("authorization") {
        return Err(ProviderError::AuthenticationError(
            "Missing Authorization header".to_string()
        ));
    }
    Ok(())
}
```

### 5. Register Provider

```rust
// In main application setup
let openai_provider = Arc::new(OpenAIProvider::new());
registry.register(openai_provider);
```

## Testing Providers

### Unit Tests

Test each provider component in isolation:

```rust
#[test]
fn test_url_transformation() {
    let provider = OpenAIProvider::new();
    let input_uri: Uri = "/openai/v1/chat/completions".parse().unwrap();
    let output_uri = provider.transform_url(&input_uri).unwrap();
    assert_eq!(
        output_uri.to_string(),
        "https://api.openai.com/v1/chat/completions"
    );
}
```

### Integration Tests

Test the complete flow with mocked responses:

```rust
#[tokio::test]
async fn test_complete_request_flow() {
    let provider = OpenAIProvider::new();
    let client = create_mock_client(); // Returns mocked responses

    let request = create_test_request(
        "/openai/v1/chat/completions",
        json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Hello"}]
        })
    );

    let response = provider.forward_request(request, &client).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Verify metadata extraction
    let metadata = provider.extract_metadata(&request, &response);
    assert_eq!(metadata.model_id, Some("gpt-4".to_string()));
}
```

### Testing Checklist

- [ ] URL routing and transformation
- [ ] Authentication validation
- [ ] Request forwarding
- [ ] Response processing
- [ ] Error handling
- [ ] Streaming responses
- [ ] Metadata extraction
- [ ] Cost calculation

## Performance Considerations

### Zero-Copy Streaming

Providers should pass through streaming responses without buffering:

```rust
// Good: Direct pass-through
let response = client.request(request).await?;
Ok(response) // Body streams directly to client

// Bad: Buffering entire response
let response = client.request(request).await?;
let body = response.collect().await?.to_bytes();
// Process body...
Ok(Response::new(Body::from(body)))
```

### Connection Pooling

Use shared HTTP client with connection pooling:

```rust
// Share client across all providers
let client = Client::builder()
    .pool_idle_timeout(Duration::from_secs(30))
    .pool_max_idle_per_host(10)
    .build(HttpConnector::new());
```

### Async Processing

Process metadata extraction asynchronously when possible:

```rust
// Extract basic metadata immediately
let metadata = provider.extract_metadata(&request, &response);

// Process response body asynchronously for token counting
tokio::spawn(async move {
    let processor = ProviderResponseProcessor::new(metadata);
    let final_metadata = processor.process_complete_body(&body);
    // Record final metadata
});
```

## Provider-Specific Considerations

### AWS Bedrock

- Requires SigV4 authentication
- Different response formats per model family
- Region-specific endpoints

### OpenAI

- API key authentication (Bearer token)
- Consistent response format across models
- Rate limiting headers to track

### Anthropic

- API key authentication (X-API-Key header)
- Streaming uses Server-Sent Events
- Version header required

### Google Vertex AI

- OAuth2 authentication
- Project ID in URL path
- Regional endpoints

## Best Practices

1. **Type Safety**: Use newtypes for provider-specific types
2. **Error Handling**: Model provider-specific errors in the type system
3. **Documentation**: Document authentication requirements and API quirks
4. **Testing**: Comprehensive tests with mocked responses
5. **Monitoring**: Track provider-specific metrics (latency, errors, costs)

## Future Enhancements

- Provider fallback/failover mechanisms
- Request retry with exponential backoff
- Response caching for identical requests
- Provider-specific rate limiting
- Dynamic provider configuration
- Multi-region support with latency-based routing
