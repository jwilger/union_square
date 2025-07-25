# Authentication and Authorization

- Status: accepted
- Deciders: John Wilger, Technical Architecture Team
- Date: 2025-07-14

## Context and Problem Statement

Union Square needs to support flexible authentication that integrates with customers' existing identity providers while maintaining security for multi-tenant data access. The system must support OIDC for user authentication, predefined roles for authorization, pass-through API keys for LLM providers, and complete data isolation between applications/customers.

## Decision Drivers

- **Flexibility**: Support any OIDC-compliant identity provider
- **Security**: Complete data isolation between tenants
- **Simplicity**: Start with predefined roles, allow future RBAC
- **No Vendor Lock-in**: Avoid proprietary auth systems
- **Self-Hosted Friendly**: Must work in air-gapped environments
- **API Key Safety**: Never store provider API keys

## Considered Options

- **Option 1**: Build custom auth system
- **Option 2**: Embed specific auth provider (Auth0, Okta)
- **Option 3**: OIDC client with JWT validation
- **Option 4**: mTLS for service-to-service auth

## Decision Outcome

Chosen option: **"OIDC client with JWT validation"** for user authentication, combined with **pass-through API keys** for LLM providers and **predefined roles** for initial authorization. This provides maximum flexibility while keeping implementation straightforward.

### Authentication Architecture

```rust
// OIDC Configuration per application
#[derive(Debug, Clone)]
pub struct OidcConfig {
    pub issuer_url: Url,
    pub client_id: ClientId,
    pub client_secret: ClientSecret,
    pub redirect_uri: Url,
}

// JWT Claims with custom fields
#[derive(Debug, Deserialize)]
pub struct CustomClaims {
    pub sub: String,
    pub email: Option<Email>,
    pub name: Option<String>,
    pub roles: Vec<Role>,
    pub application_id: ApplicationId,
}

// Predefined roles
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    Admin,      // Full system access
    Developer,  // View sessions, create/run tests, API access
    CSM,        // View sessions, flag issues, add notes
    Viewer,     // Read-only access
}

// Authorization checks
impl User {
    pub fn can_view_session(&self, session: &Session) -> bool {
        self.application_id == session.application_id
    }

    pub fn can_create_test(&self) -> bool {
        matches!(self.role, Role::Admin | Role::Developer)
    }

    pub fn can_flag_session(&self) -> bool {
        matches!(self.role, Role::Admin | Role::Developer | Role::CSM)
    }
}
```

### API Key Pass-Through

```rust
// Extract API key from request, never store
pub struct ProxyRequest {
    pub headers: HeaderMap,
    pub body: Bytes,
}

impl ProxyRequest {
    pub fn extract_api_key(&self) -> Option<&str> {
        self.headers
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.strip_prefix("Bearer "))
    }
}

// Forward to provider with original auth
async fn forward_to_provider(
    request: ProxyRequest,
    provider: &dyn ProviderApi,
) -> Result<Response> {
    let api_key = request.extract_api_key()
        .ok_or(AuthError::MissingApiKey)?;

    // Use key for this request only, never persist
    provider.forward_request(request, api_key).await
}
```

### Multi-Tenant Data Isolation

```rust
// Row-level security in PostgreSQL
-- Enable RLS
ALTER TABLE sessions ENABLE ROW LEVEL SECURITY;
ALTER TABLE events ENABLE ROW LEVEL SECURITY;

-- Policy for session access
CREATE POLICY session_isolation ON sessions
    FOR ALL
    USING (application_id = current_setting('app.application_id')::uuid);

-- Policy for event access
CREATE POLICY event_isolation ON events
    FOR ALL
    USING (
        session_id IN (
            SELECT id FROM sessions
            WHERE application_id = current_setting('app.application_id')::uuid
        )
    );

// Set application context for each request
impl DatabaseConnection {
    pub async fn with_application_context<T>(
        &self,
        application_id: ApplicationId,
        f: impl Future<Output = Result<T>>,
    ) -> Result<T> {
        self.execute(&format!(
            "SET LOCAL app.application_id = '{}'",
            application_id
        )).await?;
        f.await
    }
}
```

### Session Management

```rust
// Secure session tokens
#[nutype(
    validate(regex = "^[A-Za-z0-9+/]{43}=$"),
    derive(Debug, Clone, PartialEq, Eq, Hash)
)]
pub struct SessionToken(String);

impl SessionToken {
    pub fn generate() -> Self {
        let mut bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut bytes);
        Self::new(base64::encode(bytes)).unwrap()
    }
}

// Session storage (Redis or in-memory)
#[async_trait]
pub trait SessionStore: Send + Sync {
    async fn create(&self, user: User) -> Result<SessionToken>;
    async fn get(&self, token: &SessionToken) -> Result<Option<User>>;
    async fn delete(&self, token: &SessionToken) -> Result<()>;
}
```

### Positive Consequences

- **Provider Agnostic**: Works with any OIDC provider
- **Secure by Default**: No API keys stored, tenant isolation
- **Simple to Start**: Predefined roles are easy to implement
- **Future Proof**: Can add RBAC without breaking changes
- **Self-Hosted Ready**: No external dependencies required

### Negative Consequences

- **OIDC Complexity**: Requires understanding of OAuth2/OIDC
- **No Fine-Grained Permissions**: Initial role system is coarse
- **Session Management**: Need to implement token lifecycle

## Pros and Cons of the Options

### Option 1: Build custom auth system

Implement username/password auth from scratch.

- Good, because full control over implementation
- Bad, because significant security risk
- Bad, because reinventing well-solved problems
- Bad, because no SSO support
- Bad, because high maintenance burden

### Option 2: Embed specific auth provider

Tightly integrate with Auth0, Okta, or similar.

- Good, because quick to implement
- Good, because feature-rich out of the box
- Bad, because vendor lock-in
- Bad, because doesn't work in air-gapped environments
- Bad, because forces auth choice on customers

### Option 3: OIDC client with JWT validation

Standard OIDC implementation with JWT tokens.

- Good, because industry standard
- Good, because works with any OIDC provider
- Good, because customers use existing identity
- Good, because well-understood security model
- Bad, because requires OIDC setup
- Bad, because more complex than basic auth

### Option 4: mTLS for service-to-service

Mutual TLS for API authentication.

- Good, because very secure
- Good, because no passwords/tokens
- Bad, because complex certificate management
- Bad, because poor developer experience
- Bad, because hard to use from browsers

## Implementation Notes

### OIDC Libraries

- Use `openidconnect` crate for OIDC flows
- Use `jsonwebtoken` for JWT validation
- Cache JWKS for performance

### Security Headers

```rust
// Security middleware
pub fn security_headers() -> impl Layer<...> {
    SetResponseHeaderLayer::overriding(
        vec![
            (header::X_FRAME_OPTIONS, "DENY"),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
            (header::X_XSS_PROTECTION, "1; mode=block"),
            (HeaderName::from_static("referrer-policy"), "strict-origin"),
        ]
    )
}
```

## Links

- Related to [ADR-0001](0001-overall-architecture-pattern.md) - Auth is part of imperative shell
- Influences future ADR on API design
- Related to compliance requirements in PRD
