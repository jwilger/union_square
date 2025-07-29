//! Event ordering and temporal constraint tests
//!
//! Tests for verifying event ordering constraints and temporal logic
//! in event-sourced systems.

use chrono::{DateTime, Duration, Utc};
use union_square::domain::{
    events::DomainEvent,
    llm::RequestId,
    metrics::Timestamp,
    session::{SessionId, SessionStatus},
};

/// Validates that events maintain proper temporal ordering
pub struct EventOrderingValidator;

impl EventOrderingValidator {
    /// Validate that events in a stream are properly ordered by timestamp
    pub fn validate_temporal_ordering(events: &[DomainEvent]) -> ValidationResult {
        let mut errors = Vec::new();
        let mut last_timestamp: Option<Timestamp> = None;

        for (index, event) in events.iter().enumerate() {
            let timestamp = event.occurred_at();
            
            if let Some(last) = last_timestamp {
                if timestamp < last {
                    errors.push(OrderingError::TimestampRegression {
                        event_index: index,
                        current_timestamp: timestamp,
                        previous_timestamp: last,
                    });
                }
            }
            
            last_timestamp = Some(timestamp);
        }

        ValidationResult { errors }
    }

    /// Validate session lifecycle ordering constraints
    pub fn validate_session_lifecycle(events: &[DomainEvent]) -> ValidationResult {
        let mut errors = Vec::new();
        let mut session_states = std::collections::HashMap::new();

        for (index, event) in events.iter().enumerate() {
            match event {
                DomainEvent::SessionStarted { session_id, .. } => {
                    if session_states.contains_key(session_id) {
                        errors.push(OrderingError::DuplicateSessionStart {
                            event_index: index,
                            session_id: session_id.clone(),
                        });
                    } else {
                        session_states.insert(session_id.clone(), SessionLifecycleState::Started);
                    }
                }
                DomainEvent::SessionEnded { session_id, .. } => {
                    match session_states.get(session_id) {
                        Some(SessionLifecycleState::Started) => {
                            session_states.insert(session_id.clone(), SessionLifecycleState::Ended);
                        }
                        Some(SessionLifecycleState::Ended) => {
                            errors.push(OrderingError::DuplicateSessionEnd {
                                event_index: index,
                                session_id: session_id.clone(),
                            });
                        }
                        None => {
                            errors.push(OrderingError::SessionEndWithoutStart {
                                event_index: index,
                                session_id: session_id.clone(),
                            });
                        }
                    }
                }
                DomainEvent::SessionTagged { session_id, .. } => {
                    if !session_states.contains_key(session_id) {
                        errors.push(OrderingError::SessionEventWithoutStart {
                            event_index: index,
                            session_id: session_id.clone(),
                            event_type: "SessionTagged".to_string(),
                        });
                    }
                }
                _ => {} // Other events don't affect session lifecycle
            }
        }

        ValidationResult { errors }
    }

