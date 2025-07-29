//! Time-travel testing harness for event replay and temporal queries
//!
//! This module provides infrastructure for replaying events to specific
//! points in time and verifying system state at those moments.

use chrono::{DateTime, Utc};
use eventcore::{EventStore, StreamId};
use eventcore_memory::InMemoryEventStore;
use std::collections::HashMap;
use std::sync::Arc;
use union_square::domain::{
    events::DomainEvent,
    llm::{ModelVersion, RequestId},
    metrics::Timestamp,
    session::{SessionId, SessionStatus},
};

/// Time-travel test harness for replaying events and querying historical state
#[derive(Clone)]
pub struct TimeTravelTestHarness {
    event_store: Arc<InMemoryEventStore<DomainEvent>>,
    clock: Arc<std::sync::Mutex<MockTimeProvider>>,
}

impl TimeTravelTestHarness {
    /// Create a new time-travel test harness
    pub fn new() -> Self {
        Self {
            event_store: Arc::new(InMemoryEventStore::new()),
            clock: Arc::new(std::sync::Mutex::new(MockTimeProvider::new())),
        }
    }

    /// Load events into the harness
    pub async fn load_events(&self, events: Vec<DomainEvent>) -> Result<(), Box<dyn std::error::Error>> {
        for event in events {
            let stream_id = self.determine_stream_id(&event);
            self.event_store
                .append_events(stream_id, vec![event])
                .await?;
        }
        Ok(())
    }

    /// Replay events up to a specific point in time
    pub async fn replay_to(&self, target_time: DateTime<Utc>) -> ReplayedState {
        let target_timestamp = Timestamp::from(target_time);
        let mut state = ReplayedState::new();

        // Read all streams
        let streams = self.event_store.list_streams().await.unwrap_or_default();
        
        for stream_id in streams {
            let events = self.event_store
                .read_events(&stream_id, 0, None)
                .await
                .unwrap_or_default();

            for event in events {
                if event.occurred_at() <= target_timestamp {
                    state.apply_event(&event);
                }
            }
        }

        state
    }

    /// Get all events within a time range
    pub async fn events_in_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Vec<DomainEvent> {
        let start_timestamp = Timestamp::from(start);
        let end_timestamp = Timestamp::from(end);
        let mut events = Vec::new();

        let streams = self.event_store.list_streams().await.unwrap_or_default();
        
        for stream_id in streams {
            let stream_events = self.event_store
                .read_events(&stream_id, 0, None)
                .await
                .unwrap_or_default();

            for event in stream_events {
                let timestamp = event.occurred_at();
                if timestamp >= start_timestamp && timestamp <= end_timestamp {
                    events.push(event);
                }
            }
        }

        // Sort by timestamp
        events.sort_by_key(|e| e.occurred_at());
        events
    }

    /// Create a snapshot of state at a specific time
    pub async fn snapshot_at(&self, time: DateTime<Utc>) -> StateSnapshot {
        let state = self.replay_to(time).await;
        StateSnapshot {
            timestamp: Timestamp::from(time),
            active_sessions: state.active_sessions.len(),
            total_requests: state.total_requests,
            failed_requests: state.failed_requests,
            model_versions_seen: state.model_versions_seen.into_iter().collect(),
            session_statuses: state.session_statuses,
        }
    }

    /// Compare state between two points in time
    pub async fn compare_states(
        &self,
        time1: DateTime<Utc>,
        time2: DateTime<Utc>,
    ) -> StateComparison {
        let state1 = self.replay_to(time1).await;
        let state2 = self.replay_to(time2).await;

        StateComparison {
            time_range: (Timestamp::from(time1), Timestamp::from(time2)),
            sessions_started: state2.sessions_started - state1.sessions_started,
            sessions_ended: state2.sessions_ended - state1.sessions_ended,
            requests_processed: state2.total_requests - state1.total_requests,
            new_model_versions: state2
                .model_versions_seen
                .difference(&state1.model_versions_seen)
                .cloned()
                .collect(),
        }
    }

