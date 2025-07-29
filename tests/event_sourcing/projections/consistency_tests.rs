//! Projection consistency testing
//!
//! Tests for verifying that projections maintain consistency
//! with the event stream and with each other.

use std::collections::HashMap;
use union_square::domain::{
    events::DomainEvent,
    llm::{ModelVersion, RequestId},
    metrics::Timestamp,
    session::{SessionId, SessionStatus},
};

use super::rebuild_tests::{
    MetricsProjection, ProjectionRebuildHarness, SessionProjection, VersionProjection,
};

/// Test harness for projection consistency verification
pub struct ProjectionConsistencyTester {
    harness: ProjectionRebuildHarness,
    events: Vec<DomainEvent>,
}

impl ProjectionConsistencyTester {
    /// Create a new consistency tester
    pub fn new(events: Vec<DomainEvent>) -> Self {
        Self {
            harness: ProjectionRebuildHarness::new(),
            events,
        }
    }

    /// Load events and run consistency tests
    pub async fn run_all_consistency_tests(&self) -> ConsistencyTestResults {
        self.harness.load_events(self.events.clone()).await.unwrap();

        let session_projection = self.harness.rebuild_session_projection().await;
        let version_projection = self.harness.rebuild_version_projection().await;
        let metrics_projection = self.harness.rebuild_metrics_projection().await;

        ConsistencyTestResults {
            cross_projection_consistency: self.test_cross_projection_consistency(
                &session_projection,
                &version_projection,
                &metrics_projection,
            ),
            event_stream_consistency: self.test_event_stream_consistency(&session_projection),
            temporal_consistency: self.test_temporal_consistency(),
            aggregate_consistency: self.test_aggregate_consistency(&session_projection),
            idempotency_consistency: self.test_idempotency().await,
        }
    }

    /// Test consistency between different projections
    fn test_cross_projection_consistency(
        &self,
        session_proj: &SessionProjection,
        version_proj: &VersionProjection,
        _metrics_proj: &MetricsProjection,
    ) -> CrossProjectionConsistencyResult {
        let mut errors = Vec::new();

        // Check that version usage counts align with session request counts
        let total_requests_from_sessions: usize = session_proj
            .sessions
            .values()
            .map(|s| s.request_count)
            .sum();

        let total_requests_from_versions: usize = version_proj
            .version_usage_count
            .values()
            .sum();

        if total_requests_from_sessions != total_requests_from_versions {
            errors.push(format!(
                "Request count mismatch: sessions={}, versions={}",
                total_requests_from_sessions, total_requests_from_versions
            ));
        }

        // Check that all versions used in sessions are tracked in version projection
        for event in &self.events {
            if let DomainEvent::LlmRequestReceived { model_version, .. } = event {
                if !version_proj.versions_seen.contains_key(model_version) {
                    errors.push(format!(
                        "Model version {} used in request but not tracked in version projection",
                        model_version
                    ));
                }
            }
        }

        CrossProjectionConsistencyResult {
            is_consistent: errors.is_empty(),
            errors,
        }
    }

    /// Test consistency between projections and the original event stream
    fn test_event_stream_consistency(&self, session_proj: &SessionProjection) -> EventStreamConsistencyResult {
        let mut errors = Vec::new();

        // Count events in the stream and compare with projection counts
        let mut stream_session_starts = 0;
        let mut stream_session_ends = 0;
        let mut stream_requests = 0;
        let mut stream_failed_requests = 0;

        for event in &self.events {
            match event {
                DomainEvent::SessionStarted { .. } => stream_session_starts += 1,
                DomainEvent::SessionEnded { .. } => stream_session_ends += 1,
                DomainEvent::LlmRequestReceived { .. } => stream_requests += 1,
                DomainEvent::LlmRequestFailed { .. } => stream_failed_requests += 1,
                _ => {}
            }
        }

        // Verify counts match
        if stream_session_starts != session_proj.total_sessions_started {
            errors.push(format!(
                "Session start count mismatch: stream={}, projection={}",
                stream_session_starts, session_proj.total_sessions_started
            ));
        }

        if stream_requests != session_proj.total_requests {
            errors.push(format!(
                "Request count mismatch: stream={}, projection={}",
                stream_requests, session_proj.total_requests
            ));
        }

        EventStreamConsistencyResult {
            is_consistent: errors.is_empty(),
            errors,
            events_verified: self.events.len(),
        }
    }

