//! Projection rebuild testing infrastructure
//!
//! Tests for verifying projection rebuild capabilities and ensuring
//! projections can be reconstructed from event streams consistently.

use eventcore_memory::InMemoryEventStore;
use std::collections::HashMap;
use std::sync::Arc;
use union_square::domain::{
    events::DomainEvent,
    llm::{ModelVersion, RequestId},
    metrics::{FScore, Timestamp},
    session::{ApplicationId, SessionId, SessionStatus},
    user::UserId,
};

/// Test harness for projection rebuilding
pub struct ProjectionRebuildHarness {
    event_store: Arc<InMemoryEventStore<DomainEvent>>,
}

impl ProjectionRebuildHarness {
    /// Create a new projection rebuild harness
    pub fn new() -> Self {
        Self {
            event_store: Arc::new(InMemoryEventStore::new()),
        }
    }

    /// Load events into the harness
    pub async fn load_events(&self, events: Vec<DomainEvent>) -> Result<(), Box<dyn std::error::Error>> {
        for (index, event) in events.into_iter().enumerate() {
            let stream_id = eventcore::StreamId::new(format!("stream-{}", index));
            self.event_store.append_events(stream_id, vec![event]).await?;
        }
        Ok(())
    }

    /// Rebuild a session summary projection from events
    pub async fn rebuild_session_projection(&self) -> SessionProjection {
        let mut projection = SessionProjection::new();
        
        // Read all events from all streams
        let streams = self.event_store.list_streams().await.unwrap_or_default();
        let mut all_events = Vec::new();
        
        for stream_id in streams {
            let events = self.event_store
                .read_events(&stream_id, 0, None)
                .await
                .unwrap_or_default();
            all_events.extend(events);
        }
        
        // Sort events by timestamp for proper ordering
        all_events.sort_by_key(|e| e.occurred_at());
        
        // Apply events to projection
        for event in all_events {
            projection.apply(&event);
        }
        
        projection
    }

    /// Rebuild a version tracking projection from events
    pub async fn rebuild_version_projection(&self) -> VersionProjection {
        let mut projection = VersionProjection::new();
        
        let streams = self.event_store.list_streams().await.unwrap_or_default();
        let mut all_events = Vec::new();
        
        for stream_id in streams {
            let events = self.event_store
                .read_events(&stream_id, 0, None)
                .await
                .unwrap_or_default();
            all_events.extend(events);
        }
        
        all_events.sort_by_key(|e| e.occurred_at());
        
        for event in all_events {
            projection.apply(&event);
        }
        
        projection
    }

    /// Rebuild a metrics projection from events
    pub async fn rebuild_metrics_projection(&self) -> MetricsProjection {
        let mut projection = MetricsProjection::new();
        
        let streams = self.event_store.list_streams().await.unwrap_or_default();
        let mut all_events = Vec::new();
        
        for stream_id in streams {
            let events = self.event_store
                .read_events(&stream_id, 0, None)
                .await
                .unwrap_or_default();
            all_events.extend(events);
        }
        
        all_events.sort_by_key(|e| e.occurred_at());
        
        for event in all_events {
            projection.apply(&event);
        }
        
        projection
    }

    /// Test incremental vs full rebuild consistency
    pub async fn test_rebuild_consistency(&self, events: Vec<DomainEvent>) -> RebuildConsistencyResult {
        // Clear any existing events
        *self = Self::new();
        
        // Build projection incrementally
        let mut incremental_projection = SessionProjection::new();
        let mut batch_size = 0;
        
        for event in &events {
            let stream_id = eventcore::StreamId::new(format!("stream-inc-{}", batch_size));
            self.event_store.append_events(stream_id, vec![event.clone()]).await.unwrap();
            incremental_projection.apply(event);
            batch_size += 1;
        }
        
        // Clear and rebuild from scratch
        let fresh_harness = Self::new();
        fresh_harness.load_events(events.clone()).await.unwrap();
        let full_rebuild_projection = fresh_harness.rebuild_session_projection().await;
        
        RebuildConsistencyResult {
            incremental_projection,
            full_rebuild_projection,
            events_processed: events.len(),
            consistent: incremental_projection == full_rebuild_projection,
        }
    }
}

/// Session summary projection for testing
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionProjection {
    pub sessions: HashMap<SessionId, SessionSummary>,
    pub active_sessions: HashMap<SessionId, SessionSummary>,
    pub total_sessions_started: usize,
    pub total_sessions_completed: usize,
    pub total_requests: usize,
    pub failed_requests: usize,
}

