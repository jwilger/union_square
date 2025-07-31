//! High-level projection service for production deployments
//!
//! This module provides a convenient service that sets up and manages
//! all projections with proper configuration for production use.

use eventcore::EventStore;
use sqlx::PgPool;
use std::sync::Arc;
use tracing::{error, info};

use crate::domain::events::DomainEvent;

use super::{
    core::InMemoryProjection,
    postgres::PostgresProjectionAdapter,
    query_service::ProjectionQueryService,
    runner::{ProjectionConfig, ProjectionSupervisor},
    session_summary::{
        create_session_summary_projection, SessionSummaryPostgresLogic, SessionSummaryState,
    },
};

/// Configuration for the projection service
#[derive(Clone, Debug)]
pub struct ProjectionServiceConfig {
    /// Database connection pool for persistent projections
    pub pg_pool: Option<PgPool>,
    /// Projection runner configuration
    pub projection_config: ProjectionConfig,
    /// Whether to enable in-memory projections (Tier 1)
    pub enable_in_memory: bool,
    /// Whether to enable PostgreSQL projections (Tier 2)
    pub enable_postgres: bool,
}

impl Default for ProjectionServiceConfig {
    fn default() -> Self {
        Self {
            pg_pool: None,
            projection_config: ProjectionConfig::default(),
            enable_in_memory: true,
            enable_postgres: false,
        }
    }
}

/// High-level service for managing all projections
pub struct ProjectionService<ES>
where
    ES: EventStore<Event = DomainEvent>,
{
    supervisor: Arc<ProjectionSupervisor<ES>>,
    session_query_service:
        Option<Arc<ProjectionQueryService<InMemoryProjection<SessionSummaryState, DomainEvent>>>>,
    session_query_service_pg: Option<
        Arc<
            ProjectionQueryService<
                PostgresProjectionAdapter<SessionSummaryPostgresLogic, DomainEvent>,
            >,
        >,
    >,
}

impl<ES> ProjectionService<ES>
where
    ES: EventStore<Event = DomainEvent> + 'static,
{
    /// Create a new projection service
    pub async fn new(
        event_store: Arc<ES>,
        config: ProjectionServiceConfig,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let supervisor = Arc::new(ProjectionSupervisor::new(event_store.clone()));

        let mut session_query_service = None;
        let mut session_query_service_pg = None;

        // Register in-memory projections (Tier 1 - sub-millisecond queries)
        if config.enable_in_memory {
            info!("Setting up in-memory projections");

            // Session summary projection
            let session_projection = Arc::new(create_session_summary_projection());
            let streams = Self::get_session_related_streams();

            supervisor
                .register_projection(
                    "session_summary_memory".to_string(),
                    session_projection.clone(),
                    streams,
                    config.projection_config.clone(),
                )
                .await;

            session_query_service = Some(Arc::new(ProjectionQueryService::new(session_projection)));

            // TODO: Add more in-memory projections as needed
            // - Active request tracking
            // - Real-time usage metrics
            // - User session counts
        }

        // Register PostgreSQL projections (Tier 2 - interactive queries)
        if config.enable_postgres {
            if let Some(pool) = config.pg_pool {
                info!("Setting up PostgreSQL projections");

                // Session summary projection (PostgreSQL)
                let session_projection_pg = Arc::new(
                    PostgresProjectionAdapter::new(
                        pool.clone(),
                        "session_summary".to_string(),
                        SessionSummaryPostgresLogic,
                    )
                    .await?,
                );

                let streams = Self::get_session_related_streams();

                supervisor
                    .register_projection(
                        "session_summary_postgres".to_string(),
                        session_projection_pg.clone(),
                        streams,
                        config.projection_config.clone(),
                    )
                    .await;

                session_query_service_pg =
                    Some(Arc::new(ProjectionQueryService::new(session_projection_pg)));

                // TODO: Add more PostgreSQL projections
                // - Historical session analysis
                // - User activity tracking
                // - Model usage statistics
                // - Application metrics
            } else {
                error!("PostgreSQL projections enabled but no pool provided");
            }
        }

        Ok(Self {
            supervisor,
            session_query_service,
            session_query_service_pg,
        })
    }

    /// Start all projections
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Starting projection service");
        self.supervisor
            .start()
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    /// Stop all projections gracefully
    pub async fn stop(&self) {
        info!("Stopping projection service");
        self.supervisor.stop().await
    }

    /// Get the in-memory session query service
    pub fn session_queries(
        &self,
    ) -> Option<Arc<ProjectionQueryService<InMemoryProjection<SessionSummaryState, DomainEvent>>>>
    {
        self.session_query_service.clone()
    }

    /// Get the PostgreSQL session query service
    pub fn session_queries_pg(
        &self,
    ) -> Option<
        Arc<
            ProjectionQueryService<
                PostgresProjectionAdapter<SessionSummaryPostgresLogic, DomainEvent>,
            >,
        >,
    > {
        self.session_query_service_pg.clone()
    }

    /// Get health status of all projections
    pub async fn health(&self) -> Vec<super::runner::ProjectionHealth> {
        self.supervisor.get_health().await
    }

    /// Get streams that contain session-related events
    fn get_session_related_streams() -> Vec<eventcore::StreamId> {
        // For session summary projections, we need to subscribe to all session streams.
        // In a real system, you'd have a mechanism to discover active sessions
        // and subscribe to their streams dynamically.
        //
        // For now, we return an empty vector which means the projection will need
        // to be configured with specific streams at runtime, or we need to implement
        // a stream discovery mechanism.
        //
        // EventCore doesn't support wildcard subscriptions like "session:*",
        // so each session stream must be explicitly subscribed to.
        vec![]
    }

    /// Register a custom projection
    pub async fn register_custom_projection<P>(
        &self,
        name: String,
        projection: Arc<P>,
        streams: Vec<eventcore::StreamId>,
        config: ProjectionConfig,
    ) where
        P: super::core::Projection<Event = DomainEvent> + 'static,
        P::Error: std::error::Error + Send + Sync + 'static,
    {
        self.supervisor
            .register_projection(name, projection, streams, config)
            .await
    }
}

