//! Main proxy service implementation
//!
//! The `ProxyService` is the main entry point for the Union Square proxy.
//! It orchestrates the hot path, audit path, and middleware stack to provide
//! a complete LLM API proxy solution.
//!
//! ## Service Lifecycle
//!
//! ```rust,ignore
//! use union_square::proxy::{ProxyService, ProxyConfig, AuthConfig};
//!
//! // 1. Create service with configuration
//! let config = ProxyConfig::default();
//! let service = ProxyService::new(config);
//!
//! // 2. Convert to Axum router (starts audit processor)
//! let auth_config = AuthConfig::default();
//! let router = service.into_router(auth_config);
//!
//! // 3. Serve with Axum
//! let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
//! axum::serve(listener, router).await?;
//! ```
//!
//! ## Components
//!
//! - **Hot Path**: Handles request/response streaming with minimal latency
//! - **Ring Buffer**: Lock-free buffer for passing events to audit path
//! - **Audit Processor**: Background task consuming events from ring buffer
//! - **Middleware Stack**: Tower middleware for auth, logging, etc.

use crate::providers::ProviderRegistry;
use crate::proxy::hot_path::StreamingHotPathService;
use crate::proxy::provider_router::ProviderRouter;
use crate::proxy::{
    audit_path::AuditPathProcessor, middleware_stack::ProxyMiddlewareStack,
    ring_buffer::RingBuffer, types::*, url_resolver::UrlResolver,
};
use axum::{
    body::Body,
    extract::{Request, State},
    response::{IntoResponse, Response},
};
use std::sync::Arc;
use tokio::sync::mpsc;

/// Main proxy service combining hot and audit paths
pub struct ProxyService {
    hot_path: StreamingHotPathService,
    ring_buffer: Arc<RingBuffer>,
    audit_shutdown_tx: Option<mpsc::Sender<()>>,
    provider_router: Arc<ProviderRouter>,
}

impl ProxyService {
    /// Create a new proxy service
    pub fn new(config: ProxyConfig) -> Self {
        let ring_buffer = Arc::new(RingBuffer::new(&config.ring_buffer));
        let hot_path = StreamingHotPathService::new(config.clone(), ring_buffer.clone());

        // Initialize provider registry with configured providers
        let mut registry = ProviderRegistry::new();

        // Register Bedrock provider (MVP provider)
        use crate::providers::bedrock::{provider::BedrockProvider, types::AwsRegion};

        // Check for endpoint override (for testing)
        if let Ok(endpoint_override) = std::env::var("BEDROCK_ENDPOINT_OVERRIDE") {
            let bedrock_provider = Arc::new(BedrockProvider::with_base_url(endpoint_override));
            registry.register(bedrock_provider);
        } else {
            let bedrock_region = config
                .bedrock_region
                .clone()
                .unwrap_or_else(|| "us-east-1".to_string());
            if let Ok(region) = AwsRegion::try_new(bedrock_region) {
                let bedrock_provider = Arc::new(BedrockProvider::new(region));
                registry.register(bedrock_provider);
            }
        }

        // Create provider router
        let provider_router = Arc::new(ProviderRouter::new(Arc::new(registry)));

        Self {
            hot_path,
            ring_buffer,
            audit_shutdown_tx: None,
            provider_router,
        }
    }

    /// Get a reference to the ring buffer for audit path processing
    pub fn ring_buffer(&self) -> Arc<RingBuffer> {
        Arc::clone(&self.ring_buffer)
    }

    /// Start the audit path processor
    pub fn start_audit_processor(&mut self) {
        let (processor, shutdown_tx) = AuditPathProcessor::new(Arc::clone(&self.ring_buffer));

        // Start the processor in a background task
        tokio::spawn(async move {
            processor.run().await;
        });

        self.audit_shutdown_tx = Some(shutdown_tx);
    }

    /// Create an Axum router for the proxy service with middleware
    pub fn into_router(mut self, auth_config: crate::proxy::AuthConfig) -> axum::Router {
        // Start the audit processor before creating the router
        self.start_audit_processor();

        // Create base router
        let router = axum::Router::new()
            .route(
                crate::proxy::headers::paths::HEALTH,
                axum::routing::get(health_handler),
            )
            .route(
                crate::proxy::headers::paths::METRICS,
                axum::routing::get(metrics_handler),
            )
            .fallback(proxy_handler)
            .with_state(Arc::new(self));

        // Apply middleware stack using the builder
        let middleware_stack = ProxyMiddlewareStack::new(auth_config);
        middleware_stack.apply_to_router(router)
    }
}

/// Axum handler for proxying requests
async fn proxy_handler(
    State(proxy): State<Arc<ProxyService>>,
    request: Request<Body>,
) -> Result<Response, ProxyError> {
    // Generate request ID for correlation
    let request_id = RequestId::new();

    // Check if this is a provider-routed request (URL-based routing)
    let path = request.uri().path();
    if path.starts_with("/bedrock/")
        || path.starts_with("/openai/")
        || path.starts_with("/anthropic/")
    {
        // Use provider-based routing
        proxy
            .provider_router
            .route_request(request, request_id)
            .await
    } else {
        // Fall back to header-based routing for backward compatibility
        let target_url = UrlResolver::extract_target_url(&request)?;

        // Forward the request using streaming hot path
        proxy
            .hot_path
            .forward_request(request, target_url, request_id)
            .await
    }
}

/// Error conversion for Axum responses using standardized format
impl IntoResponse for ProxyError {
    fn into_response(self) -> Response {
        use crate::proxy::error_response::ErrorResponseExt;

        let status = self.status_code();
        let error_response = self.to_error_response();
        error_response.into_response_with_status(status)
    }
}

/// Health check handler
async fn health_handler() -> &'static str {
    "OK"
}

/// Metrics handler - placeholder for now
async fn metrics_handler() -> &'static str {
    "metrics: placeholder"
}
