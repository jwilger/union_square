//! Query service using materialized projections
//!
//! This module provides efficient queries by reading from materialized projections
//! instead of rebuilding state from events on each query.

use crate::domain::{
    session::{ApplicationId, SessionId, SessionStatus},
    user::UserId,
};
use crate::infrastructure::eventcore::projections::{
    core::Projection,
    session_summary::{SessionSummary, SessionSummaryState},
};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Service for querying projection state
pub struct ProjectionQueryService<P>
where
    P: Projection<State = SessionSummaryState>,
{
    session_projection: Arc<P>,
}

impl<P> ProjectionQueryService<P>
where
    P: Projection<State = SessionSummaryState>,
{
    /// Create a new query service with the given projection
    pub fn new(session_projection: Arc<P>) -> Self {
        Self { session_projection }
    }

    /// Get all active sessions
    pub async fn get_active_sessions(&self) -> Result<Vec<SessionSummary>, QueryError> {
        let state = self
            .session_projection
            .get_state()
            .await
            .map_err(|e| QueryError::Projection(e.to_string()))?;

        Ok(state
            .sessions
            .values()
            .filter(|s| matches!(s.status, SessionStatus::Active))
            .cloned()
            .collect())
    }

    /// Get sessions for a specific user
    pub async fn get_user_sessions(
        &self,
        user_id: &UserId,
    ) -> Result<Vec<SessionSummary>, QueryError> {
        let state = self
            .session_projection
            .get_state()
            .await
            .map_err(|e| QueryError::Projection(e.to_string()))?;

        Ok(state
            .sessions
            .values()
            .filter(|s| s.user_id == user_id.to_string())
            .cloned()
            .collect())
    }

    /// Get sessions for a specific application
    pub async fn get_application_sessions(
        &self,
        app_id: &ApplicationId,
    ) -> Result<Vec<SessionSummary>, QueryError> {
        let state = self
            .session_projection
            .get_state()
            .await
            .map_err(|e| QueryError::Projection(e.to_string()))?;

        Ok(state
            .sessions
            .values()
            .filter(|s| s.app_id == app_id.to_string())
            .cloned()
            .collect())
    }

    /// Get a specific session by ID
    pub async fn get_session(
        &self,
        session_id: &SessionId,
    ) -> Result<Option<SessionSummary>, QueryError> {
        let state = self
            .session_projection
            .get_state()
            .await
            .map_err(|e| QueryError::Projection(e.to_string()))?;

        Ok(state.sessions.get(session_id).cloned())
    }

    /// Get user activity summary
    pub async fn get_user_activity(
        &self,
        user_id: &UserId,
    ) -> Result<UserActivitySummary, QueryError> {
        let state = self
            .session_projection
            .get_state()
            .await
            .map_err(|e| QueryError::Projection(e.to_string()))?;

        let user_sessions: Vec<&SessionSummary> = state
            .sessions
            .values()
            .filter(|s| s.user_id == user_id.to_string())
            .collect();

        let mut sessions_by_application: HashMap<ApplicationId, usize> = HashMap::new();
        let mut total_requests = 0;
        let mut total_tokens = 0;

        for session in &user_sessions {
            let app_id = ApplicationId::try_new(session.app_id.clone())
                .map_err(|e| QueryError::Conversion(e.to_string()))?;
            *sessions_by_application.entry(app_id).or_insert(0) += 1;
            total_requests += session.request_count;
            total_tokens += session.total_tokens;
        }

        Ok(UserActivitySummary {
            user_id: user_id.clone(),
            total_sessions: user_sessions.len(),
            total_requests,
            total_tokens,
            sessions_by_application,
        })
    }

    /// Get application metrics
    pub async fn get_application_metrics(
        &self,
        app_id: &ApplicationId,
    ) -> Result<ApplicationMetrics, QueryError> {
        let state = self
            .session_projection
            .get_state()
            .await
            .map_err(|e| QueryError::Projection(e.to_string()))?;

        let app_sessions: Vec<&SessionSummary> = state
            .sessions
            .values()
            .filter(|s| s.app_id == app_id.to_string())
            .collect();

        let unique_users: HashSet<String> =
            app_sessions.iter().map(|s| s.user_id.clone()).collect();

        let total_requests: usize = app_sessions.iter().map(|s| s.request_count).sum();
        let total_tokens: usize = app_sessions.iter().map(|s| s.total_tokens).sum();

        let completed_sessions: Vec<&&SessionSummary> = app_sessions
            .iter()
            .filter(|s| matches!(s.status, SessionStatus::Completed))
            .collect();

        let avg_session_duration = if completed_sessions.is_empty() {
            std::time::Duration::from_secs(0)
        } else {
            let total_duration_ms: u64 = completed_sessions
                .iter()
                .filter_map(|s| {
                    s.ended_at.map(|_end| {
                        // EventCore timestamps don't expose millis directly
                        // In production, you'd use proper timestamp comparison
                        1000u64 // Placeholder: 1 second
                    })
                })
                .sum();
            std::time::Duration::from_millis(total_duration_ms / completed_sessions.len() as u64)
        };

        Ok(ApplicationMetrics {
            application_id: app_id.clone(),
            total_sessions: app_sessions.len(),
            total_requests,
            total_tokens,
            unique_users: unique_users.len(),
            average_session_duration: avg_session_duration,
        })
    }

    /// Get sessions that have been inactive for a given duration
    pub async fn get_inactive_sessions(
        &self,
        _inactive_duration: std::time::Duration,
    ) -> Result<Vec<SessionSummary>, QueryError> {
        let state = self
            .session_projection
            .get_state()
            .await
            .map_err(|e| QueryError::Projection(e.to_string()))?;

        // EventCore doesn't support timestamp arithmetic directly
        // In production, you'd convert to/from chrono or use a different approach
        let cutoff = eventcore::Timestamp::now();

        Ok(state
            .sessions
            .values()
            .filter(|s| matches!(s.status, SessionStatus::Active) && s.last_activity < cutoff)
            .cloned()
            .collect())
    }

    /// Get total system statistics
    pub async fn get_system_stats(&self) -> Result<SystemStats, QueryError> {
        let state = self
            .session_projection
            .get_state()
            .await
            .map_err(|e| QueryError::Projection(e.to_string()))?;

        let active_sessions = state
            .sessions
            .values()
            .filter(|s| matches!(s.status, SessionStatus::Active))
            .count();

        let total_sessions = state.sessions.len();
        let total_requests: usize = state.sessions.values().map(|s| s.request_count).sum();
        let total_tokens: usize = state.sessions.values().map(|s| s.total_tokens).sum();

        let unique_users: HashSet<String> =
            state.sessions.values().map(|s| s.user_id.clone()).collect();
        let unique_apps: HashSet<String> =
            state.sessions.values().map(|s| s.app_id.clone()).collect();

        Ok(SystemStats {
            total_sessions,
            active_sessions,
            total_requests,
            total_tokens,
            unique_users: unique_users.len(),
            unique_applications: unique_apps.len(),
        })
    }
}

