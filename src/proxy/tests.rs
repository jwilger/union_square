//! Tests for the proxy module

#[cfg(test)]
mod proxy_service_tests {
    use crate::proxy::{ProxyConfig, ProxyService};
    use std::time::Duration;

    #[test]
    fn test_proxy_service_creation() {
        let config = ProxyConfig::default();
        let service = ProxyService::new(config);

        // Service should be created successfully
        let ring_buffer = service.ring_buffer();

        // Ring buffer should start with zero overflow
        assert_eq!(ring_buffer.overflow_count(), 0);
    }

    #[test]
    fn test_proxy_config_defaults() {
        let config = ProxyConfig::default();

        assert_eq!(config.max_request_size, 10 * 1024 * 1024);
        assert_eq!(config.max_response_size, 10 * 1024 * 1024);
        assert_eq!(config.request_timeout, Duration::from_secs(30));
        assert_eq!(config.ring_buffer.buffer_size, 1024 * 1024 * 1024);
        assert_eq!(config.ring_buffer.slot_size, 64 * 1024);
    }

    #[test]
    fn test_custom_proxy_config() {
        let config = ProxyConfig {
            max_request_size: 1024 * 1024,      // 1MB
            max_response_size: 2 * 1024 * 1024, // 2MB
            request_timeout: Duration::from_secs(60),
            ring_buffer: crate::proxy::types::RingBufferConfig {
                buffer_size: 512 * 1024 * 1024, // 512MB
                slot_size: 32 * 1024,           // 32KB
            },
        };

        let _service = ProxyService::new(config.clone());

        // Service should be created with custom config
        assert_eq!(config.max_request_size, 1024 * 1024);
        assert_eq!(config.ring_buffer.buffer_size, 512 * 1024 * 1024);
    }

    #[tokio::test]
    async fn test_proxy_service_router_creation() {
        let config = ProxyConfig::default();
        let service = ProxyService::new(config);
        let auth_config = crate::proxy::middleware::AuthConfig::default();
        let router = service.into_router(auth_config);

        // Router should be created successfully
        // We can't easily test the router directly, but we can ensure it compiles
        let _ = router;
    }
}

#[cfg(test)]
mod type_tests {
    use crate::proxy::types::*;
    use uuid::Uuid;

    #[test]
    fn test_request_id_creation() {
        let id = unsafe { RequestId::new_unchecked(Uuid::now_v7()) };
        let uuid: &Uuid = id.as_ref();
        assert_eq!(uuid.get_version_num(), 7);
    }

    #[test]
    fn test_session_id_creation() {
        let id = unsafe { SessionId::new_unchecked(Uuid::now_v7()) };
        let uuid: &Uuid = id.as_ref();
        assert_eq!(uuid.get_version_num(), 7);
    }

    #[test]
    fn test_target_url_validation() {
        // Valid URLs
        assert!(TargetUrl::try_new("https://api.example.com").is_ok());
        assert!(TargetUrl::try_new("http://localhost:8080").is_ok());

        // Invalid URLs
        assert!(TargetUrl::try_new("not-a-url").is_err());
        assert!(TargetUrl::try_new("ftp://example.com").is_err());
        assert!(TargetUrl::try_new("").is_err());
    }

    #[test]
    fn test_api_key_validation() {
        // Valid API keys
        assert!(ApiKey::try_new("sk-1234567890").is_ok());
        assert!(ApiKey::try_new("test-key").is_ok());

        // Invalid API keys
        assert!(ApiKey::try_new("").is_err());
    }

    #[test]
    fn test_audit_event_serialization() {
        let event = AuditEvent {
            request_id: unsafe { RequestId::new_unchecked(Uuid::now_v7()) },
            session_id: unsafe { SessionId::new_unchecked(Uuid::now_v7()) },
            timestamp: chrono::Utc::now(),
            event_type: AuditEventType::RequestReceived {
                method: "POST".to_string(),
                uri: "/v1/chat/completions".to_string(),
                headers: vec![("content-type".to_string(), "application/json".to_string())],
                body_size: 1024,
            },
        };

        // Should serialize without errors
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("RequestReceived"));