impl SessionProjection {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            active_sessions: HashMap::new(),
            total_sessions_started: 0,
            total_sessions_completed: 0,
            total_requests: 0,
            failed_requests: 0,
        }
    }

    pub fn apply(&mut self, event: &DomainEvent) {
        match event {
            DomainEvent::SessionStarted {
                session_id,
                user_id,
                application_id,
                started_at,
            } => {
                let summary = SessionSummary {
                    session_id: session_id.clone(),
                    user_id: user_id.clone(),
                    application_id: application_id.clone(),
                    started_at: *started_at,
                    ended_at: None,
                    status: SessionStatus::Active,
                    request_count: 0,
                    failed_request_count: 0,
                    tags: Vec::new(),
                };
                
                self.sessions.insert(session_id.clone(), summary.clone());
                self.active_sessions.insert(session_id.clone(), summary);
                self.total_sessions_started += 1;
            }
            DomainEvent::SessionEnded {
                session_id,
                ended_at,
                final_status,
            } => {
                if let Some(summary) = self.sessions.get_mut(session_id) {
                    summary.ended_at = Some(*ended_at);
                    summary.status = final_status.clone();
                    
                    if *final_status == SessionStatus::Completed {
                        self.total_sessions_completed += 1;
                    }
                }
                
                self.active_sessions.remove(session_id);
            }
            DomainEvent::SessionTagged {
                session_id,
                tag,
                ..
            } => {
                if let Some(summary) = self.sessions.get_mut(session_id) {
                    summary.tags.push(tag.to_string());
                }
                if let Some(summary) = self.active_sessions.get_mut(session_id) {
                    summary.tags.push(tag.to_string());
                }
            }
            DomainEvent::LlmRequestReceived { session_id, .. } => {
                if let Some(summary) = self.sessions.get_mut(session_id) {
                    summary.request_count += 1;
                }
                if let Some(summary) = self.active_sessions.get_mut(session_id) {
                    summary.request_count += 1;
                }
                self.total_requests += 1;
            }
            DomainEvent::LlmRequestFailed { session_id, .. } => {
                if let Some(summary) = self.sessions.get_mut(session_id) {
                    summary.failed_request_count += 1;
                }
                if let Some(summary) = self.active_sessions.get_mut(session_id) {
                    summary.failed_request_count += 1;
                }
                self.failed_requests += 1;
            }
            _ => {} // Ignore other events for this projection
        }
    }

    /// Verify projection invariants
    pub fn verify_invariants(&self) -> ProjectionInvariantResult {
        let mut errors = Vec::new();

        // Check that active sessions are a subset of all sessions
        for (session_id, active_summary) in &self.active_sessions {
            if let Some(full_summary) = self.sessions.get(session_id) {
                if active_summary != full_summary {
                    errors.push(format!(
                        "Active session {} differs from full session record",
                        session_id
                    ));
                }
            } else {
                errors.push(format!(
                    "Active session {} not found in full session records",
                    session_id
                ));
            }
        }

        // Check that ended sessions are not in active sessions
        for (session_id, summary) in &self.sessions {
            if summary.ended_at.is_some() && self.active_sessions.contains_key(session_id) {
                errors.push(format!(
                    "Ended session {} is still in active sessions",
                    session_id
                ));
            }
        }

        // Check request count consistency
        let total_from_sessions: usize = self.sessions.values().map(|s| s.request_count).sum();
        if total_from_sessions != self.total_requests {
            errors.push(format!(
                "Request count mismatch: sessions={}, total={}",
                total_from_sessions, self.total_requests
            ));
        }

        ProjectionInvariantResult {
            is_valid: errors.is_empty(),
            errors,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionSummary {
    pub session_id: SessionId,
    pub user_id: UserId,
    pub application_id: ApplicationId,
    pub started_at: Timestamp,
    pub ended_at: Option<Timestamp>,
    pub status: SessionStatus,
    pub request_count: usize,
    pub failed_request_count: usize,
    pub tags: Vec<String>,
}

/// Version tracking projection for testing
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionProjection {
    pub versions_seen: HashMap<ModelVersion, VersionInfo>,
    pub version_usage_count: HashMap<ModelVersion, usize>,
    pub first_seen_order: Vec<ModelVersion>,
}

impl VersionProjection {
    pub fn new() -> Self {
        Self {
            versions_seen: HashMap::new(),
            version_usage_count: HashMap::new(),
            first_seen_order: Vec::new(),
        }
    }

    pub fn apply(&mut self, event: &DomainEvent) {
        match event {
            DomainEvent::VersionFirstSeen {
                model_version,
                session_id,
                first_seen_at,
            } => {
                if !self.versions_seen.contains_key(model_version) {
                    self.versions_seen.insert(
                        model_version.clone(),
                        VersionInfo {
                            version: model_version.clone(),
                            first_seen_at: *first_seen_at,
                            first_seen_in_session: session_id.clone(),
                            usage_count: 0,
                        },
                    );
                    self.first_seen_order.push(model_version.clone());
                }
            }
            DomainEvent::LlmRequestReceived { model_version, .. } => {
                *self.version_usage_count.entry(model_version.clone()).or_insert(0) += 1;
                
                if let Some(info) = self.versions_seen.get_mut(model_version) {
                    info.usage_count += 1;
                }
            }
            _ => {} // Ignore other events
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionInfo {
    pub version: ModelVersion,
    pub first_seen_at: Timestamp,
    pub first_seen_in_session: SessionId,
    pub usage_count: usize,
}

/// Metrics projection for testing
#[derive(Debug, Clone, PartialEq)]
pub struct MetricsProjection {
    pub f_scores: HashMap<ModelVersion, Vec<FScoreRecord>>,
    pub latest_f_scores: HashMap<ModelVersion, FScore>,
    pub f_score_history_count: usize,
}

impl MetricsProjection {
    pub fn new() -> Self {
        Self {
            f_scores: HashMap::new(),
            latest_f_scores: HashMap::new(),
            f_score_history_count: 0,
        }
    }

    pub fn apply(&mut self, event: &DomainEvent) {
        match event {
            DomainEvent::FScoreCalculated {
                model_version,
                f_score,
                calculated_at,
                sample_count,
                ..
            } => {
                let record = FScoreRecord {
                    f_score: f_score.clone(),
                    calculated_at: *calculated_at,
                    sample_count: sample_count.clone(),
                };
                
                self.f_scores
                    .entry(model_version.clone())
                    .or_insert_with(Vec::new)
                    .push(record);
                
                self.latest_f_scores.insert(model_version.clone(), f_score.clone());
                self.f_score_history_count += 1;
            }
            _ => {} // Ignore other events
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FScoreRecord {
    pub f_score: FScore,
    pub calculated_at: Timestamp,
    pub sample_count: union_square::domain::metrics::SampleCount,
}

/// Result of projection rebuild consistency test
#[derive(Debug)]
pub struct RebuildConsistencyResult {
    pub incremental_projection: SessionProjection,
    pub full_rebuild_projection: SessionProjection,
    pub events_processed: usize,
    pub consistent: bool,
}

/// Result of projection invariant verification
#[derive(Debug)]
pub struct ProjectionInvariantResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::event_sourcing::fixtures::stream_builders::{EventStreamBuilder, ScenarioBuilder};

    #[tokio::test]
    async fn test_session_projection_rebuild() {
        let harness = ProjectionRebuildHarness::new();
        let events = ScenarioBuilder::typical_user_session();
        
        harness.load_events(events).await.unwrap();
        let projection = harness.rebuild_session_projection().await;
        
        assert!(projection.total_sessions_started > 0);
        assert!(projection.total_requests > 0);
        
        // Verify projection invariants
        let invariant_result = projection.verify_invariants();
        assert!(invariant_result.is_valid, "Invariants should be valid: {:?}", invariant_result.errors);
    }

    #[tokio::test]
    async fn test_version_projection_rebuild() {
        let harness = ProjectionRebuildHarness::new();
        let events = ScenarioBuilder::session_with_version_change();
        
        harness.load_events(events).await.unwrap();
        let projection = harness.rebuild_version_projection().await;
        
        assert!(!projection.versions_seen.is_empty());
        assert!(!projection.version_usage_count.is_empty());
    }

    #[tokio::test]
    async fn test_metrics_projection_rebuild() {
        let harness = ProjectionRebuildHarness::new();
        let events = EventStreamBuilder::new()
            .with_session_lifecycle()
            .with_llm_requests(3)
            .with_metrics("gpt-4", 0.85)
            .build();
        
        harness.load_events(events).await.unwrap();
        let projection = harness.rebuild_metrics_projection().await;
        
        assert!(!projection.f_scores.is_empty());
        assert!(!projection.latest_f_scores.is_empty());
        assert!(projection.f_score_history_count > 0);
    }

    #[tokio::test]
    async fn test_rebuild_consistency() {
        let harness = ProjectionRebuildHarness::new();
        let events = ScenarioBuilder::concurrent_sessions(2)
            .into_iter()
            .flat_map(|(_, events)| events)
            .collect();
        
        let result = harness.test_rebuild_consistency(events).await;
        
        assert!(result.consistent, "Incremental and full rebuild should be consistent");
        assert!(result.events_processed > 0);
    }

    #[tokio::test]
    async fn test_projection_invariants_complex_scenario() {
        let harness = ProjectionRebuildHarness::new();
        let events = ScenarioBuilder::long_running_session_with_model_evolution();
        
        harness.load_events(events).await.unwrap();
        let projection = harness.rebuild_session_projection().await;
        
        let invariant_result = projection.verify_invariants();
        assert!(invariant_result.is_valid, "Complex scenario should maintain invariants: {:?}", invariant_result.errors);
    }

    #[tokio::test]
    async fn test_projection_handles_failures() {
        let harness = ProjectionRebuildHarness::new();
        let events = ScenarioBuilder::session_with_failures();
        
        harness.load_events(events).await.unwrap();
        let projection = harness.rebuild_session_projection().await;
        
        assert!(projection.failed_requests > 0);
        
        let invariant_result = projection.verify_invariants();
        assert!(invariant_result.is_valid, "Should handle failures correctly: {:?}", invariant_result.errors);
    }
}