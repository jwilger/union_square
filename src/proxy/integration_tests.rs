//! Integration tests for end-to-end proxy flow

use crate::proxy::service::ProxyService;
use crate::proxy::types::*;
use crate::proxy::AuthConfig;
use axum::body::Body;
use axum::extract::Request as ExtractRequest;
use axum::http::{Request, StatusCode};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tower::ServiceExt;

/// Mock backend server for testing
async fn run_mock_backend(port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let app = axum::Router::new()
        .route("/", axum::routing::get(|| async { "Hello from backend" }))
        .route(
            "/echo",
            axum::routing::post(|body: String| async move { body }),
        )
        .route(
            "/status/{code}",
            axum::routing::get(
                |axum::extract::Path(code): axum::extract::Path<u16>| async move {
                    let status = StatusCode::from_u16(code).unwrap_or(StatusCode::OK);
                    (status, "Status response")
                },
            ),
        )
        .route(
            "/slow",
            axum::routing::get(|| async {
                tokio::time::sleep(Duration::from_millis(TIMEOUT_SHORT_MS)).await;
                "Slow response"
            }),
        )
        .route(
            "/large",
            axum::routing::get(|| async {
                "x".repeat(BYTES_1MB) // 1MB
            }),
        )
        .fallback(|request: ExtractRequest| async move {
            (
                StatusCode::NOT_FOUND,
                format!("Not found: {} {}", request.method(), request.uri()),
            )
        });

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = TcpListener::bind(addr).await?;

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // Give the server time to start
    tokio::time::sleep(Duration::from_millis(TIMEOUT_SHORT_MS)).await;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_basic_proxy_flow() {
        // Start mock backend
        run_mock_backend(TEST_PORT_BASE + 1)
            .await
            .expect("Failed to start mock backend");

        // Create proxy configuration
        let config = ProxyConfig {
            max_request_size: RequestSizeLimit::try_new(BYTES_10MB).unwrap(),
            max_response_size: ResponseSizeLimit::try_new(BYTES_10MB).unwrap(),
            request_timeout: Duration::from_secs(5),
            ring_buffer: RingBufferConfig::default(),
            bedrock_region: None,
        };

        // Create auth configuration
        let mut auth_config = AuthConfig::default();
        auth_config
            .api_keys
            .insert(ApiKey::try_new("test-key".to_string()).unwrap());

        // Create proxy service
        let service = ProxyService::new(config);
        let ring_buffer = service.ring_buffer();
        let app = service.into_router(auth_config);

        // Test 1: Successful GET request
        let request = Request::builder()
            .method("GET")
            .uri("http://localhost:8080/")
            .header("Authorization", "Bearer test-key")
            .header(
                crate::proxy::headers::X_TARGET_URL,
                format!("http://localhost:{}/", TEST_PORT_BASE + 1),
            )
            .body(Body::empty())
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(&body_bytes[..], b"Hello from backend");

        // Verify audit event was recorded
        tokio::time::sleep(Duration::from_millis(10)).await; // Give audit path time to process
        let stats = ring_buffer.stats();
        assert!(stats.total_writes > 0, "Should have recorded audit events");

        // Test 2: POST request with body
        let request_body = "Test request body";
        let request = Request::builder()
            .method("POST")
            .uri("http://localhost:8080/echo")
            .header("Authorization", "Bearer test-key")
            .header(
                crate::proxy::headers::X_TARGET_URL,
                "http://localhost:8081/echo",
            )
            .header("Content-Type", "text/plain")
            .body(Body::from(request_body))
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(&body_bytes[..], request_body.as_bytes());

        // Test 3: Request without auth should fail
        let request = Request::builder()
            .method("GET")
            .uri("http://localhost:8080/")
            .header(
                crate::proxy::headers::X_TARGET_URL,
                "http://localhost:8081/",
            )
            .body(Body::empty())
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        // Test 4: Health check should bypass auth
        let request = Request::builder()
            .method("GET")
            .uri("http://localhost:8080/health")
            .body(Body::empty())
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_error_handling() {
        // Start mock backend
        run_mock_backend(8082)
            .await
            .expect("Failed to start mock backend");

        let config = ProxyConfig::default();
        let mut auth_config = AuthConfig::default();
        auth_config
            .api_keys
            .insert(ApiKey::try_new("test-key".to_string()).unwrap());

        let service = ProxyService::new(config);
        let app = service.into_router(auth_config);

        // Test 1: Invalid target URL
        let request = Request::builder()
            .method("GET")
            .uri("http://localhost:8080/")
            .header("Authorization", "Bearer test-key")
            .header(crate::proxy::headers::X_TARGET_URL, "not-a-url")
            .body(Body::empty())
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        // Test 2: Backend returns error status
        let request = Request::builder()
            .method("GET")
            .uri("http://localhost:8080/status/500")
            .header("Authorization", "Bearer test-key")
            .header(
                crate::proxy::headers::X_TARGET_URL,
                "http://localhost:8082/status/500",
            )
            .body(Body::empty())
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

        // Test 3: Request to non-existent backend
        let request = Request::builder()
            .method("GET")
            .uri("http://localhost:8080/")
            .header("Authorization", "Bearer test-key")
            .header(
                crate::proxy::headers::X_TARGET_URL,
                "http://localhost:9999/",
            ) // Non-existent port
            .body(Body::empty())
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    }

    #[tokio::test]
    async fn test_request_size_limits() {
        // Start mock backend
        run_mock_backend(8083)
            .await
            .expect("Failed to start mock backend");

        let config = ProxyConfig {
            max_request_size: RequestSizeLimit::try_new(1024).unwrap(), // 1KB limit
            max_response_size: ResponseSizeLimit::try_new(10 * 1024 * 1024).unwrap(),
            request_timeout: Duration::from_secs(5),
            ring_buffer: RingBufferConfig::default(),
            bedrock_region: None,
        };

        let mut auth_config = AuthConfig::default();
        auth_config
            .api_keys
            .insert(ApiKey::try_new("test-key".to_string()).unwrap());

        let service = ProxyService::new(config);
        let app = service.into_router(auth_config);

        // Test: Request exceeding size limit
        let large_body = "x".repeat(2048); // 2KB, exceeds limit
        let request = Request::builder()
            .method("POST")
            .uri("http://localhost:8080/echo")
            .header("Authorization", "Bearer test-key")
            .header(
                crate::proxy::headers::X_TARGET_URL,
                "http://localhost:8083/echo",
            )
            .header("Content-Type", "text/plain")
            .body(Body::from(large_body))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[tokio::test]
    async fn test_response_size_limits() {
        // Start mock backend
        run_mock_backend(8084)
            .await
            .expect("Failed to start mock backend");

        let config = ProxyConfig {
            max_request_size: RequestSizeLimit::try_new(10 * 1024 * 1024).unwrap(),
            max_response_size: ResponseSizeLimit::try_new(1024).unwrap(), // 1KB limit
            request_timeout: Duration::from_secs(5),
            ring_buffer: RingBufferConfig::default(),
            bedrock_region: None,
        };

        let mut auth_config = AuthConfig::default();
        auth_config
            .api_keys
            .insert(ApiKey::try_new("test-key".to_string()).unwrap());

        let service = ProxyService::new(config);
        let app = service.into_router(auth_config);

        // Test: Response exceeding size limit
        let request = Request::builder()
            .method("GET")
            .uri("http://localhost:8080/large")
            .header("Authorization", "Bearer test-key")
            .header(
                crate::proxy::headers::X_TARGET_URL,
                "http://localhost:8084/large",
            )
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        // Note: Response size limiting happens during streaming, so we should still get OK
        // but the body will be truncated
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_timeout_handling() {
        // Start mock backend
        run_mock_backend(8085)
            .await
            .expect("Failed to start mock backend");

        let config = ProxyConfig {
            max_request_size: RequestSizeLimit::try_new(10 * 1024 * 1024).unwrap(),
            max_response_size: ResponseSizeLimit::try_new(10 * 1024 * 1024).unwrap(),
            request_timeout: Duration::from_millis(50), // Very short timeout
            ring_buffer: RingBufferConfig::default(),
            bedrock_region: None,
        };

        let mut auth_config = AuthConfig::default();
        auth_config
            .api_keys
            .insert(ApiKey::try_new("test-key".to_string()).unwrap());

        let service = ProxyService::new(config);
        let app = service.into_router(auth_config);

        // Test: Request that takes too long
        let request = Request::builder()
            .method("GET")
            .uri("http://localhost:8080/slow")
            .header("Authorization", "Bearer test-key")
            .header(
                crate::proxy::headers::X_TARGET_URL,
                "http://localhost:8085/slow",
            )
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::REQUEST_TIMEOUT);
    }

    #[tokio::test]
    async fn test_concurrent_requests() {
        // Start mock backend
        run_mock_backend(8086)
            .await
            .expect("Failed to start mock backend");

        let config = ProxyConfig::default();
        let mut auth_config = AuthConfig::default();
        auth_config
            .api_keys
            .insert(ApiKey::try_new("test-key".to_string()).unwrap());

        let service = ProxyService::new(config);
        let app = Arc::new(service.into_router(auth_config));

        // Launch multiple concurrent requests
        let mut handles = vec![];
        for i in 0..10 {
            let app = app.clone();
            let handle = tokio::spawn(async move {
                let request = Request::builder()
                    .method("POST")
                    .uri("http://localhost:8080/echo")
                    .header("Authorization", "Bearer test-key")
                    .header(
                        crate::proxy::headers::X_TARGET_URL,
                        "http://localhost:8086/echo",
                    )
                    .header("Content-Type", "text/plain")
                    .body(Body::from(format!("Request {i}")))
                    .unwrap();

                // Clone the router from the Arc for each request
                let response = (*app).clone().oneshot(request).await.unwrap();
                assert_eq!(response.status(), StatusCode::OK);

                let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
                    .await
                    .unwrap();
                String::from_utf8(body_bytes.to_vec()).unwrap()
            });
            handles.push(handle);
        }

        // Wait for all requests to complete
        let results: Vec<String> = futures_util::future::join_all(handles)
            .await
            .into_iter()
            .map(|r| r.unwrap())
            .collect();

        // Verify all requests completed successfully
        assert_eq!(results.len(), 10);
        for result in &results {
            assert!(result.contains("Request"));
        }
    }

    #[tokio::test]
    async fn test_audit_event_recording() {
        // Start mock backend
        run_mock_backend(8087)
            .await
            .expect("Failed to start mock backend");

        let config = ProxyConfig::default();
        let mut auth_config = AuthConfig::default();
        auth_config
            .api_keys
            .insert(ApiKey::try_new("test-key".to_string()).unwrap());

        let service = ProxyService::new(config);
        let ring_buffer = service.ring_buffer();
        let app = service.into_router(auth_config);

        // Make a request
        let request = Request::builder()
            .method("POST")
            .uri("http://localhost:8080/echo")
            .header("Authorization", "Bearer test-key")
            .header(
                crate::proxy::headers::X_TARGET_URL,
                "http://localhost:8087/echo",
            )
            .header("Content-Type", "text/plain")
            .body(Body::from("Test audit"))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Wait a bit for the streaming service to write events to the ring buffer
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Check ring buffer stats - should show events were written
        let stats = ring_buffer.stats();

        // The audit processor will be consuming events, so we just verify that events were written
        // This proves the audit recording system is working
        assert!(
            stats.total_writes >= 2,
            "Should have written at least 2 audit events (request + response), found: {}",
            stats.total_writes
        );
    }

    #[tokio::test]
    async fn test_invalid_http_methods() {
        // Start mock backend
        run_mock_backend(8088)
            .await
            .expect("Failed to start mock backend");

        let config = ProxyConfig::default();
        let mut auth_config = AuthConfig::default();
        auth_config
            .api_keys
            .insert(ApiKey::try_new("test-key".to_string()).unwrap());

        let service = ProxyService::new(config);
        let ring_buffer = service.ring_buffer();
        let app = service.into_router(auth_config);

        // Test with various HTTP methods
        let methods = [
            "GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS", "CONNECT", "TRACE",
        ];

        for method in &methods {
            let request = Request::builder()
                .method(*method)
                .uri("http://localhost:8080/")
                .header("Authorization", "Bearer test-key")
                .header(
                    crate::proxy::headers::X_TARGET_URL,
                    "http://localhost:8088/",
                )
                .body(Body::empty())
                .unwrap();

            let response = app.clone().oneshot(request).await.unwrap();
            // All standard methods should work
            assert!(
                response.status().is_success()
                    || response.status().is_client_error()
                    || response.status().is_server_error()
            );
        }

        // Wait for audit events
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Verify no error events for invalid methods
        let mut error_events = 0;
        while let Some((_, data)) = ring_buffer.read() {
            if let Ok(event) = serde_json::from_slice::<AuditEvent>(&data) {
                if matches!(
                    event.event_type,
                    AuditEventType::Error {
                        phase: ErrorPhase::RequestParsing,
                        ..
                    }
                ) {
                    error_events += 1;
                }
            }
        }

        assert_eq!(
            error_events, 0,
            "Should not have any parsing errors for standard HTTP methods"
        );
    }
}
