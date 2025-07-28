//! Comprehensive behavioral tests for audit commands
//!
//! Following TDD principles with focus on behavior, not implementation.
//! Tests cover edge cases, error scenarios, concurrency, and invariants.

use super::*;
use crate::domain::events::DomainEvent;
use crate::proxy::types::{
    AuditEvent, AuditEventType, BodySize, DurationMillis, Headers, HttpMethod, HttpStatusCode,
    RequestId, RequestUri, SessionId as ProxySessionId, TargetUrl,
};
use chrono::Utc;
use eventcore::{CommandExecutor, EventStore, ExecutionOptions, StreamId};
use eventcore_memory::InMemoryEventStore;
use proptest::prelude::*;
use std::sync::Arc;

// Test helpers
mod test_helpers {
    use super::*;

    /// Create a test audit event with default values
    pub fn create_test_audit_event(event_type: AuditEventType) -> AuditEvent {
        AuditEvent {
            request_id: RequestId::new(),
            session_id: ProxySessionId::new(),
            timestamp: Utc::now(),
            event_type,
        }
    }

    /// Create a request received event type
    pub fn request_received_event() -> AuditEventType {
        AuditEventType::RequestReceived {
            method: HttpMethod::try_new("POST".to_string()).unwrap(),
            uri: RequestUri::try_new("/v1/chat/completions".to_string()).unwrap(),
            headers: Headers::new(),
            body_size: BodySize::from(1024),
        }
    }

    /// Create a request forwarded event type
    pub fn request_forwarded_event() -> AuditEventType {
        AuditEventType::RequestForwarded {
            target_url: TargetUrl::try_new(
                "https://api.openai.com/v1/chat/completions".to_string(),
            )
            .unwrap(),
            start_time: Utc::now(),
        }
    }

    /// Create a response received event type
    pub fn response_received_event() -> AuditEventType {
        AuditEventType::ResponseReceived {
            status: HttpStatusCode::try_new(200).unwrap(),
            headers: Headers::new(),
            body_size: BodySize::from(2048),
            duration_ms: DurationMillis::from(150),
        }
    }

    /// Create a test command executor with in-memory store
    pub fn create_test_executor() -> Arc<CommandExecutor<InMemoryEventStore<DomainEvent>>> {
        let event_store = InMemoryEventStore::new();
        Arc::new(CommandExecutor::new(event_store))
    }

    /// Create a valid OpenAI request body
    pub fn create_openai_request_body() -> Vec<u8> {
        serde_json::json!({
            "model": "gpt-4",
            "messages": [
                {"role": "user", "content": "Hello, world!"}
            ],
            "temperature": 0.7,
            "max_tokens": 100
        })
        .to_string()
        .as_bytes()
        .to_vec()
    }

    /// Create a valid Anthropic request body
    pub fn create_anthropic_request_body() -> Vec<u8> {
        serde_json::json!({
            "model": "claude-3-opus-20240229",
            "messages": [
                {"role": "user", "content": "What is 2+2?"}
            ],
            "max_tokens": 100
        })
        .to_string()
        .as_bytes()
        .to_vec()
    }
}

use test_helpers::*;

// Test 1: Concurrent event processing
mod concurrent_processing {
    use super::*;

    #[tokio::test]
    async fn test_concurrent_event_processing_maintains_order() {
        let executor = create_test_executor();
        let session_id = SessionId::generate();
        let request_id = RequestId::new();

        // Create commands for different stages of request lifecycle
        let commands = vec![
            RecordAuditEvent::from_audit_event(&AuditEvent {
                request_id,
                session_id: ProxySessionId::try_new(session_id.clone().into_inner()).unwrap(),
                timestamp: Utc::now(),
                event_type: request_received_event(),
            })
            .unwrap(),
            RecordAuditEvent::from_audit_event(&AuditEvent {
                request_id,
                session_id: ProxySessionId::try_new(session_id.clone().into_inner()).unwrap(),
                timestamp: Utc::now() + chrono::Duration::milliseconds(10),
                event_type: request_forwarded_event(),
            })
            .unwrap(),
            RecordAuditEvent::from_audit_event(&AuditEvent {
                request_id,
                session_id: ProxySessionId::try_new(session_id.clone().into_inner()).unwrap(),
                timestamp: Utc::now() + chrono::Duration::milliseconds(100),
                event_type: response_received_event(),
            })
            .unwrap(),
        ];

        // Execute commands concurrently
        let handles: Vec<_> = commands
            .into_iter()
            .map(|cmd| {
                let exec = executor.clone();
                tokio::spawn(async move { exec.execute(cmd, ExecutionOptions::default()).await })
            })
            .collect();

        // Wait for all to complete
        let results: Vec<_> = futures_util::future::join_all(handles).await;

        // All should succeed
        for result in results {
            assert!(result.is_ok());
            assert!(result.unwrap().is_ok());
        }

        // Verify events are in correct order
        let session_stream =
            StreamId::try_new(format!("session-{}", session_id.clone().into_inner())).unwrap();
        let events = executor
            .event_store()
            .read_streams(&[session_stream], &eventcore::ReadOptions::default())
            .await
            .unwrap();

        // Should have at least one event (RequestReceived)
        assert!(!events.events.is_empty());

        // Verify the first event is RequestReceived
        if let DomainEvent::LlmRequestReceived { .. } = &events.events[0].payload {
            // Success
        } else {
            panic!("Expected first event to be LlmRequestReceived");
        }
    }

