//! Test helpers specific to middleware testing
//!
//! This module provides utilities for testing individual middleware
//! components in isolation.

#[cfg(test)]
pub mod helpers {
    use crate::proxy::{AuthConfig, ProxyMiddlewareStack};
    use axum::{
        body::Body,
        http::{Request, Response, StatusCode},
        Router,
    };
    use tower::ServiceExt;

    /// Test harness for middleware testing
    pub struct MiddlewareTestHarness {
        router: Router,
    }

    impl Default for MiddlewareTestHarness {
        fn default() -> Self {
            Self::new()
        }
    }

    impl MiddlewareTestHarness {
        /// Create a new test harness with a simple handler
        pub fn new() -> Self {
            let router = Router::new()
                .route("/test", axum::routing::any(test_handler))
                .route("/echo", axum::routing::any(echo_handler))
                .route("/error", axum::routing::any(error_handler));

            Self { router }
        }

        /// Apply a single middleware for testing
        /// Note: This is a placeholder - actual middleware application
        /// should use the ProxyMiddlewareStack for proper composition
        pub fn with_middleware(self) -> Self {
            // Middleware is applied via with_full_stack method
            self
        }

        /// Apply the full middleware stack
        pub fn with_full_stack(self, auth_config: AuthConfig) -> Self {
            let stack = ProxyMiddlewareStack::new(auth_config);
            Self {
                router: stack.apply_to_router(self.router),
            }
        }

        /// Send a request through the middleware stack
        pub async fn send_request(&mut self, request: Request<Body>) -> Response<Body> {
            self.router
                .clone()
                .oneshot(request)
                .await
                .expect("middleware should not fail")
        }

        /// Helper to create and send a GET request
        pub async fn get(&mut self, path: &str) -> Response<Body> {
            let request = Request::builder()
                .method("GET")
                .uri(path)
                .body(Body::empty())
                .expect("test request should be valid");
            self.send_request(request).await
        }

        /// Helper to create and send an authenticated request
        pub async fn get_with_auth(&mut self, path: &str, api_key: &str) -> Response<Body> {
            let request = Request::builder()
                .method("GET")
                .uri(path)
                .header("Authorization", format!("Bearer {api_key}"))
                .body(Body::empty())
                .expect("test request should be valid");
            self.send_request(request).await
        }
    }

    /// Simple test handler that returns OK
    async fn test_handler() -> StatusCode {
        StatusCode::OK
    }

    /// Echo handler that returns request details
    async fn echo_handler(req: Request<Body>) -> Response<Body> {
        let headers = req
            .headers()
            .iter()
            .map(|(k, v)| format!("{}: {}", k, v.to_str().unwrap_or("<binary>")))
            .collect::<Vec<_>>()
            .join("\n");

        Response::builder()
            .status(StatusCode::OK)
            .body(Body::from(headers))
            .expect("response should be valid")
    }

    /// Error handler that always returns an error
    async fn error_handler() -> Result<StatusCode, TestError> {
        Err(TestError::Simulated)
    }

    #[derive(Debug)]
    enum TestError {
        Simulated,
    }

    impl axum::response::IntoResponse for TestError {
        fn into_response(self) -> axum::response::Response {
            (StatusCode::INTERNAL_SERVER_ERROR, "Test error").into_response()
        }
    }

    /// Assertions for middleware testing
    pub mod assertions {
        use super::*;

        /// Assert that a response has the expected status code
        pub fn assert_status(response: &Response<Body>, expected: StatusCode) {
            assert_eq!(
                response.status(),
                expected,
                "Expected status {} but got {}",
                expected,
                response.status()
            );
        }

        /// Assert that a response contains a specific header
        pub fn assert_header_exists(response: &Response<Body>, header_name: &str) {
            assert!(
                response.headers().contains_key(header_name),
                "Expected header '{header_name}' not found"
            );
        }

        /// Assert that a response header has a specific value
        pub fn assert_header_value(
            response: &Response<Body>,
            header_name: &str,
            expected_value: &str,
        ) {
            let actual = response
                .headers()
                .get(header_name)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("<missing>");

            assert_eq!(
                actual, expected_value,
                "Header '{header_name}' expected '{expected_value}' but got '{actual}'"
            );
        }

        /// Assert that response body contains expected text
        pub async fn assert_body_contains(response: Response<Body>, expected: &str) {
            let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("body should be readable");
            let body_str = String::from_utf8_lossy(&body_bytes);

            assert!(
                body_str.contains(expected),
                "Body does not contain '{expected}'. Actual: '{body_str}'"
            );
        }
    }

    /// Middleware test scenarios
    pub mod scenarios {
        use super::*;
        use crate::proxy::headers::X_REQUEST_ID;

        /// Test that request ID middleware adds IDs to requests
        pub async fn test_request_id_generation(harness: &mut MiddlewareTestHarness) {
            let response = harness.get("/test").await;
            assertions::assert_header_exists(&response, X_REQUEST_ID);

            // Verify UUID format
            let request_id = response
                .headers()
                .get(X_REQUEST_ID)
                .and_then(|v| v.to_str().ok())
                .expect("request ID should exist");

            assert!(
                uuid::Uuid::parse_str(request_id).is_ok(),
                "Request ID should be a valid UUID"
            );
        }

        /// Test that existing request IDs are preserved
        pub async fn test_request_id_preservation(harness: &mut MiddlewareTestHarness) {
            let existing_id = uuid::Uuid::new_v4().to_string();
            let request = Request::builder()
                .uri("/test")
                .header(X_REQUEST_ID, &existing_id)
                .body(Body::empty())
                .expect("request should be valid");

            let response = harness.send_request(request).await;
            assertions::assert_header_value(&response, X_REQUEST_ID, &existing_id);
        }

        /// Test authentication with valid API key
        pub async fn test_valid_authentication(
            harness: &mut MiddlewareTestHarness,
            valid_key: &str,
        ) {
            let response = harness.get_with_auth("/test", valid_key).await;
            assertions::assert_status(&response, StatusCode::OK);
        }

        /// Test authentication with invalid API key
        pub async fn test_invalid_authentication(harness: &mut MiddlewareTestHarness) {
            let response = harness.get_with_auth("/test", "invalid-key").await;
            assertions::assert_status(&response, StatusCode::UNAUTHORIZED);
        }

        /// Test health check bypass
        pub async fn test_health_check_bypass(harness: &mut MiddlewareTestHarness) {
            let response = harness.get("/health").await;
            assertions::assert_status(&response, StatusCode::OK);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::helpers::*;
    use axum::http::StatusCode;

    #[tokio::test]
    async fn test_middleware_harness() {
        let mut harness = MiddlewareTestHarness::new();
        let response = harness.get("/test").await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_echo_handler() {
        let mut harness = MiddlewareTestHarness::new();
        let response = harness.get_with_auth("/echo", "test-key").await;

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body_str = String::from_utf8_lossy(&body);

        assert!(body_str.contains("authorization: Bearer test-key"));
    }
}