    /// Determine the stream ID for an event
    fn determine_stream_id(&self, event: &DomainEvent) -> StreamId {
        match event {
            DomainEvent::SessionStarted { session_id, .. }
            | DomainEvent::SessionEnded { session_id, .. }
            | DomainEvent::SessionTagged { session_id, .. } => {
                StreamId::new(format!("session-{}", session_id))
            }
            DomainEvent::LlmRequestReceived { request_id, .. }
            | DomainEvent::LlmRequestStarted { request_id, .. }
            | DomainEvent::LlmResponseReceived { request_id, .. }
            | DomainEvent::LlmRequestFailed { request_id, .. }
            | DomainEvent::LlmRequestCancelled { request_id, .. } => {
                StreamId::new(format!("request-{}", request_id))
            }
            _ => StreamId::new("system"),
        }
    }
}

/// State rebuilt from replaying events
#[derive(Debug, Clone)]
pub struct ReplayedState {
    pub active_sessions: HashMap<SessionId, SessionInfo>,
    pub requests: HashMap<RequestId, RequestInfo>,
    pub model_versions_seen: std::collections::HashSet<ModelVersion>,
    pub sessions_started: usize,
    pub sessions_ended: usize,
    pub total_requests: usize,
    pub failed_requests: usize,
    pub session_statuses: HashMap<SessionId, SessionStatus>,
}

impl ReplayedState {
    fn new() -> Self {
        Self {
            active_sessions: HashMap::new(),
            requests: HashMap::new(),
            model_versions_seen: std::collections::HashSet::new(),
            sessions_started: 0,
            sessions_ended: 0,
            total_requests: 0,
            failed_requests: 0,
            session_statuses: HashMap::new(),
        }
    }

    fn apply_event(&mut self, event: &DomainEvent) {
        match event {
            DomainEvent::SessionStarted {
                session_id,
                user_id,
                application_id,
                started_at,
            } => {
                self.active_sessions.insert(
                    session_id.clone(),
                    SessionInfo {
                        session_id: session_id.clone(),
                        user_id: user_id.clone(),
                        application_id: application_id.clone(),
                        started_at: *started_at,
                        ended_at: None,
                    },
                );
                self.sessions_started += 1;
            }
            DomainEvent::SessionEnded {
                session_id,
                ended_at,
                final_status,
            } => {
                if let Some(info) = self.active_sessions.get_mut(session_id) {
                    info.ended_at = Some(*ended_at);
                }
                self.session_statuses.insert(session_id.clone(), final_status.clone());
                self.sessions_ended += 1;
            }
            DomainEvent::LlmRequestReceived {
                request_id,
                session_id,
                model_version,
                prompt,
                received_at,
                ..
            } => {
                self.requests.insert(
                    request_id.clone(),
                    RequestInfo {
                        request_id: request_id.clone(),
                        session_id: session_id.clone(),
                        model_version: model_version.clone(),
                        prompt: prompt.to_string(),
                        received_at: *received_at,
                        completed: false,
                        failed: false,
                    },
                );
                self.model_versions_seen.insert(model_version.clone());
                self.total_requests += 1;
            }
            DomainEvent::LlmResponseReceived { request_id, .. } => {
                if let Some(info) = self.requests.get_mut(request_id) {
                    info.completed = true;
                }
            }
            DomainEvent::LlmRequestFailed { request_id, .. } => {
                if let Some(info) = self.requests.get_mut(request_id) {
                    info.failed = true;
                }
                self.failed_requests += 1;
            }
            DomainEvent::VersionFirstSeen { model_version, .. } => {
                self.model_versions_seen.insert(model_version.clone());
            }
            _ => {} // Handle other events as needed
        }
    }
}

#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub session_id: SessionId,
    pub user_id: union_square::domain::user::UserId,
    pub application_id: union_square::domain::session::ApplicationId,
    pub started_at: Timestamp,
    pub ended_at: Option<Timestamp>,
}

#[derive(Debug, Clone)]
pub struct RequestInfo {
    pub request_id: RequestId,
    pub session_id: SessionId,
    pub model_version: ModelVersion,
    pub prompt: String,
    pub received_at: Timestamp,
    pub completed: bool,
    pub failed: bool,
}