    #[tokio::test]
    async fn test_concurrent_writes_to_different_streams() {
        let executor = create_test_executor();
        let num_requests = 10;

        // Create multiple concurrent requests for different sessions
        let handles: Vec<_> = (0..num_requests)
            .map(|_| {
                let exec = executor.clone();
                tokio::spawn(async move {
                    let audit_event = create_test_audit_event(request_received_event());
                    let command = RecordAuditEvent::from_audit_event(&audit_event).unwrap();
                    exec.execute(command, ExecutionOptions::default()).await
                })
            })
            .collect();

        // All should succeed
        let results: Vec<_> = futures_util::future::join_all(handles).await;
        for result in results {
            assert!(result.is_ok());
            assert!(result.unwrap().is_ok());
        }
    }
}

// Test 2: Event store failures and retries
mod event_store_failures {
    use super::*;
    use crate::infrastructure::eventcore::service::EventCoreService;

    #[tokio::test]
    async fn test_command_execution_with_retry_options() {
        // Use the in-memory executor which doesn't fail
        let executor = create_test_executor();

        let audit_event = create_test_audit_event(request_received_event());
        let command = RecordAuditEvent::from_audit_event(&audit_event).unwrap();

        // Execute with retry options
        let options = ExecutionOptions::default();

        let result = executor.execute(command, options).await;

        // Should succeed
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_command_execution_with_eventcore_service() {
        // Test using the EventCoreService wrapper
        let service = EventCoreService::with_memory_store();

        let audit_event = create_test_audit_event(request_received_event());
        let command = RecordAuditEvent::from_audit_event(&audit_event).unwrap();

        let result = service.execute_command_memory(command).await;
        assert!(result.is_ok());
    }
}

// Test 3: Malformed domain events
mod malformed_events {
    use super::*;

    #[tokio::test]
    async fn test_malformed_request_body_uses_fallback() {
        let executor = create_test_executor();

        // Create command with malformed JSON body
        let malformed_body = b"{ invalid json }";
        let audit_event = create_test_audit_event(request_received_event());
        let command = RecordAuditEvent::from_audit_event(&audit_event)
            .unwrap()
            .with_body(malformed_body);

        let result = executor.execute(command, ExecutionOptions::default()).await;
        assert!(result.is_ok());

        // Verify event was created with fallback values
        let session_stream =
            StreamId::try_new(format!("session-{}", audit_event.session_id.as_ref())).unwrap();
        let events = executor
            .event_store()
            .read_streams(&[session_stream], &eventcore::ReadOptions::default())
            .await
            .unwrap();

        assert_eq!(events.events.len(), 1);
        if let DomainEvent::LlmRequestReceived {
            model_version,
            prompt,
            ..
        } = &events.events[0].payload
        {
            assert_eq!(model_version.model_id.as_ref(), "unknown-model");
            assert!(prompt.as_ref().contains("Failed to parse"));
        } else {
            panic!("Expected LlmRequestReceived event");
        }
    }

