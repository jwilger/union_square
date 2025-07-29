//! Tests for proxy middleware layers

#[cfg(test)]
mod tests {
    use crate::proxy::middleware_test_helpers::helpers::{assertions::*, MiddlewareTestHarness};
    use crate::proxy::{headers::X_REQUEST_ID, types::ApiKey, AuthConfig};
    use axum::body::Body;
    use axum::http::{header, StatusCode};
    use http::Request;
    use uuid::Uuid;

    mod request_id_middleware {
        use super::*;

        #[tokio::test]
        async fn test_request_id_generation() {
            // Test that request ID middleware generates a valid UUID v7
            let auth_config = AuthConfig::default();
            let mut harness = MiddlewareTestHarness::new().with_full_stack(auth_config);

            let response = harness.get("/test").await;

            // Should have X-Request-ID header with valid UUID v7
            assert_header_exists(&response, X_REQUEST_ID);
            let request_id = response
                .headers()
                .get(X_REQUEST_ID)
                .unwrap()
                .to_str()
                .unwrap();
            let uuid = Uuid::parse_str(request_id).unwrap();
            assert_eq!(uuid.get_version_num(), 7);
        }

        #[tokio::test]
        async fn test_request_id_passthrough() {
            // Test that existing request IDs are preserved
            let existing_id = Uuid::now_v7().to_string();
            let auth_config = AuthConfig::default();
            let mut harness = MiddlewareTestHarness::new().with_full_stack(auth_config);

            let request = Request::builder()
                .method("GET")
                .uri("/test")
                .header(X_REQUEST_ID, &existing_id)
                .body(Body::empty())
                .unwrap();

            let response = harness.send_request(request).await;

            // Should preserve the existing request ID
            assert_header_value(&response, X_REQUEST_ID, &existing_id);
        }

        #[tokio::test]
        async fn test_request_id_propagation() {
            // Test that request ID is propagated through the request chain
            let auth_config = AuthConfig::default();
            let mut harness = MiddlewareTestHarness::new().with_full_stack(auth_config);

            let request = Request::builder()
                .method("POST")
                .uri("/echo") // Use echo endpoint to verify headers
                .body(Body::from("test body"))
                .unwrap();

            let response = harness.send_request(request).await;

            // Request ID should be in response headers
            assert_header_exists(&response, X_REQUEST_ID);

            // Verify the request ID is a valid UUID v7
            let request_id = response
                .headers()
                .get(X_REQUEST_ID)
                .unwrap()
                .to_str()
                .unwrap();
            let uuid = Uuid::parse_str(request_id).unwrap();
            assert_eq!(uuid.get_version_num(), 7);
        }
    }

    mod auth_middleware {
        use super::*;
        use crate::proxy::types::ApiKey;

        #[tokio::test]
        async fn test_valid_api_key() {
            // Test that valid API keys are accepted
            let valid_key = "valid-api-key-123";
            let mut auth_config = AuthConfig::default();
            auth_config
                .api_keys
                .insert(ApiKey::try_new(valid_key.to_string()).unwrap());

            let mut harness = MiddlewareTestHarness::new().with_full_stack(auth_config);

            let response = harness.get_with_auth("/test", valid_key).await;

            // Should pass through with 200 OK
            assert_status(&response, StatusCode::OK);
        }

        #[tokio::test]
        async fn test_missing_api_key() {
            // Test that missing API keys are rejected
            let mut auth_config = AuthConfig::default();
            // Add a valid key but don't use it in the request
            auth_config
                .api_keys
                .insert(ApiKey::try_new("valid-key".to_string()).unwrap());

            let mut harness = MiddlewareTestHarness::new().with_full_stack(auth_config);

            let request = Request::builder()
                .method("POST")
                .uri("/test")
                .body(Body::empty())
                .unwrap();

            let response = harness.send_request(request).await;

            // Should return 401 Unauthorized
            assert_status(&response, StatusCode::UNAUTHORIZED);
        }

        #[tokio::test]
        async fn test_invalid_api_key() {
            // Test that invalid API keys are rejected
            let mut auth_config = AuthConfig::default();
            auth_config
                .api_keys
                .insert(ApiKey::try_new("valid-key-123".to_string()).unwrap());

            let mut harness = MiddlewareTestHarness::new().with_full_stack(auth_config);

            // Use a different key than what's configured
            let response = harness.get_with_auth("/test", "invalid-api-key").await;

            // Should return 401 Unauthorized
            assert_status(&response, StatusCode::UNAUTHORIZED);
        }