    /// Validate request lifecycle ordering constraints
    pub fn validate_request_lifecycle(events: &[DomainEvent]) -> ValidationResult {
        let mut errors = Vec::new();
        let mut request_states = std::collections::HashMap::new();
        let mut session_states = std::collections::HashSet::new();

        // First pass: collect active sessions
        for event in events {
            match event {
                DomainEvent::SessionStarted { session_id, .. } => {
                    session_states.insert(session_id.clone());
                }
                DomainEvent::SessionEnded { session_id, .. } => {
                    session_states.remove(session_id);
                }
                _ => {}
            }
        }

        // Second pass: validate request ordering
        session_states.clear(); // Reset for proper tracking
        
        for (index, event) in events.iter().enumerate() {
            match event {
                DomainEvent::SessionStarted { session_id, .. } => {
                    session_states.insert(session_id.clone());
                }
                DomainEvent::SessionEnded { session_id, .. } => {
                    session_states.remove(session_id);
                }
                DomainEvent::LlmRequestReceived {
                    request_id,
                    session_id,
                    ..
                } => {
                    if !session_states.contains(session_id) {
                        errors.push(OrderingError::RequestInInactiveSession {
                            event_index: index,
                            request_id: request_id.clone(),
                            session_id: session_id.clone(),
                        });
                    }
                    
                    if request_states.contains_key(request_id) {
                        errors.push(OrderingError::DuplicateRequestReceived {
                            event_index: index,
                            request_id: request_id.clone(),
                        });
                    } else {
                        request_states.insert(request_id.clone(), RequestLifecycleState::Received);
                    }
                }
                DomainEvent::LlmRequestStarted { request_id, .. } => {
                    match request_states.get(request_id) {
                        Some(RequestLifecycleState::Received) => {
                            request_states.insert(request_id.clone(), RequestLifecycleState::Started);
                        }
                        Some(state) => {
                            errors.push(OrderingError::InvalidRequestTransition {
                                event_index: index,
                                request_id: request_id.clone(),
                                from_state: format!("{:?}", state),
                                to_state: "Started".to_string(),
                            });
                        }
                        None => {
                            errors.push(OrderingError::RequestEventWithoutReceived {
                                event_index: index,
                                request_id: request_id.clone(),
                                event_type: "LlmRequestStarted".to_string(),
                            });
                        }
                    }
                }
                DomainEvent::LlmResponseReceived { request_id, .. } => {
                    match request_states.get(request_id) {
                        Some(RequestLifecycleState::Started) => {
                            request_states.insert(request_id.clone(), RequestLifecycleState::Completed);
                        }
                        Some(state) => {
                            errors.push(OrderingError::InvalidRequestTransition {
                                event_index: index,
                                request_id: request_id.clone(),
                                from_state: format!("{:?}", state),
                                to_state: "Completed".to_string(),
                            });
                        }
                        None => {
                            errors.push(OrderingError::RequestEventWithoutReceived {
                                event_index: index,
                                request_id: request_id.clone(),
                                event_type: "LlmResponseReceived".to_string(),
                            });
                        }
                    }
                }
                DomainEvent::LlmRequestFailed { request_id, .. } => {
                    match request_states.get(request_id) {
                        Some(RequestLifecycleState::Received) | Some(RequestLifecycleState::Started) => {
                            request_states.insert(request_id.clone(), RequestLifecycleState::Failed);
                        }
                        Some(state) => {
                            errors.push(OrderingError::InvalidRequestTransition {
                                event_index: index,
                                request_id: request_id.clone(),
                                from_state: format!("{:?}", state),
                                to_state: "Failed".to_string(),
                            });
                        }
                        None => {
                            errors.push(OrderingError::RequestEventWithoutReceived {
                                event_index: index,
                                request_id: request_id.clone(),
                                event_type: "LlmRequestFailed".to_string(),
                            });
                        }
                    }
                }
                _ => {} // Other events don't affect request lifecycle
            }
        }

        ValidationResult { errors }
    }

    /// Validate that causally related events maintain proper ordering
    pub fn validate_causal_ordering(events: &[DomainEvent]) -> ValidationResult {
        let mut errors = Vec::new();
        let mut event_index_map = std::collections::HashMap::new();

        // Build index of when each event type occurred for each entity
        for (index, event) in events.iter().enumerate() {
            match event {
                DomainEvent::SessionStarted { session_id, .. } => {
                    event_index_map.insert(
                        format!("session_start_{}", session_id),
                        index,
                    );
                }
                DomainEvent::LlmRequestReceived { request_id, session_id, .. } => {
                    // Check if session was started before this request
                    if let Some(session_start_index) = event_index_map.get(&format!("session_start_{}", session_id)) {
                        if *session_start_index >= index {
                            errors.push(OrderingError::CausalOrderViolation {
                                event_index: index,
                                description: format!(
                                    "Request received before session {} was started",
                                    session_id
                                ),
                            });
                        }
                    }
                    
                    event_index_map.insert(
                        format!("request_received_{}", request_id),
                        index,
                    );
                }
                DomainEvent::LlmRequestStarted { request_id, .. } => {
                    if let Some(received_index) = event_index_map.get(&format!("request_received_{}", request_id)) {
                        if *received_index >= index {
                            errors.push(OrderingError::CausalOrderViolation {
                                event_index: index,
                                description: format!(
                                    "Request {} started before it was received",
                                    request_id
                                ),
                            });
                        }
                    }
                }
                _ => {}
            }
        }

        ValidationResult { errors }
    }
}

#[derive(Debug, Clone)]
pub enum SessionLifecycleState {
    Started,
    Ended,
}

#[derive(Debug, Clone)]
pub enum RequestLifecycleState {
    Received,
    Started,
    Completed,
    Failed,
}

#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub errors: Vec<OrderingError>,
}

impl ValidationResult {
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn error_count(&self) -> usize {
        self.errors.len()
    }
}

