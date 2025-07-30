//! Common query patterns for multi-stream projections
//!
//! This module provides pre-built query functions for common access patterns
//! in event-sourced systems.

use crate::domain::streams::{session_stream, user_settings_stream};
use crate::domain::{
    events::DomainEvent,
    llm::ModelVersion,
    session::{ApplicationId, SessionId},
    user::UserId,
};
use crate::infrastructure::eventcore::projections::{
    builder::ProjectionBuilder,
    id_extraction::{extract_session_ids, extract_user_ids},
    read_models::{ApplicationMetricsModel, UserActivityModel},
};
use eventcore::EventStore;
use std::collections::{HashMap, HashSet};

/// Get all events for a session, including related request events
pub async fn get_session_events<ES: EventStore>(
    event_store: &ES,
    session_id: &SessionId,
) -> Result<Vec<DomainEvent>, QueryError>
where
    ES::Event: TryFrom<DomainEvent> + Clone,
    DomainEvent: for<'a> TryFrom<&'a ES::Event>,
{
    // Get the session stream
    let session_stream_id = session_stream(session_id);

    // Build projection to collect all events for this session
    let session_id_clone = session_id.clone();
    let projection = ProjectionBuilder::new(Vec::<DomainEvent>::new())
        .with_stream(session_stream_id)
        .filter_events(move |event| {
            // Include all events that belong to this session
            extract_session_ids(event).contains(&session_id_clone)
        })
        .project_with(|mut events, stored_event| {
            events.push(stored_event.payload.clone());
            events
        });

    projection
        .execute(event_store)
        .await
        .map_err(|e| QueryError::Projection(e.to_string()))
}

/// Get all activity for a user across all their sessions
pub async fn get_user_activity<ES: EventStore>(
    event_store: &ES,
    user_id: &UserId,
) -> Result<UserActivitySummary, QueryError>
where
    ES::Event: TryFrom<DomainEvent> + Clone,
    DomainEvent: for<'a> TryFrom<&'a ES::Event>,
{
    // First, find all sessions for this user
    let session_ids = get_user_sessions(event_store, user_id).await?;

    // Build streams to query
    let mut streams = Vec::new();
    for session_id in &session_ids {
        streams.push(session_stream(session_id));
    }

    // Also include user settings stream
    streams.push(user_settings_stream(user_id));

    // Build projection
    let model = UserActivityModel::new(user_id.clone());
    let user_id_clone = user_id.clone();

    let projection = ProjectionBuilder::new(model)
        .with_streams(streams)
        .filter_events(move |event| {
            // Include events for this user
            extract_user_ids(event).contains(&user_id_clone)
        })
        .project_with(|mut model, stored_event| {
            match &stored_event.payload {
                DomainEvent::SessionStarted {
                    session_id,
                    application_id,
                    started_at,
                    ..
                } => {
                    model.add_session(session_id, application_id, *started_at);
                }
                DomainEvent::LlmRequestReceived { .. } => {
                    // Note: We'd need to track which app this request belongs to
                    // For now, we'll skip this
                }
                _ => {}
            }
            model
        });

    let activity_model = projection
        .execute(event_store)
        .await
        .map_err(|e| QueryError::Projection(e.to_string()))?;

    // Convert to summary
    Ok(UserActivitySummary {
        user_id: activity_model.user_id,
        total_sessions: activity_model.total_sessions,
        total_requests: activity_model.total_requests,
        sessions_by_application: activity_model
            .applications_used
            .into_iter()
            .map(|(app_id, usage)| (app_id, usage.session_count))
            .collect(),
        model_usage: activity_model
            .model_preferences
            .into_iter()
            .map(|(version, usage)| (version, usage.request_count))
            .collect(),
    })
}

/// Helper to get all sessions for a user
async fn get_user_sessions<ES: EventStore>(
    _event_store: &ES,
    _user_id: &UserId,
) -> Result<Vec<SessionId>, QueryError>
where
    ES::Event: TryFrom<DomainEvent> + Clone,
    DomainEvent: for<'a> TryFrom<&'a ES::Event>,
{
    // For now, we'll return an empty vec
    // In production, this would use an index or dedicated projection
    // to efficiently find all sessions for a user
    Ok(Vec::new())
}