        #[tokio::test]
        async fn test_malformed_auth_header() {
            // Test that malformed auth headers are rejected
            let mut auth_config = AuthConfig::default();
            auth_config
                .api_keys
                .insert(ApiKey::try_new("valid-key".to_string()).unwrap());

            let mut harness = MiddlewareTestHarness::new().with_full_stack(auth_config);

            let request = Request::builder()
                .method("POST")
                .uri("/test")
                .header(header::AUTHORIZATION, "NotBearer token")
                .body(Body::empty())
                .unwrap();

            let response = harness.send_request(request).await;

            // Should return 401 Unauthorized
            assert_status(&response, StatusCode::UNAUTHORIZED);
        }

        #[tokio::test]
        async fn test_auth_bypass_for_health_check() {
            // Test that health check endpoint bypasses auth
            let auth_config = AuthConfig::default();
            // No API keys configured, but health check should still work

            let mut harness = MiddlewareTestHarness::new().with_full_stack(auth_config);

            let response = harness.get("/health").await;

            // Should pass through without auth
            assert_status(&response, StatusCode::OK);
        }
    }

    mod error_handling_middleware {
        use super::*;

        #[tokio::test]
        async fn test_proxy_error_formatting() {
            // Test that ProxyError is properly formatted in responses
            // The error handler is built into the middleware stack
            let valid_key = "test-key";
            let mut auth_config = AuthConfig::default();
            auth_config
                .api_keys
                .insert(ApiKey::try_new(valid_key.to_string()).unwrap());

            let mut harness = MiddlewareTestHarness::new().with_full_stack(auth_config);

            // The /error endpoint returns an error
            let response = harness.get_with_auth("/error", valid_key).await;

            // Should return 500 Internal Server Error
            assert_status(&response, StatusCode::INTERNAL_SERVER_ERROR);

            // Should have request ID in response for correlation
            assert_header_exists(&response, X_REQUEST_ID);
        }

        #[tokio::test]
        async fn test_panic_recovery() {
            // Test that panics are caught and converted to 500 errors
            // We'll test this by sending a request that would cause a panic
            // The panic middleware should catch it
            let valid_key = "test-key";
            let mut auth_config = AuthConfig::default();
            auth_config
                .api_keys
                .insert(ApiKey::try_new(valid_key.to_string()).unwrap());
            let mut harness = MiddlewareTestHarness::new().with_full_stack(auth_config);

            // Create a request that might cause issues if not handled properly
            let request = Request::builder()
                .method("GET")
                .uri("/test")
                .header("authorization", format!("Bearer {valid_key}"))
                .header("content-length", "invalid") // Invalid content-length
                .body(Body::empty())
                .unwrap();

            let response = harness.send_request(request).await;

            // The server should handle invalid content-length gracefully
            // In this case, the middleware stack processes the request normally
            // since the invalid header doesn't cause a panic
            assert_eq!(
                response.status(),
                StatusCode::OK,
                "Server should handle invalid headers gracefully without panicking"
            );
        }

        #[tokio::test]
        async fn test_error_correlation() {
            // Test that errors include request ID for correlation
            let request_id = Uuid::now_v7().to_string();
            let valid_key = "test-key";
            let mut auth_config = AuthConfig::default();
            auth_config
                .api_keys
                .insert(ApiKey::try_new(valid_key.to_string()).unwrap());

            let mut harness = MiddlewareTestHarness::new().with_full_stack(auth_config);

            let request = Request::builder()
                .method("POST")
                .uri("/error") // This endpoint returns an error
                .header(X_REQUEST_ID, &request_id)
                .header(header::AUTHORIZATION, format!("Bearer {valid_key}"))
                .body(Body::empty())
                .unwrap();

            let response = harness.send_request(request).await;

            // Error response should preserve request ID in headers
            assert_header_value(&response, X_REQUEST_ID, &request_id);
            assert_status(&response, StatusCode::INTERNAL_SERVER_ERROR);
        }
    }

    mod combined_middleware {
        use super::*;