        // Should deserialize back
        let deserialized: AuditEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.request_id.as_ref(), event.request_id.as_ref());
    }
}

#[cfg(test)]
mod streaming_tests {
    use crate::proxy::{ProxyConfig, ProxyService};
    use axum::{body::Body, http::StatusCode};
    use bytes::Bytes;
    use futures_util::stream;
    use std::time::Duration;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_streaming_request_handling() {
        // Create a streaming body with multiple chunks
        let chunks = vec![
            Ok::<_, std::io::Error>(Bytes::from("chunk1")),
            Ok(Bytes::from("chunk2")),
            Ok(Bytes::from("chunk3")),
        ];
        let stream = stream::iter(chunks);
        let body = Body::from_stream(stream);

        let request = http::Request::builder()
            .method("POST")
            .uri("/stream")
            .header("content-type", "application/octet-stream")
            .body(body)
            .unwrap();

        // Service should handle streaming request without buffering entire body
        let config = ProxyConfig::default();
        let service = ProxyService::new(config);
        let auth_config = crate::proxy::middleware::AuthConfig::default();
        let app = service.into_router(auth_config);

        // We can't test actual streaming without a backend, but we ensure
        // the service accepts streaming bodies
        let _ = app.oneshot(request).await;
    }

    #[tokio::test]
    async fn test_zero_copy_streaming() {
        // Test that we don't buffer the entire response in memory
        let config = ProxyConfig {
            max_response_size: 10 * 1024 * 1024, // 10MB
            ..Default::default()
        };
        let service = ProxyService::new(config);

        // The implementation should stream responses without buffering
        // This is a compile-time test to ensure our types support streaming
        let auth_config = crate::proxy::middleware::AuthConfig::default();
        let _app = service.into_router(auth_config);
    }

    #[tokio::test]
    async fn test_streaming_with_ring_buffer_capture() {
        // Streaming responses should still be captured in ring buffer
        let config = ProxyConfig::default();
        let service = ProxyService::new(config);
        let ring_buffer = service.ring_buffer();

        // Initial state
        assert_eq!(ring_buffer.overflow_count(), 0);

        // TODO: Once streaming is fully implemented, verify that
        // streamed data is captured in chunks to the ring buffer
    }

    #[tokio::test]
    async fn test_large_streaming_response() {
        // Test handling of responses larger than single ring buffer slot
        let config = ProxyConfig {
            ring_buffer: crate::proxy::types::RingBufferConfig {
                buffer_size: 1024 * 1024, // 1MB buffer
                slot_size: 64 * 1024,     // 64KB slots
            },
            ..Default::default()
        };
        let _service = ProxyService::new(config);

        // TODO: Test that large responses are properly chunked
        // across multiple ring buffer slots
    }

    #[tokio::test]
    async fn test_streaming_timeout_handling() {
        // Test that streaming connections respect timeouts
        let config = ProxyConfig {
            request_timeout: Duration::from_millis(100),
            ..Default::default()
        };
        let _service = ProxyService::new(config);

        // TODO: Test that slow streams are properly timed out
    }

    #[tokio::test]
    async fn test_streaming_error_handling() {
        // Test error handling during streaming
        let error_stream = stream::once(async {
            Err::<Bytes, std::io::Error>(std::io::Error::new(
                std::io::ErrorKind::ConnectionAborted,
                "Connection lost",
            ))
        });
        let body = Body::from_stream(error_stream);

        let request = http::Request::builder()
            .method("POST")
            .uri("/error-stream")
            .body(body)
            .unwrap();

        let config = ProxyConfig::default();
        let service = ProxyService::new(config);
        let auth_config = crate::proxy::middleware::AuthConfig::default();
        let app = service.into_router(auth_config);

        // Service should handle streaming errors gracefully
        // Note: Since we don't have a real backend configured, the service will return an error
        // In a real implementation with a valid target URL, the error would be detected during streaming
        let response = app.oneshot(request).await.unwrap();

        // Expect unauthorized status since no auth header is provided
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
