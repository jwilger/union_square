//! Tests for error handling and edge cases

use crate::proxy::middleware::AuthConfig;
use crate::proxy::service::ProxyService;
use crate::proxy::types::*;
use axum::http::StatusCode;
use std::time::Duration;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_messages() {
        // Test that all error variants have meaningful messages
        let errors = vec![
            ProxyError::RequestTooLarge {
                size: BodySize::from(2000),
                max_size: RequestSizeLimit::try_new(1000).unwrap(),
            },
            ProxyError::ResponseTooLarge {
                size: BodySize::from(2000),
                max_size: ResponseSizeLimit::try_new(1000).unwrap(),
            },
            ProxyError::RequestTimeout(Duration::from_secs(30)),
            ProxyError::InvalidTargetUrl("not-a-url".to_string()),
            ProxyError::RingBufferOverflow {
                dropped: DroppedEventCount::from(5),
            },
            ProxyError::InvalidHttpMethod("INVALID".to_string()),
            ProxyError::InvalidRequestUri("".to_string()),
            ProxyError::InvalidHttpStatusCode(999),
            ProxyError::InvalidHeader {
                name: "bad-header".to_string(),
            },
            ProxyError::AuditEventCreationFailed("test error".to_string()),
            ProxyError::Internal("internal error".to_string()),
        ];

        for error in errors {
            let message = error.to_string();
            assert!(!message.is_empty());
            assert!(!message.contains("ProxyError")); // Should have user-friendly message
        }
    }

    #[test]
    fn test_error_to_status_code_mapping() {
        use axum::response::IntoResponse;

        let test_cases = vec![
            (
                ProxyError::RequestTooLarge {
                    size: BodySize::from(2000),
                    max_size: RequestSizeLimit::try_new(1000).unwrap(),
                },
                StatusCode::PAYLOAD_TOO_LARGE,
            ),
            (
                ProxyError::ResponseTooLarge {
                    size: BodySize::from(2000),
                    max_size: ResponseSizeLimit::try_new(1000).unwrap(),
                },
                StatusCode::INSUFFICIENT_STORAGE,
            ),
            (
                ProxyError::RequestTimeout(Duration::from_secs(30)),
                StatusCode::REQUEST_TIMEOUT,
            ),
            (
                ProxyError::InvalidTargetUrl("bad-url".to_string()),
                StatusCode::BAD_REQUEST,
            ),
            (
                ProxyError::InvalidHttpMethod("INVALID".to_string()),
                StatusCode::BAD_REQUEST,
            ),
            (
                ProxyError::InvalidRequestUri("".to_string()),
                StatusCode::BAD_REQUEST,
            ),
            (
                ProxyError::InvalidHttpStatusCode(999),
                StatusCode::BAD_GATEWAY,
            ),
            (
                ProxyError::InvalidHeader {
                    name: "bad-header".to_string(),
                },
                StatusCode::BAD_REQUEST,
            ),
            (
                ProxyError::AuditEventCreationFailed("error".to_string()),
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
            (
                ProxyError::Internal("error".to_string()),
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
        ];

        for (error, expected_status) in test_cases {
            let response = error.into_response();
            assert_eq!(response.status(), expected_status);
        }
    }

    #[test]
    fn test_type_validation_edge_cases() {
        // Test boundary conditions for validated types

        // RequestSizeLimit must be > 0
        assert!(RequestSizeLimit::try_new(0).is_err());
        assert!(RequestSizeLimit::try_new(1).is_ok());
        assert!(RequestSizeLimit::try_new(usize::MAX).is_ok());

        // BufferSize must be > 0 and power of 2
        assert!(BufferSize::try_new(0).is_err());
        assert!(BufferSize::try_new(3).is_err()); // Not power of 2
        assert!(BufferSize::try_new(1024).is_ok());
        assert!(BufferSize::try_new(1024 * 1024).is_ok());

        // SlotSize must be > 0
        assert!(SlotSize::try_new(0).is_err());
        assert!(SlotSize::try_new(1).is_ok());

        // HttpMethod must not be empty
        assert!(HttpMethod::try_new("".to_string()).is_err());
        assert!(HttpMethod::try_new("GET".to_string()).is_ok());
        assert!(HttpMethod::try_new("CUSTOM-METHOD".to_string()).is_ok());

        // RequestUri must not be empty
        assert!(RequestUri::try_new("".to_string()).is_err());
        assert!(RequestUri::try_new("/".to_string()).is_ok());
        assert!(RequestUri::try_new("/path/to/resource".to_string()).is_ok());

        // HttpStatusCode must be 100-599
        assert!(HttpStatusCode::try_new(99).is_err());
        assert!(HttpStatusCode::try_new(100).is_ok());
        assert!(HttpStatusCode::try_new(599).is_ok());
        assert!(HttpStatusCode::try_new(600).is_err());

        // TargetUrl must start with http:// or https://
        assert!(TargetUrl::try_new("".to_string()).is_err());
        assert!(TargetUrl::try_new("ftp://example.com".to_string()).is_err());
        assert!(TargetUrl::try_new("http://example.com".to_string()).is_ok());
        assert!(TargetUrl::try_new("https://example.com".to_string()).is_ok());

        // BypassPath must start with /
        assert!(BypassPath::try_new("".to_string()).is_err());
        assert!(BypassPath::try_new("health".to_string()).is_err());
        assert!(BypassPath::try_new("/health".to_string()).is_ok());

        // ApiKey must not be empty
        assert!(ApiKey::try_new("".to_string()).is_err());
        assert!(ApiKey::try_new("valid-key".to_string()).is_ok());
    }

    #[test]
    fn test_headers_collection() {
        // Test Headers collection functionality
        let headers = Headers::new();
        assert_eq!(headers.as_vec().len(), 0);

        // Test from_vec with valid headers
        let header_vec = vec![
            ("Content-Type".to_string(), "application/json".to_string()),
            ("Authorization".to_string(), "Bearer token".to_string()),
        ];

        let headers = Headers::from_vec(header_vec).unwrap();
        assert_eq!(headers.as_vec().len(), 2);

        // Test with invalid header name
        let invalid_headers = vec![
            ("".to_string(), "value".to_string()), // Empty header name
        ];

        assert!(Headers::from_vec(invalid_headers).is_err());
    }

    #[test]
    fn test_request_id_generation() {
        // Test that RequestId generates valid v7 UUIDs
        let id1 = RequestId::new();
        let id2 = RequestId::new();

        // Should be different
        assert_ne!(id1.as_ref(), id2.as_ref());

        // Should be v7 UUIDs
        assert_eq!(id1.as_ref().get_version_num(), 7);
        assert_eq!(id2.as_ref().get_version_num(), 7);

        // Test default trait
        let default_id = RequestId::default();
        assert_eq!(default_id.as_ref().get_version_num(), 7);
    }

    #[test]
    fn test_session_id_generation() {
        // Test that SessionId generates valid v7 UUIDs
        let id1 = SessionId::new();
        let id2 = SessionId::new();

        // Should be different
        assert_ne!(id1.as_ref(), id2.as_ref());

        // Should be v7 UUIDs
        assert_eq!(id1.as_ref().get_version_num(), 7);
        assert_eq!(id2.as_ref().get_version_num(), 7);

        // Test default trait
        let default_id = SessionId::default();
        assert_eq!(default_id.as_ref().get_version_num(), 7);
    }

    #[test]
    fn test_audit_event_serialization() {
        // Test that audit events can be serialized/deserialized
        let event = AuditEvent {
            request_id: RequestId::new(),
            session_id: SessionId::new(),
            timestamp: chrono::Utc::now(),
            event_type: AuditEventType::RequestReceived {
                method: HttpMethod::try_new("POST".to_string()).unwrap(),
                uri: RequestUri::try_new("/api/test".to_string()).unwrap(),
                headers: Headers::new(),
                body_size: BodySize::from(1024),
            },
        };

        // Serialize
        let serialized = serde_json::to_string(&event).unwrap();
        assert!(!serialized.is_empty());

        // Deserialize
        let deserialized: AuditEvent = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.request_id.as_ref(), event.request_id.as_ref());

        // Test error event
        let error_event = AuditEvent {
            request_id: RequestId::new(),
            session_id: SessionId::new(),
            timestamp: chrono::Utc::now(),
            event_type: AuditEventType::Error {
                error: "Test error".to_string(),
                phase: ErrorPhase::RequestParsing,
            },
        };

        let serialized = serde_json::to_string(&error_event).unwrap();
        let deserialized: AuditEvent = serde_json::from_str(&serialized).unwrap();

        match deserialized.event_type {
            AuditEventType::Error { error, phase } => {
                assert_eq!(error, "Test error");
                matches!(phase, ErrorPhase::RequestParsing);
            }
            _ => panic!("Expected Error event type"),
        }
    }

    #[test]
    fn test_config_defaults() {
        let config = ProxyConfig::default();

        // Check defaults are reasonable
        assert_eq!(*config.max_request_size.as_ref(), 10 * 1024 * 1024); // 10MB
        assert_eq!(*config.max_response_size.as_ref(), 10 * 1024 * 1024); // 10MB
        assert_eq!(config.request_timeout, Duration::from_secs(30));

        let ring_config = config.ring_buffer;
        assert_eq!(*ring_config.buffer_size.as_ref(), 1024 * 1024 * 1024); // 1GB
        assert_eq!(*ring_config.slot_size.as_ref(), 64 * 1024); // 64KB
    }

    #[tokio::test]
    async fn test_proxy_service_creation() {
        let config = ProxyConfig::default();
        let service = ProxyService::new(config.clone());

        // Service should be created with ring buffer
        let stats = service.ring_buffer().stats();
        assert_eq!(stats.total_writes, 0);
        assert_eq!(stats.total_reads, 0);
        assert_eq!(*stats.dropped_events.as_ref(), 0);

        // Should be able to create router
        let auth_config = AuthConfig::default();
        let service = ProxyService::new(config);
        let _router = service.into_router(auth_config);
    }

    #[test]
    fn test_error_phase_variants() {
        // Ensure all error phases are covered
        let phases = vec![
            ErrorPhase::RequestParsing,
            ErrorPhase::RequestForwarding,
            ErrorPhase::ResponseReceiving,
            ErrorPhase::ResponseReturning,
            ErrorPhase::AuditRecording,
        ];

        for phase in phases {
            let event = AuditEvent {
                request_id: RequestId::new(),
                session_id: SessionId::new(),
                timestamp: chrono::Utc::now(),
                event_type: AuditEventType::Error {
                    error: "Test".to_string(),
                    phase,
                },
            };

            // Should serialize/deserialize correctly
            let serialized = serde_json::to_string(&event).unwrap();
            let _deserialized: AuditEvent = serde_json::from_str(&serialized).unwrap();
        }
    }
}