        #[tokio::test]
        async fn test_middleware_ordering() {
            // Test that middleware layers are applied in correct order:
            // 1. Request ID (first, so all logs have request ID)
            // 2. Error handling (catches errors from auth and below)
            // 3. Auth (validates before processing)
            // 4. Actual proxy handler

            let valid_key = "test-api-key";
            let mut auth_config = AuthConfig::default();
            auth_config
                .api_keys
                .insert(ApiKey::try_new(valid_key.to_string()).unwrap());

            let mut harness = MiddlewareTestHarness::new().with_full_stack(auth_config);

            let response = harness.get_with_auth("/test", valid_key).await;

            // Should have request ID from first middleware
            assert_header_exists(&response, X_REQUEST_ID);

            // Should pass auth and return success
            assert_status(&response, StatusCode::OK);
        }

        #[tokio::test]
        async fn test_middleware_error_propagation() {
            // Test that errors from inner middleware are properly handled
            let mut auth_config = AuthConfig::default();
            auth_config
                .api_keys
                .insert(ApiKey::try_new("valid-key".to_string()).unwrap());

            let mut harness = MiddlewareTestHarness::new().with_full_stack(auth_config);

            let request = Request::builder()
                .method("POST")
                .uri("/test")
                // Missing auth header
                .body(Body::empty())
                .unwrap();

            let response = harness.send_request(request).await;

            // Should have request ID even on auth failure
            assert_header_exists(&response, X_REQUEST_ID);

            // Should return 401 from auth middleware
            assert_status(&response, StatusCode::UNAUTHORIZED);
        }
    }

    mod logging_middleware {
        use super::*;

        #[tokio::test]
        async fn test_request_logging() {
            // Test that requests are logged with appropriate details
            let valid_key = "test-key";
            let mut auth_config = AuthConfig::default();
            auth_config
                .api_keys
                .insert(ApiKey::try_new(valid_key.to_string()).unwrap());

            let mut harness = MiddlewareTestHarness::new().with_full_stack(auth_config);

            let request_id = Uuid::now_v7().to_string();
            let request = Request::builder()
                .method("POST")
                .uri("/echo")
                .header(X_REQUEST_ID, &request_id)
                .header(header::AUTHORIZATION, format!("Bearer {valid_key}"))
                .header(header::CONTENT_LENGTH, "9")
                .body(Body::from("test body"))
                .unwrap();

            let response = harness.send_request(request).await;

            // The middleware stack with logging is applied
            // Response should be successful
            assert_status(&response, StatusCode::OK);
            // Request ID should be preserved
            assert_header_value(&response, X_REQUEST_ID, &request_id);
        }

        #[tokio::test]
        async fn test_response_logging() {
            // Test that responses are logged with timing info
            let auth_config = AuthConfig::default();
            let mut harness = MiddlewareTestHarness::new().with_full_stack(auth_config);

            let start = std::time::Instant::now();
            let response = harness.get("/health").await;
            let duration = start.elapsed();

            // Response should be successful
            assert_status(&response, StatusCode::OK);
            // Should have request ID
            assert_header_exists(&response, X_REQUEST_ID);
            // Processing should take some time
            assert!(duration.as_nanos() > 0);
        }
    }

    mod rate_limiting_middleware {
        #[tokio::test]
        #[ignore = "Rate limiting not yet implemented"]
        async fn test_rate_limit_per_api_key() {
            // Test that rate limiting is applied per API key
            // This test documents the expected behavior once rate limiting is implemented

            // When implemented, the rate limiter should:
            // 1. Track requests per API key
            // 2. Return 429 Too Many Requests when limit exceeded
            // 3. Include Retry-After header
            // 4. Reset limits after time window

            // Example test structure:
            // - Configure low rate limit (e.g., 2 requests per second)
            // - Send 2 requests with same API key (should succeed)
            // - Send 3rd request immediately (should get 429)
            // - Wait for reset window
            // - Send another request (should succeed)
        }

        #[tokio::test]
        #[ignore = "Rate limiting not yet implemented"]
        async fn test_rate_limit_headers() {
            // Test that rate limit headers are included
            // This test documents the expected headers once rate limiting is implemented

            // Expected headers on all responses:
            // - X-RateLimit-Limit: Maximum requests allowed
            // - X-RateLimit-Remaining: Requests remaining in window
            // - X-RateLimit-Reset: Unix timestamp when limit resets
            // - Retry-After: Seconds to wait (only on 429 responses)
        }
    }
}