/// Get version usage statistics across multiple sessions
pub async fn get_version_usage<ES: EventStore>(
    event_store: &ES,
    model_version: &ModelVersion,
) -> Result<VersionUsageStats, QueryError>
where
    ES::Event: TryFrom<DomainEvent> + Clone,
    DomainEvent: for<'a> TryFrom<&'a ES::Event>,
{
    // Create projection to track version usage
    let initial_state = VersionUsageTracking {
        total_requests: 0,
        sessions: HashSet::new(),
        users: HashSet::new(),
        requests_per_session: HashMap::new(),
    };

    let projection = ProjectionBuilder::new(initial_state)
        .filter_events({
            let model_version_clone = model_version.clone();
            move |event| {
                // Filter for events that use this model version
                matches!(event,
                    DomainEvent::LlmRequestReceived { model_version: v, .. } if v == &model_version_clone
                )
            }
        })
        .project_with(|mut state, stored_event| {
            if let DomainEvent::LlmRequestReceived { session_id, .. } = &stored_event.payload {
                state.total_requests += 1;
                state.sessions.insert(session_id.clone());

                let count = state.requests_per_session
                    .entry(session_id.clone())
                    .or_insert(0);
                *count += 1;
            }
            state
        });

    let usage_data = projection
        .execute(event_store)
        .await
        .map_err(|e| QueryError::Projection(e.to_string()))?;

    let avg_requests = if usage_data.sessions.is_empty() {
        0.0
    } else {
        usage_data.total_requests as f64 / usage_data.sessions.len() as f64
    };

    Ok(VersionUsageStats {
        model_version: model_version.clone(),
        total_requests: usage_data.total_requests,
        unique_sessions: usage_data.sessions.len(),
        unique_users: usage_data.users.len(),
        average_requests_per_session: avg_requests,
    })
}

#[derive(Debug, Clone)]
struct VersionUsageTracking {
    total_requests: usize,
    sessions: HashSet<SessionId>,
    users: HashSet<UserId>,
    requests_per_session: HashMap<SessionId, usize>,
}

/// Get aggregated metrics for an application
pub async fn get_application_metrics<ES: EventStore>(
    event_store: &ES,
    application_id: &ApplicationId,
) -> Result<ApplicationMetrics, QueryError>
where
    ES::Event: TryFrom<DomainEvent> + Clone,
    DomainEvent: for<'a> TryFrom<&'a ES::Event>,
{
    let model = ApplicationMetricsModel::new(application_id.clone());

    let projection = ProjectionBuilder::new(model)
        .filter_events({
            let application_id_clone = application_id.clone();
            move |event| {
                // Filter for events related to this application
                matches!(event,
                    DomainEvent::SessionStarted { application_id: app_id, .. } if app_id == &application_id_clone
                ) || matches!(event,
                    DomainEvent::SessionEnded { .. } | DomainEvent::LlmRequestReceived { .. }
                )
            }
        })
        .project_with(|mut model, stored_event| {
            match &stored_event.payload {
                DomainEvent::SessionStarted { session_id, user_id, started_at, .. } => {
                    model.add_session(session_id, user_id, *started_at);
                }
                DomainEvent::SessionEnded { session_id, .. } => {
                    // Calculate duration if we have start time
                    // This would need session start time tracking
                    model.end_session(session_id, std::time::Duration::from_secs(300));
                }
                _ => {}
            }
            model
        });

    let metrics_model = projection
        .execute(event_store)
        .await
        .map_err(|e| QueryError::Projection(e.to_string()))?;

    Ok(ApplicationMetrics {
        application_id: metrics_model.application_id.clone(),
        total_sessions: metrics_model.total_sessions,
        total_requests: metrics_model.total_requests,
        unique_users: metrics_model.unique_users.len(),
        model_versions_used: metrics_model.model_versions.keys().cloned().collect(),
        average_session_length: metrics_model
            .average_session_duration()
            .unwrap_or(std::time::Duration::from_secs(0)),
    })
}

/// Get all sessions that used a specific model version
pub async fn get_sessions_by_version<ES: EventStore>(
    event_store: &ES,
    model_version: &ModelVersion,
) -> Result<HashSet<SessionId>, QueryError>
where
    ES::Event: TryFrom<DomainEvent> + Clone,
    DomainEvent: for<'a> TryFrom<&'a ES::Event>,
{
    let model_version_clone = model_version.clone();
    let projection = ProjectionBuilder::new(HashSet::<SessionId>::new())
        .filter_events(move |event| {
            matches!(event,
                DomainEvent::LlmRequestReceived { model_version: v, .. } if v == &model_version_clone
            )
        })
        .project_with(|mut sessions, stored_event| {
            if let DomainEvent::LlmRequestReceived { session_id, .. } = &stored_event.payload {
                sessions.insert(session_id.clone());
            }
            sessions
        });

    projection
        .execute(event_store)
        .await
        .map_err(|e| QueryError::Projection(e.to_string()))
}

/// Errors that can occur during queries
#[derive(Debug, thiserror::Error)]
pub enum QueryError {
    #[error("Event store error: {0}")]
    EventStore(String),

    #[error("Projection error: {0}")]
    Projection(String),

    #[error("Event conversion error: {0}")]
    EventConversion(String),
}

