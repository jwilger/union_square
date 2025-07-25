//! Middleware stack builder for clean composition
//!
//! This module provides a builder pattern for composing the Tower middleware stack,
//! making it easier to maintain and test the middleware pipeline.

use crate::proxy::middleware::*;
use axum::{
    middleware::{from_fn, from_fn_with_state},
    Router,
};
use std::sync::Arc;

/// Builder for composing the proxy middleware stack
pub struct ProxyMiddlewareStack {
    auth_config: Arc<AuthConfig>,
}

impl ProxyMiddlewareStack {
    /// Create a new middleware stack builder
    pub fn new(auth_config: AuthConfig) -> Self {
        Self {
            auth_config: Arc::new(auth_config),
        }
    }

    /// Apply the complete middleware stack to a router
    ///
    /// The middleware are applied in the following order (outer to inner):
    /// 1. Request ID generation/propagation
    /// 2. Logging (with request ID)
    /// 3. Error handling
    /// 4. Authentication
    ///
    /// This ordering ensures:
    /// - Every request has an ID for correlation
    /// - All requests are logged (including auth failures)
    /// - Errors are properly formatted with request IDs
    /// - Authentication happens after basic request processing
    pub fn apply_to_router<S>(self, router: Router<S>) -> Router<S>
    where
        S: Clone + Send + Sync + 'static,
    {
        router
            // Apply middleware in reverse order (innermost first in the builder)
            .layer(from_fn_with_state(
                self.auth_config.clone(),
                auth_middleware,
            ))
            .layer(from_fn(error_handling_middleware))
            .layer(from_fn(logging_middleware))
            .layer(from_fn(request_id_middleware))
    }

    /// Create a minimal stack for testing (no auth)
    #[cfg(test)]
    pub fn minimal() -> Self {
        Self::new(AuthConfig::default())
    }

    /// Create a stack with custom auth configuration
    pub fn with_auth(mut self, auth_config: AuthConfig) -> Self {
        self.auth_config = Arc::new(auth_config);
        self
    }
}

/// Configuration for the entire middleware stack
#[derive(Clone, Debug)]
pub struct ProxyMiddlewareConfig {
    /// Authentication configuration
    pub auth: AuthConfig,
    /// Whether to enable request logging
    pub enable_logging: bool,
    /// Whether to enable detailed error responses (for debugging)
    pub detailed_errors: bool,
}

impl Default for ProxyMiddlewareConfig {
    fn default() -> Self {
        Self {
            auth: AuthConfig::default(),
            enable_logging: true,
            detailed_errors: false,
        }
    }
}

impl ProxyMiddlewareConfig {
    /// Create middleware stack from configuration
    pub fn build_stack(self) -> ProxyMiddlewareStack {
        ProxyMiddlewareStack::new(self.auth)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proxy::types::{ApiKey, HEALTH_PATH, REQUEST_ID_HEADER};
    use axum::{body::Body, http::StatusCode, response::IntoResponse};
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_middleware_stack_builder() {
        // Create a simple handler
        async fn handler() -> impl IntoResponse {
            StatusCode::OK
        }

        // Build router with middleware stack
        let router = Router::new()
            .route("/test", axum::routing::get(handler))
            .with_state(());

        // Create auth config with a test API key
        let mut auth_config = AuthConfig::default();
        auth_config
            .api_keys
            .insert(ApiKey::try_new("test-key".to_string()).unwrap());

        let stack = ProxyMiddlewareStack::new(auth_config);
        let app = stack.apply_to_router(router);

        // Test with valid API key
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/test")
                    .header("Authorization", "Bearer test-key")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert!(response.headers().contains_key(REQUEST_ID_HEADER));
    }

    #[tokio::test]
    async fn test_middleware_stack_health_bypass() {
        // Create a simple handler
        async fn handler() -> impl IntoResponse {
            StatusCode::OK
        }

        // Build router with middleware stack
        let router = Router::new()
            .route(HEALTH_PATH, axum::routing::get(handler))
            .with_state(());

        let stack = ProxyMiddlewareStack::minimal();
        let app = stack.apply_to_router(router);

        // Test health endpoint bypasses auth
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri(HEALTH_PATH)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_middleware_config_builder() {
        let config = ProxyMiddlewareConfig {
            auth: AuthConfig::default(),
            enable_logging: true,
            detailed_errors: false,
        };

        let stack = config.build_stack();
        assert!(Arc::strong_count(&stack.auth_config) == 1);
    }
}
