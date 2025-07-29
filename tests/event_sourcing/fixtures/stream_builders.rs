//! Stream builders for creating complex event sequences
//!
//! These builders enable creation of event streams that represent
//! various system states and scenarios for comprehensive testing.

use chrono::Duration;
use std::collections::HashMap;
use union_square::domain::{
    events::DomainEvent,
    llm::ModelVersion,
    session::{ApplicationId, SessionId, SessionStatus},
    user::UserId,
    version::VersionComparison,
};

use super::event_builders::{
    LlmEventBuilder, MetricsEventBuilder, MockClock, SessionEventBuilder, VersionEventBuilder,
};

/// Builder for creating complete event streams
#[derive(Debug, Clone)]
pub struct EventStreamBuilder {
    events: Vec<DomainEvent>,
    clock: MockClock,
    session_id: SessionId,
    user_id: UserId,
    application_id: ApplicationId,
}

impl EventStreamBuilder {
    /// Create a new event stream builder
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            clock: MockClock::new(),
            session_id: SessionId::generate(),
            user_id: UserId::generate(),
            application_id: ApplicationId::try_new("test-app".to_string()).unwrap(),
        }
    }

    /// Set the session ID for all events
    pub fn with_session_id(mut self, session_id: SessionId) -> Self {
        self.session_id = session_id;
        self
    }

    /// Set the user ID for all events
    pub fn with_user_id(mut self, user_id: UserId) -> Self {
        self.user_id = user_id;
        self
    }

    /// Set the application ID
    pub fn with_application_id(mut self, app_id: ApplicationId) -> Self {
        self.application_id = app_id;
        self
    }

    /// Set the starting time for the event stream
    pub fn starting_at(mut self, time: chrono::DateTime<chrono::Utc>) -> Self {
        self.clock = MockClock::at(time);
        self
    }

    /// Add a single event to the stream
    pub fn add_event(mut self, event: DomainEvent) -> Self {
        self.events.push(event);
        self
    }

    /// Add multiple events to the stream
    pub fn add_events(mut self, events: Vec<DomainEvent>) -> Self {
        self.events.extend(events);
        self
    }

    /// Add a complete session lifecycle
    pub fn with_session_lifecycle(mut self) -> Self {
        let mut builder = SessionEventBuilder::new()
            .with_session_id(self.session_id.clone())
            .with_user_id(self.user_id.clone())
            .with_application_id(self.application_id.clone())
            .with_clock(self.clock.clone());

        let events = builder.complete_lifecycle();
        self.events.extend(events);
        
        // Update our clock to match the builder's clock
        self.clock = builder.clock;
        self
    }

    /// Add multiple LLM request/response cycles
    pub fn with_llm_requests(mut self, count: usize) -> Self {
        for i in 0..count {
            let mut builder = LlmEventBuilder::new(self.session_id.clone())
                .with_clock(self.clock.clone());

            let prompt = format!("Test prompt {}", i + 1);
            let response = format!("Test response {}", i + 1);
            
            let events = builder.successful_request_cycle(&prompt, &response);
            self.events.extend(events);
            
            // Update clock and add delay between requests
            self.clock = builder.clock;
            self.clock.advance_by(Duration::seconds(30));
        }
        self
    }

    /// Add a failed LLM request
    pub fn with_failed_llm_request(mut self, error_message: &str) -> Self {
        let mut builder = LlmEventBuilder::new(self.session_id.clone())
            .with_clock(self.clock.clone());

        let events = builder.failed_request_cycle("Test prompt", error_message);
        self.events.extend(events);
        
        self.clock = builder.clock;
        self
    }

    /// Add version tracking events
    pub fn with_version_change(
        mut self,
        from_version: &str,
        to_version: &str,
        change_type: VersionComparison,
    ) -> Self {
        let mut builder = VersionEventBuilder::new(self.session_id.clone())
            .with_clock(self.clock.clone());

        let events = vec![
            builder.version_first_seen(from_version),
            builder.version_changed(from_version, to_version, change_type),
        ];
        
        self.events.extend(events);
        self.clock = builder.clock;
        self
    }

    /// Add F-score metrics
    pub fn with_metrics(mut self, model: &str, f_score: f64) -> Self {
        let builder = MetricsEventBuilder::new(self.session_id.clone())
            .with_clock(self.clock.clone());

        let event = builder.f_score_calculated(model, f_score, Some(f_score - 0.05), Some(f_score + 0.05), 100);
        self.events.push(event);
        self
    }

    /// Build the event stream
    pub fn build(self) -> Vec<DomainEvent> {
        self.events
    }

    /// Build and return both the events and the final clock state
    pub fn build_with_clock(self) -> (Vec<DomainEvent>, MockClock) {
        (self.events, self.clock)
    }
}

/// Predefined scenario builders for common test cases
pub struct ScenarioBuilder;

impl ScenarioBuilder {
    /// Create a typical user session with multiple LLM requests
    pub fn typical_user_session() -> Vec<DomainEvent> {
        EventStreamBuilder::new()
            .with_session_lifecycle()
            .with_llm_requests(5)
            .with_metrics("gpt-4-turbo-2024-01", 0.85)
            .build()
    }

    /// Create a session with a version change mid-session
    pub fn session_with_version_change() -> Vec<DomainEvent> {
        EventStreamBuilder::new()
            .with_session_lifecycle()
            .with_llm_requests(2)
            .with_version_change(
                "gpt-4-0613",
                "gpt-4-turbo-2024-01",
                VersionComparison::MinorUpgrade,
            )
            .with_llm_requests(3)
            .build()
    }

