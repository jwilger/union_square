# ADR-0011: Provider Abstraction and Routing

## Status

Accepted

## Context

Union Square must support multiple LLM providers (OpenAI, Anthropic, Bedrock, Vertex AI) while:

1. Maintaining provider-specific API compatibility
2. Allowing seamless failover between Union Square and direct provider access
3. Supporting different API styles (REST, streaming, different auth methods)
4. Enabling provider-specific features without abstraction leakage
5. Routing requests efficiently with <5ms overhead

Each provider has unique characteristics:
- **OpenAI**: REST API with Bearer token auth, streaming via SSE
- **Anthropic**: Similar to OpenAI but different request/response formats
- **Bedrock**: AWS Signature v4 auth, different endpoints per model
- **Vertex AI**: Google OAuth2, region-specific endpoints

## Decision

We will implement a provider abstraction that preserves API compatibility while enabling routing:

### URL-Based Routing

Requests are routed based on URL path prefix:
```
https://union-square.example.com/openai/v1/chat/completions → OpenAI
https://union-square.example.com/anthropic/v1/messages → Anthropic  
https://union-square.example.com/bedrock/model/invoke → Bedrock
https://union-square.example.com/vertex-ai/v1/projects/... → Vertex AI
```

This allows applications to switch between Union Square and direct provider access by only changing the base URL.

### Provider Trait Architecture

```rust
trait Provider: Send + Sync {
    /// Provider identifier (openai, anthropic, etc.)
    fn id(&self) -> &'static str;
    
    /// Check if this provider handles the given path
    fn matches_path(&self, path: &str) -> bool;
    
    /// Transform Union Square URL to provider URL
    fn transform_url(&self, url: &Url) -> Result<Url, ProviderError>;
    
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
    ) -> Metadata;
    
    /// Provider-specific health check
    async fn health_check(&self, client: &HttpClient) -> HealthStatus;
}
```

### Provider Implementations

Each provider implementation handles its specific requirements:

```rust
struct OpenAIProvider {
    base_url: Url,  // Default: https://api.openai.com
}

impl Provider for OpenAIProvider {
    fn matches_path(&self, path: &str) -> bool {
        path.starts_with("/openai/")
    }
    
    fn transform_url(&self, url: &Url) -> Result<Url, ProviderError> {
        // Remove /openai prefix and forward to api.openai.com
        let path = url.path().strip_prefix("/openai")
            .ok_or_else(|| ProviderError::InvalidPath("Missing /openai prefix".to_string()))?;
        self.base_url.join(path).map_err(Into::into)
    }
    
    // ... other methods
}
```

### Request Flow

1. **Incoming Request** → Router examines URL path
2. **Provider Selection** → First matching provider handles request
3. **URL Transformation** → Remove Union Square prefix, add provider domain
4. **Header Preservation** → Pass through all headers (auth, custom headers)
5. **Request Forwarding** → Provider-specific client configuration
6. **Response Handling** → Stream or buffer based on response type
7. **Metadata Extraction** → Provider extracts relevant audit data

### Provider Registry

```rust
struct ProviderRegistry {
    providers: Vec<Box<dyn Provider>>,
}

impl ProviderRegistry {
    fn route(&self, path: &str) -> Option<&dyn Provider> {
        self.providers.iter()
            .find(|p| p.matches_path(path))
            .map(|p| p.as_ref())
    }
}
```

### Configuration

```toml
[providers.openai]
enabled = true
base_url = "https://api.openai.com"  # Override for self-hosted

[providers.bedrock]
enabled = true
region = "us-east-1"
# Uses AWS SDK for endpoint discovery

[providers.anthropic]
enabled = true
base_url = "https://api.anthropic.com"

[providers.vertex_ai]
enabled = false  # Post-MVP
```

## Consequences

### Positive

- Zero code changes for client applications
- Easy to add new providers
- Provider-specific optimizations possible
- Clean separation of provider logic
- Supports testing with mock providers
- Enables gradual rollout (route percentage of traffic)

### Negative

- URL path coupling (must maintain compatibility)
- Different error handling per provider
- Complex testing matrix (providers × features)
- Authentication complexity (different schemes)
- Must track provider API changes

### Mitigation Strategies

1. **Provider Tests**: Integration tests against real providers
2. **Version Pinning**: Document supported API versions
3. **Error Mapping**: Standardize error reporting
4. **Mock Providers**: For testing and development
5. **Provider Modules**: Separate crate per provider for isolation

## Alternatives Considered

1. **Header-Based Routing**
   - Use custom header to indicate provider
   - Rejected: Requires client changes, not drop-in compatible

2. **Single Unified API**
   - Abstract all providers behind common API
   - Rejected: Loses provider-specific features, requires client changes

3. **Subdomain Routing**
   - openai.union-square.com, anthropic.union-square.com
   - Rejected: Complex SSL cert management, DNS configuration

4. **Query Parameter Routing**
   - Add ?provider=openai to requests
   - Rejected: Modifies request signature, auth complications

5. **Auto-Detection**
   - Detect provider from request format
   - Rejected: Ambiguous, error-prone, slow

## Implementation Notes

- Use tower middleware for provider abstraction
- Implement providers as separate modules
- Share common HTTP client with connection pooling
- Add provider metrics (requests, latency, errors)
- Support provider-specific retry policies

## Related Decisions

- ADR-0008: Dual-path Architecture (routing happens in hot path)
- ADR-0003: Proxy Implementation Strategy (HTTP client selection)
- ADR-0016: Performance Monitoring (provider-specific metrics)