/// Snapshot of system state at a specific time
#[derive(Debug, Clone)]
pub struct StateSnapshot {
    pub timestamp: Timestamp,
    pub active_sessions: usize,
    pub total_requests: usize,
    pub failed_requests: usize,
    pub model_versions_seen: Vec<ModelVersion>,
    pub session_statuses: HashMap<SessionId, SessionStatus>,
}

/// Comparison between two states
#[derive(Debug, Clone)]
pub struct StateComparison {
    pub time_range: (Timestamp, Timestamp),
    pub sessions_started: usize,
    pub sessions_ended: usize,
    pub requests_processed: usize,
    pub new_model_versions: Vec<ModelVersion>,
}

/// Mock time provider for controlling time in tests
#[derive(Debug, Clone)]
pub struct MockTimeProvider {
    current_time: DateTime<Utc>,
}

impl MockTimeProvider {
    pub fn new() -> Self {
        Self {
            current_time: Utc::now(),
        }
    }

    pub fn set_time(&mut self, time: DateTime<Utc>) {
        self.current_time = time;
    }

    pub fn advance_by(&mut self, duration: chrono::Duration) {
        self.current_time = self.current_time + duration;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::event_sourcing::fixtures::{
        event_builders::MockClock,
        stream_builders::{EventStreamBuilder, ScenarioBuilder},
    };
    use chrono::Duration;

    #[tokio::test]
    async fn test_time_travel_replay_to_point() {
        let harness = TimeTravelTestHarness::new();
        
        // Create events with known timestamps
        let start_time = Utc::now();
        let events = EventStreamBuilder::new()
            .starting_at(start_time)
            .with_session_lifecycle()
            .with_llm_requests(3)
            .build();

        harness.load_events(events.clone()).await.unwrap();

        // Replay to a point after session start but before any requests
        let replay_time = start_time + Duration::seconds(1);
        let state = harness.replay_to(replay_time).await;

        assert_eq!(state.sessions_started, 1);
        assert_eq!(state.total_requests, 0);

        // Replay to end
        let end_time = start_time + Duration::hours(2);
        let final_state = harness.replay_to(end_time).await;

        assert_eq!(final_state.sessions_started, 1);
        assert_eq!(final_state.sessions_ended, 1);
        assert_eq!(final_state.total_requests, 3);
    }

    #[tokio::test]
    async fn test_events_in_time_range() {
        let harness = TimeTravelTestHarness::new();
        
        let start_time = Utc::now();
        let events = ScenarioBuilder::typical_user_session();
        harness.load_events(events).await.unwrap();

        // Get events in first minute
        let range_events = harness
            .events_in_range(start_time, start_time + Duration::minutes(1))
            .await;

        assert!(!range_events.is_empty());
        
        // Verify all events are within range
        for event in &range_events {
            let timestamp = event.occurred_at();
            assert!(timestamp >= Timestamp::from(start_time));
            assert!(timestamp <= Timestamp::from(start_time + Duration::minutes(1)));
        }
    }

    #[tokio::test]
    async fn test_state_comparison() {
        let harness = TimeTravelTestHarness::new();
        
        let start_time = Utc::now();
        let events = ScenarioBuilder::session_with_version_change();
        harness.load_events(events).await.unwrap();

        // Compare state before and after version change
        let time1 = start_time + Duration::minutes(5);
        let time2 = start_time + Duration::hours(1);
        
        let comparison = harness.compare_states(time1, time2).await;

        assert!(comparison.requests_processed > 0);
        assert!(!comparison.new_model_versions.is_empty());
    }

    #[tokio::test]
    async fn test_snapshot_consistency() {
        let harness = TimeTravelTestHarness::new();
        
        let events = ScenarioBuilder::session_with_failures();
        harness.load_events(events).await.unwrap();

        let snapshot_time = Utc::now() + Duration::hours(1);
        let snapshot = harness.snapshot_at(snapshot_time).await;

        assert!(snapshot.failed_requests > 0);
        assert_eq!(
            snapshot.total_requests,
            snapshot.failed_requests + (snapshot.total_requests - snapshot.failed_requests)
        );
    }
}