    /// Create a session with mixed success and failure
    pub fn session_with_failures() -> Vec<DomainEvent> {
        EventStreamBuilder::new()
            .with_session_lifecycle()
            .with_llm_requests(2)
            .with_failed_llm_request("Rate limit exceeded")
            .with_llm_requests(1)
            .with_failed_llm_request("Service temporarily unavailable")
            .build()
    }

    /// Create multiple concurrent sessions
    pub fn concurrent_sessions(session_count: usize) -> HashMap<SessionId, Vec<DomainEvent>> {
        let mut sessions = HashMap::new();
        
        for i in 0..session_count {
            let session_id = SessionId::generate();
            let user_id = UserId::generate();
            
            let events = EventStreamBuilder::new()
                .with_session_id(session_id.clone())
                .with_user_id(user_id)
                .with_session_lifecycle()
                .with_llm_requests(3 + i % 3) // Vary request count
                .build();
                
            sessions.insert(session_id, events);
        }
        
        sessions
    }

    /// Create a long-running session with multiple model versions
    pub fn long_running_session_with_model_evolution() -> Vec<DomainEvent> {
        let mut builder = EventStreamBuilder::new();
        
        // Start session
        builder = builder.with_session_lifecycle();
        
        // Initial requests with GPT-3.5
        builder = builder
            .with_llm_requests(5)
            .with_metrics("gpt-3.5-turbo", 0.72);
        
        // Upgrade to GPT-4
        builder = builder
            .with_version_change(
                "gpt-3.5-turbo",
                "gpt-4-0613",
                VersionComparison::MajorUpgrade,
            )
            .with_llm_requests(10)
            .with_metrics("gpt-4-0613", 0.85);
        
        // Upgrade to GPT-4 Turbo
        builder = builder
            .with_version_change(
                "gpt-4-0613",
                "gpt-4-turbo-2024-01",
                VersionComparison::MinorUpgrade,
            )
            .with_llm_requests(15)
            .with_metrics("gpt-4-turbo-2024-01", 0.91);
        
        builder.build()
    }

    /// Create an event stream for testing replay scenarios
    pub fn replay_test_scenario() -> Vec<DomainEvent> {
        let session_id = SessionId::generate();
        let mut events = Vec::new();
        
        // Session events
        let mut session_builder = SessionEventBuilder::new()
            .with_session_id(session_id.clone());
        events.push(session_builder.session_started());
        
        // Interleave different event types with specific timing
        let mut llm_builder = LlmEventBuilder::new(session_id.clone())
            .with_clock(session_builder.clock.clone());
        
        // First request
        events.extend(llm_builder.successful_request_cycle("Query 1", "Response 1"));
        
        // Tag the session
        session_builder.clock = llm_builder.clock.clone();
        events.push(session_builder.session_tagged("important"));
        
        // More requests
        llm_builder.clock = session_builder.clock.clone();
        events.extend(llm_builder.successful_request_cycle("Query 2", "Response 2"));
        
        // Add metrics
        let metrics_builder = MetricsEventBuilder::new(session_id.clone())
            .with_clock(llm_builder.clock.clone());
        events.push(metrics_builder.f_score_calculated("gpt-4", 0.88, Some(0.85), Some(0.91), 50));
        
        // Final request and session end
        llm_builder.clock.advance_by(Duration::minutes(5));
        events.extend(llm_builder.successful_request_cycle("Query 3", "Response 3"));
        
        session_builder.clock = llm_builder.clock.clone();
        events.push(session_builder.session_ended(SessionStatus::Completed));
        
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_stream_builder_basic() {
        let events = EventStreamBuilder::new()
            .with_session_lifecycle()
            .build();
        
        assert_eq!(events.len(), 3); // start, tag, end
    }

    #[test]
    fn test_event_stream_builder_with_requests() {
        let events = EventStreamBuilder::new()
            .with_session_lifecycle()
            .with_llm_requests(3)
            .build();
        
        // 3 session events + (3 events per request * 3 requests) = 12
        assert_eq!(events.len(), 12);
    }

    #[test]
    fn test_scenario_typical_session() {
        let events = ScenarioBuilder::typical_user_session();
        
        // Verify we have a complete session
        assert!(matches!(events.first(), Some(DomainEvent::SessionStarted { .. })));
        assert!(matches!(events.last(), Some(DomainEvent::SessionEnded { .. })));
        
        // Count LLM requests
        let request_count = events.iter()
            .filter(|e| matches!(e, DomainEvent::LlmRequestReceived { .. }))
            .count();
        assert_eq!(request_count, 5);
    }

    #[test]
    fn test_concurrent_sessions_scenario() {
        let sessions = ScenarioBuilder::concurrent_sessions(3);
        
        assert_eq!(sessions.len(), 3);
        
        // Each session should be complete
        for (_, events) in sessions {
            assert!(matches!(events.first(), Some(DomainEvent::SessionStarted { .. })));
            assert!(matches!(events.last(), Some(DomainEvent::SessionEnded { .. })));
        }
    }

    #[test]
    fn test_events_have_increasing_timestamps() {
        let events = ScenarioBuilder::typical_user_session();
        
        let mut last_timestamp = None;
        for event in events {
            let timestamp = event.occurred_at();
            if let Some(last) = last_timestamp {
                assert!(timestamp >= last, "Events should have non-decreasing timestamps");
            }
            last_timestamp = Some(timestamp);
        }
    }
}