#[derive(Debug, Clone)]
pub enum OrderingError {
    TimestampRegression {
        event_index: usize,
        current_timestamp: Timestamp,
        previous_timestamp: Timestamp,
    },
    DuplicateSessionStart {
        event_index: usize,
        session_id: SessionId,
    },
    DuplicateSessionEnd {
        event_index: usize,
        session_id: SessionId,
    },
    SessionEndWithoutStart {
        event_index: usize,
        session_id: SessionId,
    },
    SessionEventWithoutStart {
        event_index: usize,
        session_id: SessionId,
        event_type: String,
    },
    RequestInInactiveSession {
        event_index: usize,
        request_id: RequestId,
        session_id: SessionId,
    },
    DuplicateRequestReceived {
        event_index: usize,
        request_id: RequestId,
    },
    InvalidRequestTransition {
        event_index: usize,
        request_id: RequestId,
        from_state: String,
        to_state: String,
    },
    RequestEventWithoutReceived {
        event_index: usize,
        request_id: RequestId,
        event_type: String,
    },
    CausalOrderViolation {
        event_index: usize,
        description: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::event_sourcing::fixtures::{
        event_builders::{LlmEventBuilder, SessionEventBuilder},
        stream_builders::{EventStreamBuilder, ScenarioBuilder},
    };

    #[test]
    fn test_valid_temporal_ordering() {
        let events = ScenarioBuilder::typical_user_session();
        let result = EventOrderingValidator::validate_temporal_ordering(&events);
        
        assert!(result.is_valid(), "Events should be temporally ordered");
    }

    #[test]
    fn test_invalid_temporal_ordering() {
        let mut events = ScenarioBuilder::typical_user_session();
        
        // Artificially break temporal ordering by swapping two events
        if events.len() >= 2 {
            events.swap(0, events.len() - 1);
        }
        
        let result = EventOrderingValidator::validate_temporal_ordering(&events);
        
        assert!(!result.is_valid(), "Should detect temporal ordering violation");
        assert!(matches!(
            result.errors.first(),
            Some(OrderingError::TimestampRegression { .. })
        ));
    }

    #[test]
    fn test_valid_session_lifecycle() {
        let events = EventStreamBuilder::new()
            .with_session_lifecycle()
            .build();
            
        let result = EventOrderingValidator::validate_session_lifecycle(&events);
        
        assert!(result.is_valid(), "Session lifecycle should be valid");
    }

    #[test]
    fn test_invalid_session_lifecycle_duplicate_start() {
        let session_id = SessionId::generate();
        let mut builder = SessionEventBuilder::new()
            .with_session_id(session_id.clone());
            
        let events = vec![
            builder.session_started(),
            builder.session_started(), // Duplicate start
        ];
        
        let result = EventOrderingValidator::validate_session_lifecycle(&events);
        
        assert!(!result.is_valid(), "Should detect duplicate session start");
        assert!(matches!(
            result.errors.first(),
            Some(OrderingError::DuplicateSessionStart { .. })
        ));
    }

    #[test]
    fn test_invalid_session_lifecycle_end_without_start() {
        let session_id = SessionId::generate();
        let mut builder = SessionEventBuilder::new()
            .with_session_id(session_id.clone());
            
        let events = vec![
            builder.session_ended(SessionStatus::Completed), // End without start
        ];
        
        let result = EventOrderingValidator::validate_session_lifecycle(&events);
        
        assert!(!result.is_valid(), "Should detect session end without start");
        assert!(matches!(
            result.errors.first(),
            Some(OrderingError::SessionEndWithoutStart { .. })
        ));
    }

    #[test]
    fn test_valid_request_lifecycle() {
        let events = ScenarioBuilder::typical_user_session();
        let result = EventOrderingValidator::validate_request_lifecycle(&events);
        
        assert!(result.is_valid(), "Request lifecycle should be valid");
    }

    #[test]
    fn test_invalid_request_lifecycle_without_session() {
        let session_id = SessionId::generate();
        let mut builder = LlmEventBuilder::new(session_id);
        
        let events = builder.successful_request_cycle("test", "response");
        
        let result = EventOrderingValidator::validate_request_lifecycle(&events);
        
        assert!(!result.is_valid(), "Should detect request without active session");
        assert!(matches!(
            result.errors.first(),
            Some(OrderingError::RequestInInactiveSession { .. })
        ));
    }

    #[test]
    fn test_causal_ordering_validation() {
        let events = ScenarioBuilder::typical_user_session();
        let result = EventOrderingValidator::validate_causal_ordering(&events);
        
        assert!(result.is_valid(), "Causal ordering should be valid");
    }

    #[test]
    fn test_comprehensive_validation() {
        let events = ScenarioBuilder::session_with_failures();
        
        // Run all validations
        let temporal_result = EventOrderingValidator::validate_temporal_ordering(&events);
        let session_result = EventOrderingValidator::validate_session_lifecycle(&events);
        let request_result = EventOrderingValidator::validate_request_lifecycle(&events);
        let causal_result = EventOrderingValidator::validate_causal_ordering(&events);
        
        assert!(temporal_result.is_valid(), "Temporal ordering should be valid");
        assert!(session_result.is_valid(), "Session lifecycle should be valid");
        assert!(request_result.is_valid(), "Request lifecycle should be valid");
        assert!(causal_result.is_valid(), "Causal ordering should be valid");
    }
}