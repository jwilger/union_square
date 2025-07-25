//! Middleware implementations for the proxy service

use crate::proxy::types::*;
use axum::{
    extract::{Request, State},
    http::{header, HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;
use tracing::{error, info, warn};
use uuid::Uuid;

/// Configuration for authentication middleware
#[derive(Clone, Debug)]
pub struct AuthConfig {
    /// Valid API keys
    pub api_keys: HashSet<ApiKey>,
    /// Paths that bypass authentication
    pub bypass_paths: HashSet<BypassPath>,
}

impl Default for AuthConfig {
    fn default() -> Self {
        let mut bypass_paths = HashSet::new();
        // These are hardcoded constants that we know are valid paths
        // If they fail validation, it's a programming error that should be caught during development
        bypass_paths.insert(
            BypassPath::try_new(HEALTH_PATH.to_string())
                .expect("HEALTH_PATH constant should be a valid path"),
        );
        bypass_paths.insert(
            BypassPath::try_new(METRICS_PATH.to_string())
                .expect("METRICS_PATH constant should be a valid path"),
        );

        Self {
            api_keys: HashSet::new(),
            bypass_paths,
        }
    }
}

/// Request ID middleware - ensures every request has a unique ID for tracing
pub async fn request_id_middleware(
    mut request: Request,
    next: Next,
) -> Result<Response, ProxyError> {
    // Check if request already has an ID
    let request_id = if let Some(existing_id) = request.headers().get(REQUEST_ID_HEADER) {
        // Validate and use existing ID
        existing_id
            .to_str()
            .ok()
            .and_then(|s| Uuid::parse_str(s).ok())
            .and_then(|uuid| {
                // UUID strings are always valid header values, but handle gracefully
                HeaderValue::from_str(&uuid.to_string()).ok()
            })
            .unwrap_or_else(|| {
                // Generate new ID if invalid
                let new_id = Uuid::now_v7();
                // UUID v7 strings are always valid ASCII, but handle the theoretical error case
                HeaderValue::from_str(&new_id.to_string())
                    .expect("UUID v7 should always produce valid header value")
            })
    } else {
        // Generate new request ID
        let new_id = Uuid::now_v7();
        // UUID v7 strings are always valid ASCII, but handle the theoretical error case
        HeaderValue::from_str(&new_id.to_string())
            .expect("UUID v7 should always produce valid header value")
    };

    // Clone for response header
    let request_id_clone = request_id.clone();

    // Add to request headers
    request.headers_mut().insert(REQUEST_ID_HEADER, request_id);

    // Process request
    let mut response = next.run(request).await;

    // Add request ID to response
    response
        .headers_mut()
        .insert(REQUEST_ID_HEADER, request_id_clone);

    Ok(response)
}

/// Authentication middleware - validates API keys
pub async fn auth_middleware(
    State(auth_config): State<Arc<AuthConfig>>,
    request: Request,
    next: Next,
) -> Result<Response, ProxyError> {
    // Check if path should bypass auth
    let path = request.uri().path();
    if let Ok(bypass_path) = BypassPath::try_new(path.to_string()) {
        if auth_config.bypass_paths.contains(&bypass_path) {
            return Ok(next.run(request).await);
        }
    }

    // Extract bearer token from Authorization header
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    let api_key_str = match auth_header {
        Some(auth) if auth.starts_with(BEARER_PREFIX) => {
            auth.trim_start_matches(BEARER_PREFIX).trim()
        }
        _ => {
            warn!("Missing or invalid Authorization header");
            return Ok((
                StatusCode::UNAUTHORIZED,
                "Missing or invalid Authorization header",
            )
                .into_response());
        }
    };

    // Validate API key
    if let Ok(api_key) = ApiKey::try_new(api_key_str.to_string()) {
        if auth_config.api_keys.contains(&api_key) {
            // Process authenticated request
            return Ok(next.run(request).await);
        }
    }

    warn!("Invalid API key attempted: {}", api_key_str);
    Ok((StatusCode::UNAUTHORIZED, "Invalid API key").into_response())
}

/// Logging middleware - logs request/response details with timing
pub async fn logging_middleware(request: Request, next: Next) -> Result<Response, ProxyError> {
    let start = Instant::now();

    // Extract request details before passing ownership
    let method = request.method().clone();
    let uri = request.uri().clone();
    let request_id = request
        .headers()
        .get(REQUEST_ID_HEADER)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    info!(
        request_id = request_id,
        method = %method,
        path = %uri.path(),
        "Incoming request"
    );

    // Process request
    let response = next.run(request).await;
    let duration = start.elapsed();

    // Log response
    info!(
        request_id = request_id,
        method = %method,
        path = %uri.path(),
        status = response.status().as_u16(),
        duration_ms = duration.as_millis(),
        "Request completed"
    );

    Ok(response)
}

/// Error handling wrapper that converts ProxyError to HTTP responses
pub async fn error_handling_middleware(request: Request, next: Next) -> Response {
    let request_id = request
        .headers()
        .get(REQUEST_ID_HEADER)
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "unknown".to_string());

    match next.run(request).await.into_response() {
        response if response.status().is_success() => response,
        error_response => {
            // Log error with request ID
            error!(
                request_id = request_id,
                status = error_response.status().as_u16(),
                "Request failed"
            );

            // Ensure request ID is in error response
            let mut response = error_response;
            if let Ok(header_value) = HeaderValue::from_str(&request_id) {
                response
                    .headers_mut()
                    .insert(REQUEST_ID_HEADER, header_value);
            }
            response
        }
    }
}

// We'll implement the middleware stack composition directly in the router
// rather than trying to return a complex Layer type

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::middleware::{from_fn, from_fn_with_state};
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_request_id_generation() {
        // Create a simple handler that echoes the request ID
        let handler = tower::service_fn(|req: Request| async move {
            let request_id = req
                .headers()
                .get(REQUEST_ID_HEADER)
                .and_then(|h| h.to_str().ok())
                .unwrap_or("missing");

            Ok::<_, std::convert::Infallible>(
                Response::builder()
                    .status(StatusCode::OK)
                    .header(REQUEST_ID_HEADER, request_id)
                    .body(Body::empty())
                    .unwrap(),
            )
        });

        // Apply request ID middleware
        let service = tower::ServiceBuilder::new()
            .layer(from_fn(request_id_middleware))
            .service(handler);

        // Test without existing request ID
        let request = Request::builder()
            .method("GET")
            .uri("/test")
            .body(Body::empty())
            .unwrap();

        let response = service.clone().oneshot(request).await.unwrap();
        assert!(response.headers().contains_key(REQUEST_ID_HEADER));

        let request_id = response.headers().get(REQUEST_ID_HEADER).unwrap();
        let uuid = Uuid::parse_str(request_id.to_str().unwrap()).unwrap();
        assert_eq!(uuid.get_version_num(), 7);
    }

    #[tokio::test]
    async fn test_auth_middleware_valid_key() {
        let mut auth_config = AuthConfig::default();
        auth_config
            .api_keys
            .insert(ApiKey::try_new("valid-key-123".to_string()).unwrap());

        let handler = tower::service_fn(|_req: Request| async move {
            Ok::<_, std::convert::Infallible>(
                Response::builder()
                    .status(StatusCode::OK)
                    .body(Body::empty())
                    .unwrap(),
            )
        });

        let service = tower::ServiceBuilder::new()
            .layer(from_fn_with_state(Arc::new(auth_config), auth_middleware))
            .service(handler);

        let request = Request::builder()
            .method("POST")
            .uri("/api/v1/completion")
            .header(header::AUTHORIZATION, "Bearer valid-key-123")
            .body(Body::empty())
            .unwrap();

        let response = service.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_auth_middleware_invalid_key() {
        let auth_config = AuthConfig::default();

        let handler = tower::service_fn(|_req: Request| async move {
            Ok::<_, std::convert::Infallible>(
                Response::builder()
                    .status(StatusCode::OK)
                    .body(Body::empty())
                    .unwrap(),
            )
        });

        let service = tower::ServiceBuilder::new()
            .layer(from_fn_with_state(Arc::new(auth_config), auth_middleware))
            .service(handler);

        let request = Request::builder()
            .method("POST")
            .uri("/api/v1/completion")
            .header(header::AUTHORIZATION, "Bearer invalid-key")
            .body(Body::empty())
            .unwrap();

        let response = service.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_auth_bypass_health_check() {
        let auth_config = AuthConfig::default();

        let handler = tower::service_fn(|_req: Request| async move {
            Ok::<_, std::convert::Infallible>(
                Response::builder()
                    .status(StatusCode::OK)
                    .body(Body::empty())
                    .unwrap(),
            )
        });

        let service = tower::ServiceBuilder::new()
            .layer(from_fn_with_state(Arc::new(auth_config), auth_middleware))
            .service(handler);

        let request = Request::builder()
            .method("GET")
            .uri(HEALTH_PATH)
            .body(Body::empty())
            .unwrap();

        let response = service.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