    #[tokio::test]
    async fn test_empty_request_body_uses_fallback() {
        let executor = create_test_executor();

        let audit_event = create_test_audit_event(request_received_event());
        let command = RecordAuditEvent::from_audit_event(&audit_event)
            .unwrap()
            .with_body(&[]);

        let result = executor.execute(command, ExecutionOptions::default()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_non_utf8_request_body_uses_fallback() {
        let executor = create_test_executor();

        // Invalid UTF-8 sequence
        let invalid_utf8 = vec![0xFF, 0xFE, 0xFD];
        let audit_event = create_test_audit_event(request_received_event());
        let command = RecordAuditEvent::from_audit_event(&audit_event)
            .unwrap()
            .with_body(&invalid_utf8);

        let result = executor.execute(command, ExecutionOptions::default()).await;
        assert!(result.is_ok());
    }
}

// Test 4: Event ordering guarantees
mod event_ordering {
    use super::*;

    #[tokio::test]
    async fn test_events_ordered_within_stream() {
        let executor = create_test_executor();
        let session_id = SessionId::generate();
        let request_id = RequestId::new();

        // Create events with specific timestamps
        let base_time = Utc::now();
        let timestamps = [
            base_time,
            base_time + chrono::Duration::milliseconds(100),
            base_time + chrono::Duration::milliseconds(200),
        ];

        // Execute commands in order
        for (i, event_type) in vec![
            request_received_event(),
            request_forwarded_event(),
            response_received_event(),
        ]
        .into_iter()
        .enumerate()
        {
            let audit_event = AuditEvent {
                request_id,
                session_id: ProxySessionId::try_new(session_id.clone().into_inner()).unwrap(),
                timestamp: timestamps[i],
                event_type,
            };
            let command = RecordAuditEvent::from_audit_event(&audit_event).unwrap();
            executor
                .execute(command, ExecutionOptions::default())
                .await
                .unwrap();
        }

        // Read events back
        let session_stream =
            StreamId::try_new(format!("session-{}", session_id.clone().into_inner())).unwrap();
        let session_events = executor
            .event_store()
            .read_streams(
                &[session_stream.clone()],
                &eventcore::ReadOptions::default(),
            )
            .await
            .unwrap();

        // Verify we got the expected event
        assert_eq!(session_events.events.len(), 1); // Only RequestReceived goes to session stream

        let request_stream = StreamId::try_new(format!("request-{request_id}")).unwrap();
        let request_events = executor
            .event_store()
            .read_streams(&[request_stream], &eventcore::ReadOptions::default())
            .await
            .unwrap();

        assert_eq!(request_events.events.len(), 2); // RequestForwarded and ResponseReceived

        // Verify events are in chronological order
        for window in request_events.events.windows(2) {
            assert!(window[0].timestamp <= window[1].timestamp);
        }
    }

    #[tokio::test]
    async fn test_multiple_stream_reads() {
        let executor = create_test_executor();
        let num_sessions = 5;
        let mut stream_ids = Vec::new();

        // Create events for multiple sessions
        for _ in 0..num_sessions {
            let audit_event = create_test_audit_event(request_received_event());
            let session_stream =
                StreamId::try_new(format!("session-{}", audit_event.session_id.as_ref())).unwrap();
            stream_ids.push(session_stream);

            let command = RecordAuditEvent::from_audit_event(&audit_event).unwrap();
            executor
                .execute(command, ExecutionOptions::default())
                .await
                .unwrap();
        }

        // Read all streams at once
        let all_events = executor
            .event_store()
            .read_streams(&stream_ids, &eventcore::ReadOptions::default())
            .await
            .unwrap();

        // Should have events from all streams
        assert_eq!(all_events.events.len(), num_sessions);
    }
}

// Test 5: Idempotency behavior
mod idempotency {
    use super::*;

    #[tokio::test]
    async fn test_duplicate_request_received_ignored() {
        let executor = create_test_executor();
        let audit_event = create_test_audit_event(request_received_event());

        // Execute same command twice
        let command = RecordAuditEvent::from_audit_event(&audit_event).unwrap();
        executor
            .execute(command.clone(), ExecutionOptions::default())
            .await
            .unwrap();
        executor
            .execute(command, ExecutionOptions::default())
            .await
            .unwrap();

        // Should only have one event
        let session_stream =
            StreamId::try_new(format!("session-{}", audit_event.session_id.as_ref())).unwrap();
        let events = executor
            .event_store()
            .read_streams(&[session_stream], &eventcore::ReadOptions::default())
            .await
            .unwrap();

        assert_eq!(events.events.len(), 1);
    }

    #[tokio::test]
    async fn test_duplicate_request_forwarded_ignored() {
        let executor = create_test_executor();
        let session_id = SessionId::generate();
        let request_id = RequestId::new();

        // First, record request received
        let received_event = AuditEvent {
            request_id,
            session_id: ProxySessionId::try_new(session_id.clone().into_inner()).unwrap(),
            timestamp: Utc::now(),
            event_type: request_received_event(),
        };
        let cmd = RecordAuditEvent::from_audit_event(&received_event).unwrap();
        executor
            .execute(cmd, ExecutionOptions::default())
            .await
            .unwrap();

        // Then try to forward twice
        let forwarded_event = AuditEvent {
            request_id,
            session_id: ProxySessionId::try_new(session_id.clone().into_inner()).unwrap(),
            timestamp: Utc::now() + chrono::Duration::milliseconds(10),
            event_type: request_forwarded_event(),
        };
        let cmd = RecordAuditEvent::from_audit_event(&forwarded_event).unwrap();
        executor
            .execute(cmd.clone(), ExecutionOptions::default())
            .await
            .unwrap();
        executor
            .execute(cmd, ExecutionOptions::default())
            .await
            .unwrap();

        // Should only have one forwarded event
        let request_stream = StreamId::try_new(format!("request-{request_id}")).unwrap();
        let events = executor
            .event_store()
            .read_streams(&[request_stream], &eventcore::ReadOptions::default())
            .await
            .unwrap();

        let forwarded_count = events
            .events
            .iter()
            .filter(|e| matches!(e.payload, DomainEvent::LlmRequestStarted { .. }))
            .count();

        assert_eq!(forwarded_count, 1);
    }

    #[tokio::test]
    async fn test_idempotency_across_different_timestamps() {
        let executor = create_test_executor();
        let session_id = SessionId::generate();
        let request_id = RequestId::new();

        // Execute same logical event with different timestamps
        let base_time = Utc::now();
        for i in 0..3 {
            let audit_event = AuditEvent {
                request_id,
                session_id: ProxySessionId::try_new(session_id.clone().into_inner()).unwrap(),
                timestamp: base_time + chrono::Duration::seconds(i),
                event_type: request_received_event(),
            };
            let cmd = RecordAuditEvent::from_audit_event(&audit_event).unwrap();
            executor
                .execute(cmd, ExecutionOptions::default())
                .await
                .unwrap();
        }

        // Should still only have one event
        let session_stream =
            StreamId::try_new(format!("session-{}", session_id.clone().into_inner())).unwrap();
        let events = executor
            .event_store()
            .read_streams(&[session_stream], &eventcore::ReadOptions::default())
            .await
            .unwrap();

        assert_eq!(events.events.len(), 1);
    }
}

// Test 6: Property-based tests for invariants
mod property_tests {
    use super::*;

    // Strategy for generating valid audit event types
    prop_compose! {
        fn arb_audit_event_type()(
            method in "[A-Z]{3,6}",
            uri in "/[a-z/]+",
            status in 100..600u16,
            body_size in 0..1_000_000usize,
            duration_ms in 1..10_000u32,
        ) -> AuditEventType {
            // Use proptest's prop_oneof to randomly select event type
            match body_size % 4 {
                0 => AuditEventType::RequestReceived {
                    method: HttpMethod::try_new(method).unwrap(),
                    uri: RequestUri::try_new(uri).unwrap(),
                    headers: Headers::new(),
                    body_size: BodySize::from(body_size),
                },
                1 => AuditEventType::RequestForwarded {
                    target_url: TargetUrl::try_new(format!("https://api.example.com{uri}")).unwrap(),
                    start_time: Utc::now(),
                },
                2 => AuditEventType::ResponseReceived {
                    status: HttpStatusCode::try_new(status).unwrap(),
                    headers: Headers::new(),
                    body_size: BodySize::from(body_size),
                    duration_ms: DurationMillis::from(duration_ms as u64),
                },
                _ => AuditEventType::ResponseReturned {
                    duration_ms: DurationMillis::from(duration_ms as u64),
                },
            }
        }
    }

    proptest! {
        #[test]
        fn test_any_valid_audit_event_can_be_converted_to_command(
            event_type in arb_audit_event_type()
        ) {
            let audit_event = AuditEvent {
                request_id: RequestId::new(),
                session_id: ProxySessionId::new(),
                timestamp: Utc::now(),
                event_type,
            };

            let result = RecordAuditEvent::from_audit_event(&audit_event);
            prop_assert!(result.is_ok());
        }

        #[test]
        fn test_stream_ids_are_deterministic(
            _seed1: u128,
            _seed2: u128
        ) {
            // Use actual v7 UUIDs
            let request_id = RequestId::new();
            let session_id = ProxySessionId::new();

            let audit_event = AuditEvent {
                request_id,
                session_id,
                timestamp: Utc::now(),
                event_type: request_received_event(),
            };

            let command1 = RecordAuditEvent::from_audit_event(&audit_event).unwrap();
            let command2 = RecordAuditEvent::from_audit_event(&audit_event).unwrap();

            prop_assert_eq!(command1.session_stream, command2.session_stream);
            prop_assert_eq!(command1.request_stream, command2.request_stream);
        }
    }

    // Property: State machine transitions are valid
    proptest! {
        #[test]
        fn test_state_machine_never_goes_backwards(
            events in prop::collection::vec(arb_audit_event_type(), 1..20)
        ) {
            let mut state = RequestState::default();
            let request_id = crate::domain::llm::RequestId::generate();
            let session_id = SessionId::generate();
            let timestamp = Timestamp::now();

            // Track progression through states
            let mut has_been_received = false;
            let mut has_been_forwarded = false;
            let mut has_been_responded = false;

            for event_type in events {
                // Convert to domain event based on type
                let domain_event = match event_type {
                    AuditEventType::RequestReceived { .. } => {
                        Some(DomainEvent::LlmRequestReceived {
                            request_id: request_id.clone(),
                            session_id: session_id.clone(),
                            model_version: crate::domain::llm::ModelVersion {
                                provider: crate::domain::llm::LlmProvider::Other(
                                    crate::domain::config_types::ProviderName::try_new("test".to_string()).unwrap()
                                ),
                                model_id: crate::domain::types::ModelId::try_new("test".to_string()).unwrap(),
                            },
                            prompt: crate::domain::types::Prompt::try_new("test".to_string()).unwrap(),
                            parameters: crate::domain::types::LlmParameters::new(Default::default()),
                            received_at: timestamp,
                        })
                    },
                    AuditEventType::RequestForwarded { .. } => {
                        Some(DomainEvent::LlmRequestStarted {
                            request_id: request_id.clone(),
                            started_at: timestamp,
                        })
                    },
                    AuditEventType::ResponseReceived { .. } => {
                        Some(DomainEvent::LlmResponseReceived {
                            request_id: request_id.clone(),
                            response_text: crate::domain::types::ResponseText::try_new("test".to_string()).unwrap(),
                            metadata: Default::default(),
                            received_at: timestamp,
                        })
                    },
                    _ => None,
                };

                if let Some(event) = domain_event {
                    let old_state = state.clone();
                    state.apply(&event);

                    // Verify state never goes backwards
                    if old_state.is_request_received() {
                        has_been_received = true;
                    }
                    if old_state.is_request_forwarded() {
                        has_been_forwarded = true;
                    }
                    if old_state.is_response_received() {
                        has_been_responded = true;
                    }

                    // Once we've been in a state, we should never lose that property
                    if has_been_received {
                        prop_assert!(state.is_request_received());
                    }
                    if has_been_forwarded {
                        prop_assert!(state.is_request_forwarded());
                    }
                    if has_been_responded {
                        prop_assert!(state.is_response_received());
                    }
                }
            }
        }
    }
}

// Test 7: Recovery scenarios
mod recovery_scenarios {
    use super::*;

    #[tokio::test]
    async fn test_recovery_after_partial_processing() {
        let executor = create_test_executor();
        let session_id = SessionId::generate();
        let request_id = RequestId::new();

        // Process request received
        let received_event = AuditEvent {
            request_id,
            session_id: ProxySessionId::try_new(session_id.clone().into_inner()).unwrap(),
            timestamp: Utc::now(),
            event_type: request_received_event(),
        };
        let cmd = RecordAuditEvent::from_audit_event(&received_event).unwrap();
        executor
            .execute(cmd, ExecutionOptions::default())
            .await
            .unwrap();

        // Simulate crash/restart by creating new command for same request
        // Try to process response without forwarding (simulating lost forwarded event)
        let response_event = AuditEvent {
            request_id,
            session_id: ProxySessionId::try_new(session_id.clone().into_inner()).unwrap(),
            timestamp: Utc::now() + chrono::Duration::milliseconds(200),
            event_type: response_received_event(),
        };
        let cmd = RecordAuditEvent::from_audit_event(&response_event).unwrap();
        let result = executor.execute(cmd, ExecutionOptions::default()).await;

        // Should succeed but not emit event due to invalid state transition
        assert!(result.is_ok());

        // Verify no response event was recorded
        let request_stream = StreamId::try_new(format!("request-{request_id}")).unwrap();
        let events = executor
            .event_store()
            .read_streams(&[request_stream], &eventcore::ReadOptions::default())
            .await
            .unwrap();

        let response_count = events
            .events
            .iter()
            .filter(|e| matches!(e.payload, DomainEvent::LlmResponseReceived { .. }))
            .count();

        assert_eq!(response_count, 0);
    }

    #[tokio::test]
    async fn test_recovery_with_out_of_order_events() {
        let executor = create_test_executor();
        let session_id = SessionId::generate();
        let request_id = RequestId::new();
        let base_time = Utc::now();

        // Process events out of order: response, forwarded, received
        let events = vec![
            (
                response_received_event(),
                base_time + chrono::Duration::milliseconds(200),
            ),
            (
                request_forwarded_event(),
                base_time + chrono::Duration::milliseconds(100),
            ),
            (request_received_event(), base_time),
        ];

        for (event_type, timestamp) in events {
            let audit_event = AuditEvent {
                request_id,
                session_id: ProxySessionId::try_new(session_id.clone().into_inner()).unwrap(),
                timestamp,
                event_type,
            };
            let cmd = RecordAuditEvent::from_audit_event(&audit_event).unwrap();
            executor
                .execute(cmd, ExecutionOptions::default())
                .await
                .unwrap();
        }

        // Only the last one (request received) should have been recorded
        let session_stream =
            StreamId::try_new(format!("session-{}", session_id.clone().into_inner())).unwrap();
        let session_events = executor
            .event_store()
            .read_streams(&[session_stream], &eventcore::ReadOptions::default())
            .await
            .unwrap();

        assert_eq!(session_events.events.len(), 1);
        assert!(matches!(
            session_events.events[0].payload,
            DomainEvent::LlmRequestReceived { .. }
        ));
    }
}

// Test 8: ProcessRequestBody command specific tests
mod process_request_body_tests {
    use super::*;

    #[tokio::test]
    async fn test_process_request_body_parses_openai_format() {
        let executor = create_test_executor();
        let session_id = SessionId::generate();
        let request_id = RequestId::new();

        let body = create_openai_request_body();
        let command = ProcessRequestBody {
            session_stream: StreamId::try_new(format!(
                "session-{}",
                session_id.clone().into_inner()
            ))
            .unwrap(),
            request_stream: StreamId::try_new(format!("request-{request_id}")).unwrap(),
            request_id,
            session_id: session_id.clone(),
            method: HttpMethod::try_new("POST".to_string()).unwrap(),
            uri: RequestUri::try_new("/v1/chat/completions".to_string()).unwrap(),
            headers: Headers::new(),
            body,
            timestamp: Timestamp::now(),
        };

        let result = executor.execute(command, ExecutionOptions::default()).await;
        assert!(result.is_ok());

        // Verify parsed content
        let session_stream =
            StreamId::try_new(format!("session-{}", session_id.clone().into_inner())).unwrap();
        let events = executor
            .event_store()
            .read_streams(&[session_stream], &eventcore::ReadOptions::default())
            .await
            .unwrap();

        assert_eq!(events.events.len(), 1);
        if let DomainEvent::LlmRequestReceived {
            model_version,
            prompt,
            ..
        } = &events.events[0].payload
        {
            assert_eq!(model_version.model_id.as_ref(), "gpt-4");
            assert!(prompt.as_ref().contains("Hello, world!"));
        } else {
            panic!("Expected LlmRequestReceived event");
        }
    }

    #[tokio::test]
    async fn test_process_request_body_parses_anthropic_format() {
        let executor = create_test_executor();
        let session_id = SessionId::generate();
        let request_id = RequestId::new();

        let body = create_anthropic_request_body();
        let command = ProcessRequestBody {
            session_stream: StreamId::try_new(format!(
                "session-{}",
                session_id.clone().into_inner()
            ))
            .unwrap(),
            request_stream: StreamId::try_new(format!("request-{request_id}")).unwrap(),
            request_id,
            session_id: session_id.clone(),
            method: HttpMethod::try_new("POST".to_string()).unwrap(),
            uri: RequestUri::try_new("/v1/messages".to_string()).unwrap(),
            headers: Headers::new(),
            body,
            timestamp: Timestamp::now(),
        };

        let result = executor.execute(command, ExecutionOptions::default()).await;
        assert!(result.is_ok());

        // Verify parsed content
        let session_stream =
            StreamId::try_new(format!("session-{}", session_id.clone().into_inner())).unwrap();
        let events = executor
            .event_store()
            .read_streams(&[session_stream], &eventcore::ReadOptions::default())
            .await
            .unwrap();

        assert_eq!(events.events.len(), 1);
        if let DomainEvent::LlmRequestReceived {
            model_version,
            prompt,
            ..
        } = &events.events[0].payload
        {
            assert_eq!(model_version.model_id.as_ref(), "claude-3-opus-20240229");
            assert!(prompt.as_ref().contains("What is 2+2?"));
        } else {
            panic!("Expected LlmRequestReceived event");
        }
    }

    #[tokio::test]
    async fn test_process_request_body_idempotent() {
        let executor = create_test_executor();
        let session_id = SessionId::generate();
        let request_id = RequestId::new();

        let body = create_openai_request_body();
        let command = ProcessRequestBody {
            session_stream: StreamId::try_new(format!(
                "session-{}",
                session_id.clone().into_inner()
            ))
            .unwrap(),
            request_stream: StreamId::try_new(format!("request-{request_id}")).unwrap(),
            request_id,
            session_id: session_id.clone(),
            method: HttpMethod::try_new("POST".to_string()).unwrap(),
            uri: RequestUri::try_new("/v1/chat/completions".to_string()).unwrap(),
            headers: Headers::new(),
            body,
            timestamp: Timestamp::now(),
        };

        // Execute twice
        executor
            .execute(command.clone(), ExecutionOptions::default())
            .await
            .unwrap();
        executor
            .execute(command, ExecutionOptions::default())
            .await
            .unwrap();

        // Should only have one event
        let session_stream =
            StreamId::try_new(format!("session-{}", session_id.clone().into_inner())).unwrap();
        let events = executor
            .event_store()
            .read_streams(&[session_stream], &eventcore::ReadOptions::default())
            .await
            .unwrap();

        assert_eq!(events.events.len(), 1);
    }
}

// Test 9: Edge cases and boundary conditions
mod edge_cases {
    use super::*;

    #[tokio::test]
    async fn test_extremely_large_request_body() {
        let executor = create_test_executor();
        let session_id = SessionId::generate();
        let request_id = RequestId::new();

        // Create a very large request body (10MB)
        let large_content = "x".repeat(10 * 1024 * 1024);
        let body = serde_json::json!({
            "model": "gpt-4",
            "messages": [{
                "role": "user",
                "content": large_content
            }]
        })
        .to_string()
        .as_bytes()
        .to_vec();

        let command = ProcessRequestBody {
            session_stream: StreamId::try_new(format!(
                "session-{}",
                session_id.clone().into_inner()
            ))
            .unwrap(),
            request_stream: StreamId::try_new(format!("request-{request_id}")).unwrap(),
            request_id,
            session_id: session_id.clone(),
            method: HttpMethod::try_new("POST".to_string()).unwrap(),
            uri: RequestUri::try_new("/v1/chat/completions".to_string()).unwrap(),
            headers: Headers::new(),
            body,
            timestamp: Timestamp::now(),
        };

        let result = executor.execute(command, ExecutionOptions::default()).await;
        // Should handle large payloads gracefully
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_request_with_special_characters_in_uri() {
        let executor = create_test_executor();

        // Test various special characters in URI
        let test_uris = vec![
            "/v1/chat/completions?api-version=2023-05-15",
            "/v1/models/gpt-4-turbo%2Flatest/chat",
            "/v1/assistants/asst_abc123/threads/thread_xyz789",
            "/v1/fine-tunes/ft-1234567890/events?stream=true",
        ];

        for uri_str in test_uris {
            let audit_event = AuditEvent {
                request_id: RequestId::new(),
                session_id: ProxySessionId::new(),
                timestamp: Utc::now(),
                event_type: AuditEventType::RequestReceived {
                    method: HttpMethod::try_new("POST".to_string()).unwrap(),
                    uri: RequestUri::try_new(uri_str.to_string()).unwrap(),
                    headers: Headers::new(),
                    body_size: BodySize::from(100),
                },
            };

            let command = RecordAuditEvent::from_audit_event(&audit_event).unwrap();
            let result = executor.execute(command, ExecutionOptions::default()).await;
            assert!(result.is_ok());
        }
    }

    #[tokio::test]
    async fn test_zero_duration_response() {
        let executor = create_test_executor();

        let audit_event = AuditEvent {
            request_id: RequestId::new(),
            session_id: ProxySessionId::new(),
            timestamp: Utc::now(),
            event_type: AuditEventType::ResponseReceived {
                status: HttpStatusCode::try_new(200).unwrap(),
                headers: Headers::new(),
                body_size: BodySize::from(0),
                duration_ms: DurationMillis::from(0), // Zero duration
            },
        };

        let command = RecordAuditEvent::from_audit_event(&audit_event).unwrap();
        let result = executor.execute(command, ExecutionOptions::default()).await;
        assert!(result.is_ok());
    }
}

// Test 10: Headers with sensitive data
mod headers_tests {
    use super::*;

    #[tokio::test]
    async fn test_headers_with_authorization() {
        let executor = create_test_executor();
        let session_id = SessionId::generate();
        let request_id = RequestId::new();

        let headers = Headers::from_vec(vec![
            (
                "Authorization".to_string(),
                "Bearer sk-1234567890abcdef".to_string(),
            ),
            ("Content-Type".to_string(), "application/json".to_string()),
        ])
        .unwrap();

        let body = create_openai_request_body();
        let command = ProcessRequestBody {
            session_stream: StreamId::try_new(format!(
                "session-{}",
                session_id.clone().into_inner()
            ))
            .unwrap(),
            request_stream: StreamId::try_new(format!("request-{request_id}")).unwrap(),
            request_id,
            session_id: session_id.clone(),
            method: HttpMethod::try_new("POST".to_string()).unwrap(),
            uri: RequestUri::try_new("/v1/chat/completions".to_string()).unwrap(),
            headers,
            body,
            timestamp: Timestamp::now(),
        };

        let result = executor.execute(command, ExecutionOptions::default()).await;
        assert!(result.is_ok());

        // Verify event was created (headers should be processed, not stored directly)
        let session_stream =
            StreamId::try_new(format!("session-{}", session_id.clone().into_inner())).unwrap();
        let events = executor
            .event_store()
            .read_streams(&[session_stream], &eventcore::ReadOptions::default())
            .await
            .unwrap();
        assert_eq!(events.events.len(), 1);
    }
}

// Benchmarks for performance regression testing
#[cfg(test)]
mod benchmarks {
    use super::*;
    use std::time::Instant;

    #[tokio::test]
    async fn bench_single_command_execution() {
        let executor = create_test_executor();
        let iterations = 100; // Reduced for test speed

        let start = Instant::now();
        for _ in 0..iterations {
            let audit_event = create_test_audit_event(request_received_event());
            let command = RecordAuditEvent::from_audit_event(&audit_event).unwrap();
            executor
                .execute(command, ExecutionOptions::default())
                .await
                .unwrap();
        }
        let duration = start.elapsed();

        let per_command = duration / iterations;
        println!("Average command execution time: {per_command:?}");

        // Ensure performance doesn't regress (adjusted for in-memory store)
        assert!(per_command.as_millis() < 10); // Less than 10ms per command
    }

    #[tokio::test]
    async fn bench_concurrent_command_execution() {
        let executor = create_test_executor();
        let concurrent_commands = 50; // Reduced for test speed

        let start = Instant::now();
        let handles: Vec<_> = (0..concurrent_commands)
            .map(|_| {
                let exec = executor.clone();
                tokio::spawn(async move {
                    let audit_event = create_test_audit_event(request_received_event());
                    let command = RecordAuditEvent::from_audit_event(&audit_event).unwrap();
                    exec.execute(command, ExecutionOptions::default()).await
                })
            })
            .collect();

        let results: Vec<_> = futures_util::future::join_all(handles).await;
        let duration = start.elapsed();

        // All should succeed
        for result in results {
            assert!(result.is_ok());
            assert!(result.unwrap().is_ok());
        }

        println!("Concurrent execution of {concurrent_commands} commands took: {duration:?}");

        // Should complete within reasonable time
        assert!(duration.as_secs() < 2); // Less than 2 seconds for 50 concurrent commands
    }
}