    /// Test temporal consistency (events processed in correct order)
    fn test_temporal_consistency(&self) -> TemporalConsistencyResult {
        let mut errors = Vec::new();
        let mut last_timestamp: Option<Timestamp> = None;

        // Verify events are in temporal order
        for (index, event) in self.events.iter().enumerate() {
            let timestamp = event.occurred_at();
            
            if let Some(last) = last_timestamp {
                if timestamp < last {
                    errors.push(format!(
                        "Event {} has timestamp {} before previous event timestamp {}",
                        index, timestamp, last
                    ));
                }
            }
            
            last_timestamp = Some(timestamp);
        }

        // Test session lifecycle temporal consistency
        let mut session_lifecycles = HashMap::new();
        
        for event in &self.events {
            match event {
                DomainEvent::SessionStarted { session_id, started_at, .. } => {
                    session_lifecycles.insert(session_id.clone(), (*started_at, None));
                }
                DomainEvent::SessionEnded { session_id, ended_at, .. } => {
                    if let Some((start_time, end_slot)) = session_lifecycles.get_mut(session_id) {
                        if *ended_at <= *start_time {
                            errors.push(format!(
                                "Session {} ended at {} before or at start time {}",
                                session_id, ended_at, start_time
                            ));
                        }
                        *end_slot = Some(*ended_at);
                    }
                }
                _ => {}
            }
        }

        TemporalConsistencyResult {
            is_consistent: errors.is_empty(),
            errors,
        }
    }

    /// Test aggregate consistency (session-level invariants)
    fn test_aggregate_consistency(&self, session_proj: &SessionProjection) -> AggregateConsistencyResult {
        let mut errors = Vec::new();

        // Group events by session to test session-level consistency
        let mut session_events: HashMap<SessionId, Vec<&DomainEvent>> = HashMap::new();
        
        for event in &self.events {
            let session_id = match event {
                DomainEvent::SessionStarted { session_id, .. } => Some(session_id),
                DomainEvent::SessionEnded { session_id, .. } => Some(session_id),
                DomainEvent::SessionTagged { session_id, .. } => Some(session_id),
                DomainEvent::LlmRequestReceived { session_id, .. } => Some(session_id),
                DomainEvent::LlmRequestFailed { session_id, .. } => Some(session_id),
                _ => None,
            };

            if let Some(sid) = session_id {
                session_events.entry(sid.clone()).or_insert_with(Vec::new).push(event);
            }
        }

        // Test each session's consistency
        for (session_id, events) in session_events {
            // Check that session has start event
            let has_start = events.iter().any(|e| matches!(e, DomainEvent::SessionStarted { .. }));
            if !has_start {
                errors.push(format!("Session {} has events but no start event", session_id));
            }

            // Check request/failure count consistency
            let request_count = events.iter()
                .filter(|e| matches!(e, DomainEvent::LlmRequestReceived { .. }))
                .count();
            let failure_count = events.iter()
                .filter(|e| matches!(e, DomainEvent::LlmRequestFailed { .. }))
                .count();

            if let Some(session_summary) = session_proj.sessions.get(&session_id) {
                if session_summary.request_count != request_count {
                    errors.push(format!(
                        "Session {} request count mismatch: events={}, projection={}",
                        session_id, request_count, session_summary.request_count
                    ));
                }

                if session_summary.failed_request_count != failure_count {
                    errors.push(format!(
                        "Session {} failure count mismatch: events={}, projection={}",
                        session_id, failure_count, session_summary.failed_request_count
                    ));
                }
            }
        }

        AggregateConsistencyResult {
            is_consistent: errors.is_empty(),
            errors,
            sessions_tested: session_events.len(),
        }
    }

    /// Test idempotency (processing events multiple times yields same result)
    async fn test_idempotency(&self) -> IdempotencyConsistencyResult {
        // Build projection first time
        let harness1 = ProjectionRebuildHarness::new();
        harness1.load_events(self.events.clone()).await.unwrap();
        let projection1 = harness1.rebuild_session_projection().await;

        // Build projection second time with same events
        let harness2 = ProjectionRebuildHarness::new();
        harness2.load_events(self.events.clone()).await.unwrap();
        let projection2 = harness2.rebuild_session_projection().await;

        // Test with duplicated events (simulating reprocessing)
        let mut duplicated_events = self.events.clone();
        duplicated_events.extend_from_slice(&self.events);
        
        let harness3 = ProjectionRebuildHarness::new();
        harness3.load_events(duplicated_events).await.unwrap();
        let projection3 = harness3.rebuild_session_projection().await;

        let identical = projection1 == projection2;
        let doubled_counts = projection3.total_sessions_started == projection1.total_sessions_started * 2;

        IdempotencyConsistencyResult {
            identical_rebuilds: identical,
            doubled_events_doubled_counts: doubled_counts,
            first_build_sessions: projection1.total_sessions_started,
            second_build_sessions: projection2.total_sessions_started,
            doubled_build_sessions: projection3.total_sessions_started,
        }
    }
}