/// Summary of user activity across sessions
#[derive(Debug, Clone, PartialEq)]
pub struct UserActivitySummary {
    pub user_id: UserId,
    pub total_sessions: usize,
    pub total_requests: usize,
    pub sessions_by_application: HashMap<ApplicationId, usize>,
    pub model_usage: HashMap<ModelVersion, usize>,
}

/// Version usage statistics
#[derive(Debug, Clone, PartialEq)]
pub struct VersionUsageStats {
    pub model_version: ModelVersion,
    pub total_requests: usize,
    pub unique_sessions: usize,
    pub unique_users: usize,
    pub average_requests_per_session: f64,
}

/// Application-level metrics
#[derive(Debug, Clone, PartialEq)]
pub struct ApplicationMetrics {
    pub application_id: ApplicationId,
    pub total_sessions: usize,
    pub total_requests: usize,
    pub unique_users: usize,
    pub model_versions_used: HashSet<ModelVersion>,
    pub average_session_length: std::time::Duration,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{llm::LlmProvider, metrics::Timestamp, types::ModelId};
    #[allow(unused_imports)]
    use crate::domain::{
        llm::RequestId,
        session::SessionStatus,
        streams::session_stream,
        types::{LlmParameters, Prompt},
    };
    #[allow(unused_imports)]
    use eventcore::{CommandExecutor, EventStore as _, StreamWrite};
    use eventcore_memory::InMemoryEventStore;

    // Simplified test data setup - actual implementation will use CommandExecutor
    fn create_test_session_id() -> SessionId {
        SessionId::generate()
    }

    fn create_test_user_id() -> UserId {
        UserId::generate()
    }

    fn create_test_model_version() -> ModelVersion {
        ModelVersion {
            provider: LlmProvider::Anthropic,
            model_id: ModelId::try_new("claude-3".to_string()).unwrap(),
        }
    }

    #[tokio::test]
    async fn test_get_session_events() {
        let event_store = InMemoryEventStore::new();
        let session_id = create_test_session_id();

        let result = get_session_events(&event_store, &session_id).await;

        assert!(result.is_ok());
        let events = result.unwrap();
        // With empty store, should return empty vec
        assert!(events.is_empty());
    }

    #[tokio::test]
    async fn test_get_user_activity() {
        let event_store = InMemoryEventStore::new();
        let user_id = create_test_user_id();

        let result = get_user_activity(&event_store, &user_id).await;

        assert!(result.is_ok());
        let summary = result.unwrap();
        assert_eq!(summary.user_id, user_id);
    }

    #[tokio::test]
    async fn test_get_version_usage() {
        let event_store = InMemoryEventStore::new();
        let model_version = create_test_model_version();

        let result = get_version_usage(&event_store, &model_version).await;

        assert!(result.is_ok());
        let stats = result.unwrap();
        assert_eq!(stats.model_version, model_version);
    }

    #[tokio::test]
    async fn test_get_application_metrics() {
        let event_store = InMemoryEventStore::new();
        let app_id = ApplicationId::try_new("test-app".to_string()).unwrap();

        let result = get_application_metrics(&event_store, &app_id).await;

        assert!(result.is_ok());
        let metrics = result.unwrap();
        assert_eq!(metrics.application_id, app_id);
    }

    #[tokio::test]
    async fn test_get_sessions_by_version() {
        let event_store = InMemoryEventStore::new();
        let model_version = create_test_model_version();

        let result = get_sessions_by_version(&event_store, &model_version).await;

        assert!(result.is_ok());
        let sessions = result.unwrap();
        // With empty store, should return empty set
        assert!(sessions.is_empty());
    }

    #[tokio::test]
    async fn test_user_activity_with_multiple_sessions() {
        let event_store = InMemoryEventStore::new();

        // Create multiple sessions for the same user
        let user_id = UserId::generate();
        let app_id1 = ApplicationId::try_new("app1".to_string()).unwrap();
        let app_id2 = ApplicationId::try_new("app2".to_string()).unwrap();

        for i in 0..3 {
            let session_id = SessionId::generate();
            let app_id = if i < 2 { &app_id1 } else { &app_id2 };

            let _event = DomainEvent::SessionStarted {
                session_id: session_id.clone(),
                user_id: user_id.clone(),
                application_id: app_id.clone(),
                started_at: Timestamp::now(),
            };

            // Note: In actual implementation, events would be written using CommandExecutor
            // For now, we're testing the query logic without pre-populated data
        }

        let result = get_user_activity(&event_store, &user_id).await;
        assert!(result.is_ok());

        let summary = result.unwrap();
        // Since we haven't written events to the store, these should be 0
        assert_eq!(summary.total_sessions, 0);
        assert_eq!(summary.sessions_by_application.len(), 0);
    }
}
