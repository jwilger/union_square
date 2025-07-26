# 0020. Authentication Pass-Through Design

Date: 2024-01-26
Status: Accepted

## Context

Union Square acts as a proxy between applications and LLM providers. Each provider has different authentication mechanisms:
- AWS Bedrock uses SigV4 signatures
- OpenAI uses API keys in headers
- Anthropic uses API keys in headers
- Other providers may use OAuth, JWT, or other schemes

We need to handle authentication in a way that:
1. Maintains security without storing credentials
2. Supports all provider authentication methods
3. Allows the proxy to remain stateless
4. Enables audit logging without exposing sensitive data

## Decision Drivers

- **Security**: Must never store or log credentials
- **Transparency**: Proxy should not interfere with provider authentication
- **Statelessness**: No session management or credential caching
- **Flexibility**: Support diverse authentication schemes
- **Auditability**: Track who makes requests without storing how

## Considered Options

### Option 1: Store and Manage Credentials
Proxy stores credentials and makes requests on behalf of clients.

**Pros:**
- Centralized credential management
- Could implement credential rotation
- Clients don't need provider credentials

**Cons:**
- Major security risk
- Compliance nightmare
- Complex key management required
- Violates principle of least privilege

### Option 2: Re-sign Requests
Proxy validates incoming auth, then re-signs with its own credentials.

**Pros:**
- Can validate authentication
- Single set of provider credentials

**Cons:**
- Requires proxy to have provider credentials
- Breaks direct accountability
- Complex signature manipulation
- Different logic for each provider

### Option 3: Complete Pass-Through
Forward authentication headers exactly as received.

**Pros:**
- No credential storage
- Provider handles all validation
- Maintains direct accountability
- Simple and secure

**Cons:**
- Cannot validate authentication locally
- Relies on provider error messages

## Decision Outcome

We chose **Option 3: Complete Pass-Through** for all authentication.

### Implementation Pattern

```rust
// Extract auth headers without validation
fn extract_sigv4_headers(headers: &HeaderMap) -> Result<Vec<(HeaderName, HeaderValue)>, ProviderError> {
    let mut auth_headers = Vec::new();

    for header_name in &[AUTHORIZATION, AMZ_DATE, AMZ_SECURITY_TOKEN, AMZ_CONTENT_SHA256] {
        if let Some(value) = headers.get(header_name) {
            auth_headers.push((header_name.clone(), value.clone()));
        }
    }

    Ok(auth_headers)
}

// Forward exactly as received
fn forward_request(&self, request: Request<Body>) -> Result<Response<Body>, ProviderError> {
    // Extract headers
    let auth_headers = extract_auth_headers(&request.headers())?;

    // Forward unchanged
    for (name, value) in auth_headers {
        forwarded_request.headers_mut().insert(name, value);
    }

    // Let provider validate
    client.request(forwarded_request).await
}
```

### Security Considerations

1. **No Logging**: Authentication headers are never logged
2. **No Storage**: Credentials pass through memory only
3. **No Validation**: We check presence, not correctness
4. **Provider Errors**: Authentication failures return provider's error response

## Consequences

### Positive
- Zero credential storage risk
- Simple, stateless implementation
- Supports all authentication schemes
- Maintains accountability chain
- Minimal attack surface
- Easy to audit (what, not how)

### Negative
- Cannot provide early authentication validation
- Dependent on provider error messages
- No ability to implement credential rotation
- Cannot aggregate requests under proxy credentials

### Monitoring and Audit

We track:
- ✅ Which provider was called
- ✅ When the request was made
- ✅ Whether authentication succeeded (via response)
- ❌ What credentials were used
- ❌ Who made the request (unless in other headers)

### Future Considerations

If we need request attribution, we could:
- Add optional Union Square API keys for client identification
- Use mutual TLS for client authentication
- Add request signing for non-repudiation

But these would be IN ADDITION to, not INSTEAD OF, the pass-through authentication.

### Proxy Authentication

Union Square's authentication model (as per ADR-0006) uses API keys for client authentication. This is separate from provider authentication:

1. **Proxy Authentication** (Union Square API key) - Identifies the client application to the proxy
2. **Provider Authentication** (pass-through) - Authenticates the request to the LLM provider

HTTP proxy standards (RFC 7235) define Proxy-Authorization headers for proxy authentication, but:
- Traditional HTTP proxies are typically transparent to the application
- Union Square is an application-level proxy with value-added features
- We need to track usage per client for audit purposes

Our approach:
- Use `X-API-Key` header for Union Square authentication (already implemented)
- Keep provider authentication separate and pass it through unchanged
- This allows clients to use different provider credentials while sharing a proxy

## Links

- ADR-0006 - Authentication and Authorization (establishes API key pattern)
- AWS SigV4 documentation
- PR #136 - Bedrock provider implementation
