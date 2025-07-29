//! Event builders for creating test fixtures
//!
//! These builders make it easy to create domain events with sensible defaults
//! while allowing customization for specific test scenarios.

use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use union_square::domain::{
    events::DomainEvent,
    llm::{ModelVersion, RequestId, ResponseMetadata},
    metrics::{FScore, Precision, Recall, SampleCount, Timestamp},
    session::{ApplicationId, SessionId, SessionStatus},
    types::{ChangeReason, ErrorMessage, LlmParameters, Prompt, ResponseText, Tag},
    user::{DisplayName, EmailAddress, UserId},
    version::{VersionChangeId, VersionComparison},
};

/// Builder for creating session lifecycle events
#[derive(Debug, Clone)]
pub struct SessionEventBuilder {
    session_id: SessionId,
    user_id: UserId,
    application_id: ApplicationId,
    clock: MockClock,
}

impl SessionEventBuilder {
    /// Create a new session event builder
    pub fn new() -> Self {
        Self {
            session_id: SessionId::generate(),
            user_id: UserId::generate(),
            application_id: ApplicationId::try_new("test-app".to_string()).unwrap(),
            clock: MockClock::new(),
        }
    }

    /// Set the session ID
    pub fn with_session_id(mut self, session_id: SessionId) -> Self {
        self.session_id = session_id;
        self
    }

    /// Set the user ID
    pub fn with_user_id(mut self, user_id: UserId) -> Self {
        self.user_id = user_id;
        self
    }

    /// Set the application ID
    pub fn with_application_id(mut self, app_id: ApplicationId) -> Self {
        self.application_id = app_id;
        self
    }

    /// Set the clock for timestamp generation
    pub fn with_clock(mut self, clock: MockClock) -> Self {
        self.clock = clock;
        self
    }

    /// Build a SessionStarted event
    pub fn session_started(&self) -> DomainEvent {
        DomainEvent::SessionStarted {
            session_id: self.session_id.clone(),
            user_id: self.user_id.clone(),
            application_id: self.application_id.clone(),
            started_at: Timestamp::from(self.clock.now()),
        }
    }

    /// Build a SessionEnded event
    pub fn session_ended(&mut self, status: SessionStatus) -> DomainEvent {
        DomainEvent::SessionEnded {
            session_id: self.session_id.clone(),
            ended_at: Timestamp::from(self.clock.advance_by(Duration::minutes(30))),
            final_status: status,
        }
    }

    /// Build a SessionTagged event
    pub fn session_tagged(&mut self, tag: &str) -> DomainEvent {
        DomainEvent::SessionTagged {
            session_id: self.session_id.clone(),
            tag: Tag::try_new(tag.to_string()).unwrap(),
            tagged_at: Timestamp::from(self.clock.advance_by(Duration::seconds(5))),
        }
    }

    /// Build a complete session lifecycle
    pub fn complete_lifecycle(&mut self) -> Vec<DomainEvent> {
        vec![
            self.session_started(),
            self.session_tagged("test-run"),
            self.session_ended(SessionStatus::Completed),
        ]
    }
}

/// Builder for creating LLM request/response events
#[derive(Debug, Clone)]
pub struct LlmEventBuilder {
    request_id: RequestId,
    session_id: SessionId,
    model_version: ModelVersion,
    clock: MockClock,
}

impl LlmEventBuilder {
    /// Create a new LLM event builder
    pub fn new(session_id: SessionId) -> Self {
        Self {
            request_id: RequestId::generate(),
            session_id,
            model_version: ModelVersion::new("gpt-4-turbo-2024-01".to_string()),
            clock: MockClock::new(),
        }
    }

    /// Set the request ID
    pub fn with_request_id(mut self, request_id: RequestId) -> Self {
        self.request_id = request_id;
        self
    }

    /// Set the model version
    pub fn with_model_version(mut self, model: &str) -> Self {
        self.model_version = ModelVersion::new(model.to_string());
        self
    }

    /// Set the clock
    pub fn with_clock(mut self, clock: MockClock) -> Self {
        self.clock = clock;
        self
    }