/// Results of all consistency tests
#[derive(Debug)]
pub struct ConsistencyTestResults {
    pub cross_projection_consistency: CrossProjectionConsistencyResult,
    pub event_stream_consistency: EventStreamConsistencyResult,
    pub temporal_consistency: TemporalConsistencyResult,
    pub aggregate_consistency: AggregateConsistencyResult,
    pub idempotency_consistency: IdempotencyConsistencyResult,
}

impl ConsistencyTestResults {
    /// Check if all consistency tests passed
    pub fn all_consistent(&self) -> bool {
        self.cross_projection_consistency.is_consistent
            && self.event_stream_consistency.is_consistent
            && self.temporal_consistency.is_consistent
            && self.aggregate_consistency.is_consistent
            && self.idempotency_consistency.identical_rebuilds
    }

    /// Get a summary of all errors
    pub fn all_errors(&self) -> Vec<String> {
        let mut all_errors = Vec::new();
        
        all_errors.extend(self.cross_projection_consistency.errors.clone());
        all_errors.extend(self.event_stream_consistency.errors.clone());
        all_errors.extend(self.temporal_consistency.errors.clone());
        all_errors.extend(self.aggregate_consistency.errors.clone());
        
        if !self.idempotency_consistency.identical_rebuilds {
            all_errors.push("Idempotency test failed: rebuilds not identical".to_string());
        }
        
        all_errors
    }
}

#[derive(Debug)]
pub struct CrossProjectionConsistencyResult {
    pub is_consistent: bool,
    pub errors: Vec<String>,
}

#[derive(Debug)]
pub struct EventStreamConsistencyResult {
    pub is_consistent: bool,
    pub errors: Vec<String>,
    pub events_verified: usize,
}

#[derive(Debug)]
pub struct TemporalConsistencyResult {
    pub is_consistent: bool,
    pub errors: Vec<String>,
}

#[derive(Debug)]
pub struct AggregateConsistencyResult {
    pub is_consistent: bool,
    pub errors: Vec<String>,
    pub sessions_tested: usize,
}

#[derive(Debug)]
pub struct IdempotencyConsistencyResult {
    pub identical_rebuilds: bool,
    pub doubled_events_doubled_counts: bool,
    pub first_build_sessions: usize,
    pub second_build_sessions: usize,
    pub doubled_build_sessions: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::event_sourcing::fixtures::stream_builders::ScenarioBuilder;

    #[tokio::test]
    async fn test_typical_session_consistency() {
        let events = ScenarioBuilder::typical_user_session();
        let tester = ProjectionConsistencyTester::new(events);
        
        let results = tester.run_all_consistency_tests().await;
        
        assert!(results.all_consistent(), "All consistency tests should pass: {:?}", results.all_errors());
    }

    #[tokio::test]
    async fn test_concurrent_sessions_consistency() {
        let session_events = ScenarioBuilder::concurrent_sessions(3);
        let events: Vec<DomainEvent> = session_events
            .into_iter()
            .flat_map(|(_, events)| events)
            .collect();
        
        let tester = ProjectionConsistencyTester::new(events);
        let results = tester.run_all_consistency_tests().await;
        
        assert!(results.all_consistent(), "Concurrent sessions should be consistent: {:?}", results.all_errors());
    }

    #[tokio::test]
    async fn test_complex_scenario_consistency() {
        let events = ScenarioBuilder::long_running_session_with_model_evolution();
        let tester = ProjectionConsistencyTester::new(events);
        
        let results = tester.run_all_consistency_tests().await;
        
        assert!(results.all_consistent(), "Complex scenario should be consistent: {:?}", results.all_errors());
    }

    #[tokio::test]
    async fn test_failure_scenario_consistency() {
        let events = ScenarioBuilder::session_with_failures();
        let tester = ProjectionConsistencyTester::new(events);
        
        let results = tester.run_all_consistency_tests().await;
        
        assert!(results.all_consistent(), "Failure scenarios should maintain consistency: {:?}", results.all_errors());
    }

    #[tokio::test]
    async fn test_version_change_consistency() {
        let events = ScenarioBuilder::session_with_version_change();
        let tester = ProjectionConsistencyTester::new(events);
        
        let results = tester.run_all_consistency_tests().await;
        
        assert!(results.cross_projection_consistency.is_consistent);
        assert!(results.event_stream_consistency.is_consistent);
        assert!(results.temporal_consistency.is_consistent);
    }

    #[tokio::test]
    async fn test_idempotency_with_replay_scenario() {
        let events = ScenarioBuilder::replay_test_scenario();
        let tester = ProjectionConsistencyTester::new(events);
        
        let results = tester.run_all_consistency_tests().await;
        
        assert!(results.idempotency_consistency.identical_rebuilds);
        assert!(results.idempotency_consistency.doubled_events_doubled_counts);
    }
}