/// Builder for creating projection services with fluent API
pub struct ProjectionServiceBuilder<ES>
where
    ES: EventStore<Event = DomainEvent>,
{
    event_store: Arc<ES>,
    config: ProjectionServiceConfig,
}

impl<ES> ProjectionServiceBuilder<ES>
where
    ES: EventStore<Event = DomainEvent> + 'static,
{
    /// Create a new builder
    pub fn new(event_store: Arc<ES>) -> Self {
        Self {
            event_store,
            config: ProjectionServiceConfig::default(),
        }
    }

    /// Set the PostgreSQL pool for persistent projections
    pub fn with_postgres(mut self, pool: PgPool) -> Self {
        self.config.pg_pool = Some(pool);
        self.config.enable_postgres = true;
        self
    }

    /// Enable or disable in-memory projections
    pub fn with_in_memory(mut self, enabled: bool) -> Self {
        self.config.enable_in_memory = enabled;
        self
    }

    /// Set custom projection configuration
    pub fn with_projection_config(mut self, config: ProjectionConfig) -> Self {
        self.config.projection_config = config;
        self
    }

    /// Build the projection service
    pub async fn build(
        self,
    ) -> Result<ProjectionService<ES>, Box<dyn std::error::Error + Send + Sync>> {
        ProjectionService::new(self.event_store, self.config).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eventcore_memory::InMemoryEventStore;

    #[tokio::test]
    async fn test_projection_service_builder() {
        let event_store = Arc::new(InMemoryEventStore::<DomainEvent>::new());

        let service = ProjectionServiceBuilder::new(event_store)
            .with_in_memory(true)
            .build()
            .await
            .unwrap();

        // Should have in-memory query service
        assert!(service.session_queries().is_some());
        // Should not have PostgreSQL query service
        assert!(service.session_queries_pg().is_none());
    }

    #[tokio::test]
    async fn test_projection_service_health() {
        let event_store = Arc::new(InMemoryEventStore::<DomainEvent>::new());

        let service = ProjectionServiceBuilder::new(event_store)
            .with_in_memory(true)
            .build()
            .await
            .unwrap();

        let health = service.health().await;
        // Should have at least one projection registered
        assert!(!health.is_empty());
    }
}
