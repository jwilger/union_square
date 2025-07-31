//! Query service for EventCore projections
//!
//! This module provides a type-safe query interface for accessing materialized
//! projection data. It works with EventCore's event store to maintain and
//! query projections efficiently.

use crate::domain::{
    events::DomainEvent,
    llm::ModelVersion,
    session::{ApplicationId, SessionId, SessionStatus},
    user::UserId,
};
use thiserror::Error;

use super::session_summary::{ProjectionError, SessionSummary, SessionSummaryProjection};

/// Errors that can occur during query operations
#[derive(Error, Debug)]
pub enum QueryError {
    #[error("Projection error: {0}")]
    Projection(#[from] ProjectionError),
    #[error("Session not found: {0}")]
    SessionNotFound(SessionId),
    #[error("Invalid query parameter: {0}")]
    InvalidParameter(String),
}

/// Summary of user activity across all sessions
#[derive(Debug, Clone)]
pub struct UserActivitySummary {
    pub user_id: UserId,
    pub total_sessions: usize,
    pub total_requests: usize,
    pub active_sessions: Vec<SessionId>,
    pub recent_sessions: Vec<SessionId>,
}

/// Summary of application-level metrics
#[derive(Debug, Clone)]
pub struct ApplicationMetrics {
    pub application_id: ApplicationId,
    pub total_sessions: usize,
    pub total_requests: usize,
    pub unique_users: usize,
}

/// Query service for materialized projections
///
/// This service provides type-safe access to projection data,
/// handling the complexity of managing projection state and queries.
pub struct ProjectionQueryService {
    /// Session summary projection
    session_projection: SessionSummaryProjection,
}

impl Default for ProjectionQueryService {
    fn default() -> Self {
        Self::new()
    }
}

impl ProjectionQueryService {
    /// Create a new query service
    pub fn new() -> Self {
        Self {
            session_projection: SessionSummaryProjection::new(),
        }
    }

    /// Apply events to rebuild projections
    pub fn apply_events(
        &mut self,
        events: Vec<eventcore::StoredEvent<DomainEvent>>,
    ) -> Result<(), QueryError> {
        self.session_projection.apply_events(events)?;
        Ok(())
    }

    /// Get all sessions for a specific user
    pub fn get_user_sessions(&self, user_id: &UserId) -> Vec<&SessionSummary> {
        self.session_projection
            .state()
            .user_sessions(&user_id.to_string())
    }

    /// Get all sessions for a specific application
    pub fn get_app_sessions(&self, app_id: &ApplicationId) -> Vec<&SessionSummary> {
        self.session_projection
            .state()
            .app_sessions(app_id.as_ref())
    }

    /// Get all active sessions
    pub fn get_active_sessions(&self) -> Vec<&SessionSummary> {
        self.session_projection.state().active_sessions()
    }

    /// Get a specific session by ID
    pub fn get_session(&self, session_id: &SessionId) -> Result<&SessionSummary, QueryError> {
        self.session_projection
            .state()
            .get_session(session_id)
            .ok_or_else(|| QueryError::SessionNotFound(session_id.clone()))
    }

    /// Get user activity summary
    pub fn get_user_activity(&self, user_id: &UserId) -> UserActivitySummary {
        let sessions = self.get_user_sessions(user_id);

        let active_sessions: Vec<SessionId> = sessions
            .iter()
            .filter(|s| matches!(s.status, SessionStatus::Active))
            .map(|s| s.session_id.clone())
            .collect();

        let recent_sessions: Vec<SessionId> =
            sessions.iter().map(|s| s.session_id.clone()).collect();

        let total_requests: usize = sessions.iter().map(|s| s.request_count).sum();

        UserActivitySummary {
            user_id: user_id.clone(),
            total_sessions: sessions.len(),
            total_requests,
            active_sessions,
            recent_sessions,
        }
    }

    /// Get application metrics
    pub fn get_application_metrics(&self, app_id: &ApplicationId) -> ApplicationMetrics {
        let sessions = self.get_app_sessions(app_id);

        let unique_users: std::collections::HashSet<String> =
            sessions.iter().map(|s| s.user_id.clone()).collect();

        let total_requests: usize = sessions.iter().map(|s| s.request_count).sum();

        ApplicationMetrics {
            application_id: app_id.clone(),
            total_sessions: sessions.len(),
            total_requests,
            unique_users: unique_users.len(),
        }
    }

    /// Get overall system statistics
    pub fn get_system_stats(&self) -> SystemStats {
        let state = self.session_projection.state();

        let total_sessions = state.sessions.len();
        let active_sessions = state.active_sessions().len();
        let total_requests: usize = state.sessions.values().map(|s| s.request_count).sum();

        let unique_users: std::collections::HashSet<String> =
            state.sessions.values().map(|s| s.user_id.clone()).collect();

        let unique_apps: std::collections::HashSet<String> =
            state.sessions.values().map(|s| s.app_id.clone()).collect();

        SystemStats {
            total_sessions,
            active_sessions,
            total_requests,
            unique_users: unique_users.len(),
            unique_applications: unique_apps.len(),
        }
    }
}

/// Overall system statistics
#[derive(Debug, Clone)]
pub struct SystemStats {
    pub total_sessions: usize,
    pub active_sessions: usize,
    pub total_requests: usize,
    pub unique_users: usize,
    pub unique_applications: usize,
}

/// Usage statistics for a model version
#[derive(Debug, Clone)]
pub struct VersionUsageStats {
    pub model_version: ModelVersion,
    pub total_requests: usize,
    pub successful_requests: usize,
    pub failed_requests: usize,
    pub total_tokens_used: usize,
    pub average_response_time: Option<std::time::Duration>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{session::ApplicationId, user::UserId};

    #[test]
    fn test_query_service_creation() {
        let service = ProjectionQueryService::new();
        let stats = service.get_system_stats();

        assert_eq!(stats.total_sessions, 0);
        assert_eq!(stats.active_sessions, 0);
        assert_eq!(stats.total_requests, 0);
    }

    #[test]
    fn test_user_activity_summary_empty() {
        let service = ProjectionQueryService::new();
        let user_id = UserId::generate();

        let activity = service.get_user_activity(&user_id);
        assert_eq!(activity.user_id, user_id);
        assert_eq!(activity.total_sessions, 0);
        assert_eq!(activity.active_sessions.len(), 0);
    }

    #[test]
    fn test_application_metrics_empty() {
        let service = ProjectionQueryService::new();
        let app_id = ApplicationId::try_new("test-app".to_string()).unwrap();

        let metrics = service.get_application_metrics(&app_id);
        assert_eq!(metrics.application_id, app_id);
        assert_eq!(metrics.total_sessions, 0);
        assert_eq!(metrics.unique_users, 0);
    }
}
