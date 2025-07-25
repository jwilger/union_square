//! Middleware stack builder for clean composition
//!
//! This module provides a builder pattern for composing the Tower middleware stack,
//! making it easier to maintain and test the middleware pipeline.

use crate::proxy::middleware::*;
use crate::proxy::types::BypassPath;
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
    /// Whether to enable request ID generation
    pub enable_request_id: bool,
    /// Whether to enable health check endpoint
    pub enable_health_check: bool,
    /// Whether to enable metrics endpoint
    pub enable_metrics: bool,
}

impl Default for ProxyMiddlewareConfig {
    fn default() -> Self {
        Self {
            auth: AuthConfig::default(),
            enable_logging: true,
            detailed_errors: false,
            enable_request_id: true,
            enable_health_check: true,
            enable_metrics: true,
        }
    }
}

impl ProxyMiddlewareConfig {
    /// Create middleware stack from configuration
    pub fn build_stack(self) -> ProxyMiddlewareStack {
        // Update auth config with health/metrics bypass paths based on configuration
        let mut auth_config = self.auth;

        if self.enable_health_check {
            auth_config.bypass_paths.insert(
                BypassPath::try_new(crate::proxy::headers::paths::HEALTH.to_string())
                    .expect("HEALTH path should be valid"),
            );
        }

        if self.enable_metrics {
            auth_config.bypass_paths.insert(
                BypassPath::try_new(crate::proxy::headers::paths::METRICS.to_string())
                    .expect("METRICS path should be valid"),
            );
        }

        // Create stack with configured auth
        // In the future, other flags can be used to conditionally apply middleware
        ProxyMiddlewareStack::new(auth_config)
    }

    /// Builder method to disable health check endpoint
    pub fn disable_health_check(mut self) -> Self {
        self.enable_health_check = false;
        self
    }

    /// Builder method to disable metrics endpoint
    pub fn disable_metrics(mut self) -> Self {
        self.enable_metrics = false;
        self
    }

    /// Builder method to disable request logging
    pub fn disable_logging(mut self) -> Self {
        self.enable_logging = false;
        self
    }

    /// Builder method to enable detailed error responses
    pub fn enable_detailed_errors(mut self) -> Self {
        self.detailed_errors = true;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proxy::headers::{paths, X_REQUEST_ID};
    use crate::proxy::types::ApiKey;
    use axum::{body::Body, http::StatusCode, response::IntoResponse};
    use std::collections::HashSet;
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
        assert!(response.headers().contains_key(X_REQUEST_ID));
    }

    #[tokio::test]
    async fn test_middleware_stack_health_bypass() {
        // Create a simple handler
        async fn handler() -> impl IntoResponse {
            StatusCode::OK
        }

        // Build router with middleware stack
        let router = Router::new()
            .route(paths::HEALTH, axum::routing::get(handler))
            .with_state(());

        let stack = ProxyMiddlewareStack::minimal();
        let app = stack.apply_to_router(router);

        // Test health endpoint bypasses auth
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri(paths::HEALTH)
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
            enable_request_id: true,
            enable_health_check: true,
            enable_metrics: true,
        };

        let stack = config.build_stack();
        assert!(Arc::strong_count(&stack.auth_config) == 1);
    }

    #[test]
    fn test_middleware_config_builder_methods() {
        let config = ProxyMiddlewareConfig::default()
            .disable_health_check()
            .disable_metrics()
            .enable_detailed_errors();

        assert!(!config.enable_health_check);
        assert!(!config.enable_metrics);
        assert!(config.detailed_errors);
        assert!(config.enable_logging); // Still true by default
    }

    #[test]
    fn test_middleware_config_bypass_paths() {
        // Test that bypass paths are added based on configuration
        let mut config = ProxyMiddlewareConfig::default();
        config
            .auth
            .api_keys
            .insert(ApiKey::try_new("test-key".to_string()).unwrap());

        let stack = config.build_stack();
        // The auth config should have health and metrics in bypass paths
        assert_eq!(stack.auth_config.bypass_paths.len(), 2);
    }

    #[test]
    fn test_middleware_config_no_bypass_paths() {
        // Test that bypass paths are not added when disabled
        let mut auth_config = AuthConfig {
            api_keys: HashSet::new(),
            bypass_paths: HashSet::new(), // Start with empty bypass paths
        };
        auth_config
            .api_keys
            .insert(ApiKey::try_new("test-key".to_string()).unwrap());

        let config = ProxyMiddlewareConfig {
            auth: auth_config,
            enable_logging: true,
            detailed_errors: false,
            enable_request_id: true,
            enable_health_check: false, // Disabled
            enable_metrics: false,      // Disabled
        };

        let stack = config.build_stack();
        // The auth config should have no bypass paths since we disabled them
        assert_eq!(stack.auth_config.bypass_paths.len(), 0);
    }
}
