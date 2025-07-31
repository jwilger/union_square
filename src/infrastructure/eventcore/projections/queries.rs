//! Common query patterns for multi-stream projections
//!
//! TODO: This module needs to be refactored to use EventCore's built-in projection system
//! instead of our custom ProjectionBuilder that was removed.
//!
//! The query functions below contain valuable domain logic for aggregating events
//! across streams, but need to be reimplemented using EventCore's native APIs.

use crate::domain::{
    events::DomainEvent,
    llm::ModelVersion,
    session::{ApplicationId, SessionId},
    user::UserId,
};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Summary of user activity across all sessions
#[derive(Debug, Clone)]
pub struct UserActivitySummary {
    pub user_id: UserId,
    pub total_sessions: usize,
    pub total_requests: usize,
    pub total_tokens_used: usize,
    pub active_sessions: Vec<SessionId>,
    pub recent_sessions: Vec<SessionId>,
}

/// Summary of application-level metrics
#[derive(Debug, Clone)]
pub struct ApplicationMetrics {
    pub application_id: ApplicationId,
    pub total_sessions: usize,
    pub total_requests: usize,
    pub total_tokens_used: usize,
    pub unique_users: usize,
    pub average_session_duration: std::time::Duration,
}

/// Trait for accessing materialized projection data
/// This allows us to abstract over different query service implementations
pub trait ProjectionQueryProvider: Send + Sync {
    /// Get all events for a session
    fn get_session_events<'a>(
        &'a self,
        session_id: &'a SessionId,
    ) -> BoxFuture<'a, Result<Vec<DomainEvent>, ServiceQueryError>>;

    /// Get user activity summary
    fn get_user_activity<'a>(
        &'a self,
        user_id: &'a UserId,
    ) -> BoxFuture<'a, Result<ServiceUserActivitySummary, ServiceQueryError>>;

    /// Get application metrics
    fn get_application_metrics<'a>(
        &'a self,
        app_id: &'a ApplicationId,
    ) -> BoxFuture<'a, Result<ServiceApplicationMetrics, ServiceQueryError>>;
}

/// Configuration for query functions to use either materialized projections or fallback to event sourcing
#[allow(dead_code)]
pub struct QueryConfiguration {
    provider: Option<Arc<dyn ProjectionQueryProvider>>,
}

impl QueryConfiguration {
    /// Create a new configuration with a projection provider
    pub fn with_provider(provider: Arc<dyn ProjectionQueryProvider>) -> Self {
        Self {
            provider: Some(provider),
        }
    }

    /// Create a configuration that will always use event sourcing
    pub fn event_sourcing_only() -> Self {
        Self { provider: None }
    }
}

impl Default for QueryConfiguration {
    fn default() -> Self {
        Self::event_sourcing_only()
    }
}

// Service-level types for decoupling
pub type ServiceUserActivitySummary = UserActivitySummary;
pub type ServiceApplicationMetrics = ApplicationMetrics;

#[derive(Debug, thiserror::Error)]
pub enum ServiceQueryError {
    #[error("Event store error: {0}")]
    EventStore(String),
    #[error("Projection not found")]
    ProjectionNotFound,
}

// TODO: The functions below need to be reimplemented using EventCore's projection system
// For now, they are commented out to allow compilation

/*
/// Get all events for a session as a projection
pub async fn get_session_events<ES>(
    event_store: &ES,
    session_id: &SessionId,
    config: &QueryConfiguration,
) -> Result<Vec<DomainEvent>, Box<dyn std::error::Error + Send + Sync>>
where
    ES: EventStore<Event = DomainEvent>,
{
    // Check if we have a materialized projection available
    if let Some(provider) = &config.provider {
        match provider.get_session_events(session_id).await {
            Ok(events) => return Ok(events),
            Err(_) => {
                // Fall back to event sourcing if projection fails
            }
        }
    }

    // TODO: Reimplement using EventCore's native event reading
    todo!("Reimplement using EventCore's projection system")
}

/// Get user activity across all their sessions
pub async fn get_user_activity<ES>(
    event_store: &ES,
    user_id: &UserId,
    config: &QueryConfiguration,
) -> Result<UserActivitySummary, Box<dyn std::error::Error + Send + Sync>>
where
    ES: EventStore<Event = DomainEvent>,
{
    // Check if we have a materialized projection available
    if let Some(provider) = &config.provider {
        match provider.get_user_activity(user_id).await {
            Ok(summary) => return Ok(summary),
            Err(_) => {
                // Fall back to event sourcing if projection fails
            }
        }
    }

    // TODO: Reimplement using EventCore's native projection system
    todo!("Reimplement using EventCore's projection system")
}

/// Get usage statistics for a specific model version
pub async fn get_version_usage<ES>(
    event_store: &ES,
    model_version: &ModelVersion,
) -> Result<VersionUsageStats, Box<dyn std::error::Error + Send + Sync>>
where
    ES: EventStore<Event = DomainEvent>,
{
    // TODO: Reimplement using EventCore's projection system
    todo!("Reimplement using EventCore's projection system")
}

/// Get application-level metrics
pub async fn get_application_metrics<ES>(
    event_store: &ES,
    application_id: &ApplicationId,
    config: &QueryConfiguration,
) -> Result<ApplicationMetrics, Box<dyn std::error::Error + Send + Sync>>
where
    ES: EventStore<Event = DomainEvent>,
{
    // Check if we have a materialized projection available
    if let Some(provider) = &config.provider {
        match provider.get_application_metrics(application_id).await {
            Ok(metrics) => return Ok(metrics),
            Err(_) => {
                // Fall back to event sourcing if projection fails
            }
        }
    }

    // TODO: Reimplement using EventCore's projection system
    todo!("Reimplement using EventCore's projection system")
}

/// Get all sessions that have used a specific model version
pub async fn get_sessions_by_model_version<ES>(
    event_store: &ES,
    model_version: &ModelVersion,
) -> Result<HashSet<SessionId>, Box<dyn std::error::Error + Send + Sync>>
where
    ES: EventStore<Event = DomainEvent>,
{
    // TODO: Reimplement using EventCore's projection system
    todo!("Reimplement using EventCore's projection system")
}
*/

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
    // TODO: Tests need to be rewritten once we implement EventCore's Projection trait
}