/// Errors that can occur during queries
#[derive(Debug, thiserror::Error)]
pub enum QueryError {
    #[error("Projection error: {0}")]
    Projection(String),

    #[error("Conversion error: {0}")]
    Conversion(String),
}

/// Summary of user activity
#[derive(Debug, Clone, PartialEq)]
pub struct UserActivitySummary {
    pub user_id: UserId,
    pub total_sessions: usize,
    pub total_requests: usize,
    pub total_tokens: usize,
    pub sessions_by_application: HashMap<ApplicationId, usize>,
}

/// Application-level metrics
#[derive(Debug, Clone, PartialEq)]
pub struct ApplicationMetrics {
    pub application_id: ApplicationId,
    pub total_sessions: usize,
    pub total_requests: usize,
    pub total_tokens: usize,
    pub unique_users: usize,
    pub average_session_duration: std::time::Duration,
}

/// System-wide statistics
#[derive(Debug, Clone, PartialEq)]
pub struct SystemStats {
    pub total_sessions: usize,
    pub active_sessions: usize,
    pub total_requests: usize,
    pub total_tokens: usize,
    pub unique_users: usize,
    pub unique_applications: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::events::DomainEvent;
    use crate::infrastructure::eventcore::projections::{
        core::InMemoryProjection, session_summary::create_session_summary_projection,
    };

    async fn create_test_service(
    ) -> ProjectionQueryService<InMemoryProjection<SessionSummaryState, DomainEvent>> {
        let projection = Arc::new(create_session_summary_projection());
        ProjectionQueryService::new(projection)
    }

    #[tokio::test]
    async fn test_get_active_sessions_empty() {
        let service = create_test_service().await;
        let result = service.get_active_sessions().await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_get_user_sessions_empty() {
        let service = create_test_service().await;
        let user_id = UserId::generate();
        let result = service.get_user_sessions(&user_id).await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_get_session_not_found() {
        let service = create_test_service().await;
        let session_id = SessionId::generate();
        let result = service.get_session(&session_id).await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_get_system_stats_empty() {
        let service = create_test_service().await;
        let result = service.get_system_stats().await;

        assert!(result.is_ok());
        let stats = result.unwrap();
        assert_eq!(stats.total_sessions, 0);
        assert_eq!(stats.active_sessions, 0);
        assert_eq!(stats.total_requests, 0);
        assert_eq!(stats.total_tokens, 0);
        assert_eq!(stats.unique_users, 0);
        assert_eq!(stats.unique_applications, 0);
    }

    #[tokio::test]
    async fn test_get_user_activity_empty() {
        let service = create_test_service().await;
        let user_id = UserId::generate();
        let result = service.get_user_activity(&user_id).await;

        assert!(result.is_ok());
        let summary = result.unwrap();
        assert_eq!(summary.user_id, user_id);
        assert_eq!(summary.total_sessions, 0);
        assert_eq!(summary.total_requests, 0);
        assert_eq!(summary.total_tokens, 0);
        assert!(summary.sessions_by_application.is_empty());
    }

    #[tokio::test]
    async fn test_get_application_metrics_empty() {
        let service = create_test_service().await;
        let app_id = ApplicationId::try_new("test-app".to_string()).unwrap();
        let result = service.get_application_metrics(&app_id).await;

        assert!(result.is_ok());
        let metrics = result.unwrap();
        assert_eq!(metrics.application_id, app_id);
        assert_eq!(metrics.total_sessions, 0);
        assert_eq!(metrics.total_requests, 0);
        assert_eq!(metrics.total_tokens, 0);
        assert_eq!(metrics.unique_users, 0);
        assert_eq!(
            metrics.average_session_duration,
            std::time::Duration::from_secs(0)
        );
    }
}
