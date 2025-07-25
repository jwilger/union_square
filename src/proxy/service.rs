//! Main proxy service implementation

use crate::proxy::hot_path::StreamingHotPathService;
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
}

impl ProxyService {
    /// Create a new proxy service
    pub fn new(config: ProxyConfig) -> Self {
        let ring_buffer = Arc::new(RingBuffer::new(&config.ring_buffer));
        let hot_path = StreamingHotPathService::new(config.clone(), ring_buffer.clone());

        Self {
            hot_path,
            ring_buffer,
            audit_shutdown_tx: None,
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
    pub fn into_router(
        mut self,
        auth_config: crate::proxy::middleware::AuthConfig,
    ) -> axum::Router {
        // Start the audit processor before creating the router
        self.start_audit_processor();

        // Create base router
        let router = axum::Router::new()
            .route("/health", axum::routing::get(health_handler))
            .route("/metrics", axum::routing::get(metrics_handler))
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

    // Extract target URL using centralized resolver
    let target_url = UrlResolver::extract_target_url(&request)?;

    // Forward the request using streaming hot path
    proxy
        .hot_path
        .forward_request(request, target_url, request_id)
        .await
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
