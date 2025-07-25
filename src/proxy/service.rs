//! Main proxy service implementation

use crate::proxy::{hot_path::HotPathService, ring_buffer::RingBuffer, types::*};
use axum::{
    body::Body,
    extract::{Request, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use std::sync::Arc;
use uuid::Uuid;

/// Main proxy service combining hot and audit paths
#[allow(dead_code)]
pub struct ProxyService {
    config: Arc<ProxyConfig>,
    hot_path: HotPathService,
    ring_buffer: Arc<RingBuffer>,
}

impl ProxyService {
    /// Create a new proxy service
    pub fn new(config: ProxyConfig) -> Self {
        let ring_buffer = Arc::new(RingBuffer::new(&config.ring_buffer));
        let hot_path = HotPathService::new(config.clone());

        Self {
            config: Arc::new(config),
            hot_path,
            ring_buffer,
        }
    }

    /// Get a reference to the ring buffer for audit path processing
    pub fn ring_buffer(&self) -> Arc<RingBuffer> {
        Arc::clone(&self.ring_buffer)
    }

    /// Create an Axum router for the proxy service
    pub fn into_router(self) -> axum::Router {
        axum::Router::new()
            .fallback(proxy_handler)
            .with_state(Arc::new(self))
    }
}

/// Axum handler for proxying requests
async fn proxy_handler(
    State(proxy): State<Arc<ProxyService>>,
    request: Request<Body>,
) -> Result<Response, ProxyError> {
    // Generate request ID for correlation
    let request_id = unsafe { RequestId::new_unchecked(Uuid::now_v7()) };

    // TODO: Extract target URL from request headers or path
    let _target_url = TargetUrl::try_new("https://api.example.com")
        .map_err(|e| ProxyError::InvalidTargetUrl(e.to_string()))?;

    // Record request received event (fire-and-forget to ring buffer)
    let event = AuditEvent {
        request_id,
        session_id: unsafe { SessionId::new_unchecked(Uuid::now_v7()) }, // TODO: Extract from headers/context
        timestamp: chrono::Utc::now(),
        event_type: AuditEventType::RequestReceived {
            method: request.method().to_string(),
            uri: request.uri().to_string(),
            headers: request
                .headers()
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("<binary>").to_string()))
                .collect(),
            body_size: 0, // TODO: Calculate from content-length
        },
    };

    // Write to ring buffer (ignore overflow for hot path)
    let _ = proxy
        .ring_buffer
        .write(request_id, &serde_json::to_vec(&event).unwrap_or_default());

    // TODO: Implement proper body conversion
    // For now, just return a placeholder response
    Ok(Response::builder()
        .status(200)
        .body(Body::from("Proxy placeholder"))
        .unwrap())
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