    /// Build an LlmRequestReceived event
    pub fn request_received(&self, prompt: &str) -> DomainEvent {
        let mut params = HashMap::new();
        params.insert("temperature".to_string(), serde_json::json!(0.7));
        params.insert("max_tokens".to_string(), serde_json::json!(1000));

        DomainEvent::LlmRequestReceived {
            request_id: self.request_id.clone(),
            session_id: self.session_id.clone(),
            model_version: self.model_version.clone(),
            prompt: Prompt::try_new(prompt.to_string()).unwrap(),
            parameters: LlmParameters::new(params),
            received_at: Timestamp::from(self.clock.now()),
        }
    }

    /// Build an LlmRequestStarted event
    pub fn request_started(&mut self) -> DomainEvent {
        DomainEvent::LlmRequestStarted {
            request_id: self.request_id.clone(),
            started_at: Timestamp::from(self.clock.advance_by(Duration::milliseconds(10))),
        }
    }

    /// Build an LlmResponseReceived event
    pub fn response_received(&mut self, response: &str) -> DomainEvent {
        let metadata = ResponseMetadata::new(
            Some(150),
            Some(10),
            Some("stop".to_string()),
            Some(Duration::milliseconds(500).num_milliseconds() as u64),
        );

        DomainEvent::LlmResponseReceived {
            request_id: self.request_id.clone(),
            response_text: ResponseText::try_new(response.to_string()).unwrap(),
            metadata,
            received_at: Timestamp::from(self.clock.advance_by(Duration::milliseconds(500))),
        }
    }

    /// Build an LlmRequestFailed event
    pub fn request_failed(&mut self, error: &str) -> DomainEvent {
        DomainEvent::LlmRequestFailed {
            request_id: self.request_id.clone(),
            error_message: ErrorMessage::try_new(error.to_string()).unwrap(),
            failed_at: Timestamp::from(self.clock.advance_by(Duration::milliseconds(100))),
        }
    }

    /// Build a complete successful request cycle
    pub fn successful_request_cycle(&mut self, prompt: &str, response: &str) -> Vec<DomainEvent> {
        vec![
            self.request_received(prompt),
            self.request_started(),
            self.response_received(response),
        ]
    }

    /// Build a failed request cycle
    pub fn failed_request_cycle(&mut self, prompt: &str, error: &str) -> Vec<DomainEvent> {
        vec![
            self.request_received(prompt),
            self.request_started(),
            self.request_failed(error),
        ]
    }
}

/// Builder for version tracking events
#[derive(Debug, Clone)]
pub struct VersionEventBuilder {
    session_id: SessionId,
    clock: MockClock,
}

impl VersionEventBuilder {
    /// Create a new version event builder
    pub fn new(session_id: SessionId) -> Self {
        Self {
            session_id,
            clock: MockClock::new(),
        }
    }

    /// Build a VersionFirstSeen event
    pub fn version_first_seen(&self, model: &str) -> DomainEvent {
        DomainEvent::VersionFirstSeen {
            model_version: ModelVersion::new(model.to_string()),
            session_id: self.session_id.clone(),
            first_seen_at: Timestamp::from(self.clock.now()),
        }
    }

    /// Build a VersionChanged event
    pub fn version_changed(
        &mut self,
        from: &str,
        to: &str,
        change_type: VersionComparison,
    ) -> DomainEvent {
        DomainEvent::VersionChanged {
            change_id: VersionChangeId::generate(),
            session_id: self.session_id.clone(),
            from_version: ModelVersion::new(from.to_string()),
            to_version: ModelVersion::new(to.to_string()),
            change_type,
            reason: Some(ChangeReason::try_new("Test version change".to_string()).unwrap()),
            changed_at: Timestamp::from(self.clock.advance_by(Duration::hours(1))),
        }
    }

