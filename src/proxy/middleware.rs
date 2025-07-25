//! Middleware implementations for the proxy service

use crate::proxy::headers::{self, BEARER_PREFIX, X_REQUEST_ID};
use crate::proxy::http_types::HttpPath;
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
            BypassPath::try_new(headers::paths::HEALTH.to_string())
                .expect("headers::paths::HEALTH constant should be a valid path"),
        );
        bypass_paths.insert(
            BypassPath::try_new(headers::paths::METRICS.to_string())
                .expect("headers::paths::METRICS constant should be a valid path"),
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
    let request_id = if let Some(existing_id) = request.headers().get(X_REQUEST_ID) {
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
    request.headers_mut().insert(X_REQUEST_ID, request_id);

    // Process request
    let mut response = next.run(request).await;

    // Add request ID to response
    response
        .headers_mut()
        .insert(X_REQUEST_ID, request_id_clone);

    Ok(response)
}

/// Authentication middleware - validates API keys
pub async fn auth_middleware(
    State(auth_config): State<Arc<AuthConfig>>,
    request: Request,
    next: Next,
) -> Result<Response, ProxyError> {
    // Check if path should bypass auth
    let http_path = HttpPath::from_uri(request.uri());
    if let Ok(bypass_path) = BypassPath::try_new(http_path.to_string()) {
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
            use crate::proxy::error_response::{extract_request_id, ErrorResponse};

            warn!("Missing or invalid Authorization header");
            let request_id = extract_request_id(request.headers());
            let error =
                ErrorResponse::new("UNAUTHORIZED", "Missing or invalid Authorization header");
            let error = if let Some(id) = request_id {
                error.with_request_id(id)
            } else {
                error
            };
            return Ok(error.into_response_with_status(StatusCode::UNAUTHORIZED));
        }
    };

    // Validate API key
    if let Ok(api_key) = ApiKey::try_new(api_key_str.to_string()) {
        if auth_config.api_keys.contains(&api_key) {
            // Process authenticated request
            return Ok(next.run(request).await);
        }
    }

    use crate::proxy::error_response::{extract_request_id, ErrorResponse};

    warn!("Invalid API key attempted: {}", api_key_str);
    let request_id = extract_request_id(request.headers());
    let error = ErrorResponse::new("UNAUTHORIZED", "Invalid API key");
    let error = if let Some(id) = request_id {
        error.with_request_id(id)
    } else {
        error
    };
    Ok(error.into_response_with_status(StatusCode::UNAUTHORIZED))
}

/// Logging middleware - logs request/response details with timing
pub async fn logging_middleware(request: Request, next: Next) -> Result<Response, ProxyError> {
    let start = Instant::now();

    // Extract request details before passing ownership
    let method = crate::proxy::http_types::SafeHttpMethod::from_method(request.method().clone());
    let path = HttpPath::from_uri(request.uri());
    let request_id = request
        .headers()
        .get(X_REQUEST_ID)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    info!(
        request_id = request_id,
        method = %method,
        path = %path,
        "Incoming request"
    );

    // Process request
    let response = next.run(request).await;
    let duration = start.elapsed();

    // Log response
    info!(
        request_id = request_id,
        method = %method,
        path = %path,
        status = response.status().as_u16(),
        duration_ms = duration.as_millis(),
        "Request completed"
    );

    Ok(response)
}

/// Error handling wrapper that converts ProxyError to HTTP responses
pub async fn error_handling_middleware(request: Request, next: Next) -> Response {
    use crate::proxy::error_response::{extract_request_id, standard_error_response};

    let request_id = extract_request_id(request.headers());

    match next.run(request).await.into_response() {
        response if response.status().is_success() => response,
        error_response => {
            let status = error_response.status();

            // Log error with request ID
            error!(
                request_id = ?request_id,
                status = status.as_u16(),
                "Request failed"
            );

            // Return standardized error response
            standard_error_response(status, request_id.as_deref())
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
                .get(X_REQUEST_ID)
                .and_then(|h| h.to_str().ok())
                .unwrap_or("missing");

            Ok::<_, std::convert::Infallible>(
                Response::builder()
                    .status(StatusCode::OK)
                    .header(X_REQUEST_ID, request_id)
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
        assert!(response.headers().contains_key(X_REQUEST_ID));

        let request_id = response.headers().get(X_REQUEST_ID).unwrap();
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
            .uri(headers::paths::HEALTH)
            .body(Body::empty())
            .unwrap();

        let response = service.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
