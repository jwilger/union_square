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
        let router = service.into_router();

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