    /// Build a version upgrade scenario
    pub fn version_upgrade_scenario(&mut self, old_version: &str, new_version: &str) -> Vec<DomainEvent> {
        vec![
            self.version_first_seen(old_version),
            self.version_changed(old_version, new_version, VersionComparison::MajorUpgrade),
        ]
    }
}

/// Builder for metrics events
#[derive(Debug, Clone)]
pub struct MetricsEventBuilder {
    session_id: SessionId,
    clock: MockClock,
}

impl MetricsEventBuilder {
    /// Create a new metrics event builder
    pub fn new(session_id: SessionId) -> Self {
        Self {
            session_id,
            clock: MockClock::new(),
        }
    }

    /// Build an FScoreCalculated event
    pub fn f_score_calculated(
        &self,
        model: &str,
        f_score: f64,
        precision: Option<f64>,
        recall: Option<f64>,
        sample_count: u64,
    ) -> DomainEvent {
        DomainEvent::FScoreCalculated {
            session_id: self.session_id.clone(),
            model_version: ModelVersion::new(model.to_string()),
            f_score: FScore::try_new(f_score).unwrap(),
            precision: precision.map(|p| Precision::try_new(p).unwrap()),
            recall: recall.map(|r| Recall::try_new(r).unwrap()),
            sample_count: SampleCount::try_new(sample_count).unwrap(),
            calculated_at: Timestamp::from(self.clock.now()),
        }
    }

    /// Build a series of improving F-score measurements
    pub fn improving_f_scores(&mut self, model: &str) -> Vec<DomainEvent> {
        vec![
            self.f_score_calculated(model, 0.75, Some(0.70), Some(0.80), 100),
            {
                self.clock.advance_by(Duration::days(1));
                self.f_score_calculated(model, 0.82, Some(0.78), Some(0.86), 250)
            },
            {
                self.clock.advance_by(Duration::days(1));
                self.f_score_calculated(model, 0.88, Some(0.85), Some(0.91), 500)
            },
        ]
    }
}

/// Mock clock for controlling time in tests
#[derive(Debug, Clone)]
pub struct MockClock {
    current_time: DateTime<Utc>,
}

impl MockClock {
    /// Create a new mock clock starting at the current time
    pub fn new() -> Self {
        Self {
            current_time: Utc::now(),
        }
    }

    /// Create a mock clock starting at a specific time
    pub fn at(time: DateTime<Utc>) -> Self {
        Self { current_time: time }
    }

    /// Get the current time
    pub fn now(&self) -> DateTime<Utc> {
        self.current_time
    }

    /// Advance the clock by a duration
    pub fn advance_by(&mut self, duration: Duration) -> DateTime<Utc> {
        self.current_time = self.current_time + duration;
        self.current_time
    }

    /// Set the clock to a specific time
    pub fn set_time(&mut self, time: DateTime<Utc>) {
        self.current_time = time;
    }
}

impl Default for MockClock {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_event_builder() {
        let mut builder = SessionEventBuilder::new();
        let events = builder.complete_lifecycle();
        
        assert_eq!(events.len(), 3);
        assert!(matches!(events[0], DomainEvent::SessionStarted { .. }));
        assert!(matches!(events[1], DomainEvent::SessionTagged { .. }));
        assert!(matches!(events[2], DomainEvent::SessionEnded { .. }));
    }

    #[test]
    fn test_llm_event_builder_successful_cycle() {
        let session_id = SessionId::generate();
        let mut builder = LlmEventBuilder::new(session_id);
        let events = builder.successful_request_cycle("Hello", "Hi there!");
        
        assert_eq!(events.len(), 3);
        assert!(matches!(events[0], DomainEvent::LlmRequestReceived { .. }));
        assert!(matches!(events[1], DomainEvent::LlmRequestStarted { .. }));
        assert!(matches!(events[2], DomainEvent::LlmResponseReceived { .. }));
    }

    #[test]
    fn test_mock_clock_advancement() {
        let mut clock = MockClock::new();
        let initial_time = clock.now();
        let advanced_time = clock.advance_by(Duration::hours(1));
        
        assert_eq!(advanced_time - initial_time, Duration::hours(1));
    }
}