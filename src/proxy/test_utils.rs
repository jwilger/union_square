//! Test utilities for proxy module testing
//!
//! This module provides utilities to make testing proxy components easier,
//! including mock services, test configurations, and helper functions.

#[cfg(test)]
pub mod test_helpers {
    use crate::proxy::{types::*, AuthConfig, ProxyConfig};
    use axum::{body::Body, http::StatusCode, response::IntoResponse};
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::sync::mpsc;

    /// Create a test proxy configuration with sensible defaults
    pub fn test_proxy_config() -> ProxyConfig {
        ProxyConfig {
            request_timeout: Duration::from_secs(5),
            max_request_size: RequestSizeLimit::try_new(1024 * 1024).expect("1MB is valid"), // 1MB
            max_response_size: ResponseSizeLimit::try_new(10 * 1024 * 1024).expect("10MB is valid"), // 10MB
            ring_buffer: test_ring_buffer_config(),
        }
    }

    /// Create a test ring buffer configuration
    pub fn test_ring_buffer_config() -> RingBufferConfig {
        RingBufferConfig {
            buffer_size: BufferSize::try_new(1024 * 1024).expect("1MB is valid power of 2"), // 1MB for tests
            slot_size: SlotSize::try_new(64 * 1024).expect("64KB is valid"),
        }
    }

    /// Create a test auth configuration with predefined API keys
    pub fn test_auth_config() -> AuthConfig {
        let mut auth_config = AuthConfig::default();
        auth_config.api_keys.insert(
            ApiKey::try_new("test-api-key-123".to_string()).expect("test API key should be valid"),
        );
        auth_config.api_keys.insert(
            ApiKey::try_new("test-api-key-456".to_string()).expect("test API key should be valid"),
        );
        auth_config
    }

    /// Create an auth configuration with no valid keys
    pub fn empty_auth_config() -> AuthConfig {
        AuthConfig::default()
    }

    /// Create a mock upstream server handler
    pub async fn mock_upstream_handler(
        status: StatusCode,
        body: &'static str,
    ) -> impl IntoResponse {
        (status, body)
    }

    /// Create a mock upstream server that echoes requests
    pub async fn echo_handler(
        req: axum::extract::Request,
    ) -> Result<impl IntoResponse, StatusCode> {
        let (parts, body) = req.into_parts();
        let body_bytes = axum::body::to_bytes(body, usize::MAX)
            .await
            .map_err(|_| StatusCode::BAD_REQUEST)?;

        let response_body = serde_json::json!({
            "method": parts.method.to_string(),
            "uri": parts.uri.to_string(),
            "headers": parts.headers.iter()
                .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("<binary>").to_string()))
                .collect::<Vec<_>>(),
            "body": String::from_utf8_lossy(&body_bytes),
        });

        Ok((StatusCode::OK, axum::Json(response_body)))
    }

    /// Create a mock upstream server that delays responses
    pub async fn slow_handler(delay: Duration) -> impl IntoResponse {
        tokio::time::sleep(delay).await;
        (StatusCode::OK, "Slow response")
    }

    /// Create a mock upstream server that returns large responses
    pub async fn large_response_handler(size_mb: usize) -> impl IntoResponse {
        let data = "x".repeat(size_mb * 1024 * 1024);
        (StatusCode::OK, data)
    }

    /// Helper to create a test request with authentication
    pub fn authenticated_request(path: &str, api_key: &str) -> axum::http::Request<Body> {
        axum::http::Request::builder()
            .uri(path)
            .header("Authorization", format!("Bearer {api_key}"))
            .header("X-Target-Url", "https://api.example.com")
            .body(Body::empty())
            .expect("test request should be valid")
    }

    /// Helper to create a test request without authentication
    pub fn unauthenticated_request(path: &str) -> axum::http::Request<Body> {
        axum::http::Request::builder()
            .uri(path)
            .header("X-Target-Url", "https://api.example.com")
            .body(Body::empty())
            .expect("test request should be valid")
    }

    /// Mock ring buffer for testing without actual storage
    pub struct MockRingBuffer {
        tx: mpsc::UnboundedSender<Vec<u8>>,
        rx: Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<Vec<u8>>>>,
    }

    impl MockRingBuffer {
        pub fn new() -> (Self, mpsc::UnboundedReceiver<Vec<u8>>) {
            let (tx, rx) = mpsc::unbounded_channel();
            let (_tx2, rx2) = mpsc::unbounded_channel();

            (
                Self {
                    tx,
                    rx: Arc::new(tokio::sync::Mutex::new(rx2)),
                },
                rx,
            )
        }

        pub fn write(
            &self,
            _key: RequestId,
            data: &[u8],
        ) -> Result<(), tokio::sync::mpsc::error::SendError<Vec<u8>>> {
            self.tx.send(data.to_vec())
        }

        pub async fn get_events(&self) -> Vec<Vec<u8>> {
            let mut events = Vec::new();
            let mut rx = self.rx.lock().await;
            while let Ok(event) = rx.try_recv() {
                events.push(event);
            }
            events
        }
    }

    /// Assert that an audit event was recorded
    #[macro_export]
    macro_rules! assert_audit_event {
        ($events:expr, $pattern:pat) => {
            assert!(
                $events.iter().any(|e| matches!(e.event_type, $pattern)),
                "Expected audit event matching pattern not found"
            );
        };
    }

    /// Assert response has expected status and contains request ID
    #[macro_export]
    macro_rules! assert_response_ok {
        ($response:expr) => {
            assert_eq!($response.status(), StatusCode::OK);
            assert!($response.headers().contains_key("X-Request-Id"));
        };
        ($response:expr, $status:expr) => {
            assert_eq!($response.status(), $status);
            assert!($response.headers().contains_key("X-Request-Id"));
        };
    }
}

#[cfg(test)]
mod tests {
    use super::test_helpers::*;
    use crate::proxy::types::{ApiKey, RequestId};

    #[test]
    fn test_config_creation() {
        let config = test_proxy_config();
        // Can't directly compare nutype values with integers, check via AsRef
        assert_eq!(*config.max_request_size.as_ref(), 1024 * 1024);
        assert_eq!(*config.ring_buffer.buffer_size.as_ref(), 1024 * 1024);
    }

    #[test]
    fn test_auth_config_creation() {
        let auth = test_auth_config();
        assert_eq!(auth.api_keys.len(), 2);
        assert!(auth
            .api_keys
            .contains(&ApiKey::try_new("test-api-key-123".to_string()).unwrap()));
    }

    #[tokio::test]
    async fn test_mock_ring_buffer() {
        let (mock_rb, mut rx) = MockRingBuffer::new();
        let request_id = RequestId::new();

        mock_rb.write(request_id, b"test data").unwrap();

        let event = rx.recv().await.unwrap();
        assert_eq!(event, b"test data");
    }
}
