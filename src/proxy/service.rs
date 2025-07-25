//! Main proxy service implementation

use crate::proxy::streaming_simple::StreamingHotPathService;
use crate::proxy::{
    audit_path::AuditPathProcessor, middleware as proxy_middleware, ring_buffer::RingBuffer,
    types::*,
};
use axum::{
    body::Body,
    extract::{Request, State},
    http::StatusCode,
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
    pub fn into_router(mut self, auth_config: proxy_middleware::AuthConfig) -> axum::Router {
        // Start the audit processor before creating the router
        self.start_audit_processor();

        // Create the router with middleware stack
        axum::Router::new()
            .fallback(proxy_handler)
            .layer(axum::middleware::from_fn(
                proxy_middleware::request_id_middleware,
            ))
            .layer(axum::middleware::from_fn(
                proxy_middleware::logging_middleware,
            ))
            .layer(axum::middleware::from_fn(
                proxy_middleware::error_handling_middleware,
            ))
            .layer(axum::middleware::from_fn_with_state(
                Arc::new(auth_config),
                proxy_middleware::auth_middleware,
            ))
            .with_state(Arc::new(self))
    }
}

/// Axum handler for proxying requests
async fn proxy_handler(
    State(proxy): State<Arc<ProxyService>>,
    request: Request<Body>,
) -> Result<Response, ProxyError> {
    // Generate request ID for correlation
    let request_id = RequestId::new();

    // TODO: Extract target URL from request headers or path
    let target_url = TargetUrl::try_new("https://api.example.com")
        .map_err(|e| ProxyError::InvalidTargetUrl(e.to_string()))?;

    // Forward the request using streaming hot path
    proxy
        .hot_path
        .forward_request(request, target_url, request_id)
        .await
}

/// Error conversion for Axum responses
impl IntoResponse for ProxyError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            ProxyError::RequestTooLarge { .. } => (StatusCode::PAYLOAD_TOO_LARGE, self.to_string()),
            ProxyError::ResponseTooLarge { .. } => {
                (StatusCode::INSUFFICIENT_STORAGE, self.to_string())
            }
            ProxyError::RequestTimeout(_) => (StatusCode::REQUEST_TIMEOUT, self.to_string()),
            ProxyError::InvalidTargetUrl(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            ProxyError::HttpError(_) | ProxyError::HyperError(_) => {
                (StatusCode::BAD_GATEWAY, self.to_string())
            }
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_string(),
            ),
        };

        (status, message).into_response()
    }
}
