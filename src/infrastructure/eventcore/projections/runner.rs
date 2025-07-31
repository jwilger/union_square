//! Production-ready projection runner with supervision and error recovery
//!
//! This module provides a robust projection runner that can manage multiple
//! projections concurrently, handle errors gracefully, and support proper
//! shutdown semantics for production deployments.

use async_trait::async_trait;
use eventcore::{EventStore, ReadOptions, StoredEvent, StreamId, Timestamp};
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, RwLock};
use tokio::time::{interval, sleep};
use tracing::{error, info, warn};

use super::core::Projection;

/// Configuration for projection runners
#[derive(Clone, Debug)]
pub struct ProjectionConfig {
    /// How often to poll for new events
    pub poll_interval: Duration,
    /// Maximum batch size for processing events
    pub batch_size: usize,
    /// Retry configuration for transient failures
    pub retry_config: RetryConfig,
    /// Whether to rebuild on startup
    pub rebuild_on_startup: bool,
    /// Maximum lag before warning (for monitoring)
    pub max_lag_warning: Duration,
}

impl Default for ProjectionConfig {
    fn default() -> Self {
        Self {
            poll_interval: Duration::from_millis(100),
            batch_size: 1000,
            retry_config: RetryConfig::default(),
            rebuild_on_startup: false,
            max_lag_warning: Duration::from_secs(60),
        }
    }
}

/// Retry configuration for handling transient failures
#[derive(Clone, Debug)]
pub struct RetryConfig {
    /// Maximum number of retries
    pub max_retries: u32,
    /// Initial retry delay
    pub initial_delay: Duration,
    /// Maximum retry delay
    pub max_delay: Duration,
    /// Exponential backoff factor
    pub backoff_factor: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 5,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            backoff_factor: 2.0,
        }
    }
}

/// Health status of a projection
#[derive(Clone, Debug)]
pub struct ProjectionHealth {
    pub name: String,
    pub status: HealthStatus,
    pub last_checkpoint: Option<Timestamp>,
    pub events_processed: u64,
    pub last_error: Option<String>,
    pub lag: Option<Duration>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Lagging,
    Failed,
    Rebuilding,
}

/// Manages multiple projection runners with supervision
pub struct ProjectionSupervisor<ES>
where
    ES: EventStore,
{
    event_store: Arc<ES>,
    projections: Arc<RwLock<HashMap<String, ProjectionEntry<ES>>>>,
    shutdown_tx: broadcast::Sender<()>,
    health: Arc<RwLock<HashMap<String, ProjectionHealth>>>,
}

struct ProjectionEntry<ES>
where
    ES: EventStore,
{
    projection: Arc<
        dyn Projection<
            State = Box<dyn Any + Send + Sync>,
            Event = ES::Event,
            Error = std::io::Error,
        >,
    >,
    streams: Vec<StreamId>,
    config: ProjectionConfig,
}

impl<ES> ProjectionSupervisor<ES>
where
    ES: EventStore + 'static,
    ES::Event: Send + Sync + 'static,
{
    /// Create a new projection supervisor
    pub fn new(event_store: Arc<ES>) -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);
        Self {
            event_store,
            projections: Arc::new(RwLock::new(HashMap::new())),
            shutdown_tx,
            health: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a projection with the supervisor
    pub async fn register_projection<P>(
        &self,
        name: String,
        projection: Arc<P>,
        streams: Vec<StreamId>,
        config: ProjectionConfig,
    ) where
        P: Projection<Event = ES::Event> + 'static,
        P::Error: std::error::Error + Send + Sync + 'static,
    {
        // Type-erase the projection for storage
        let erased_projection = Arc::new(TypeErasedProjection::new(projection));

        let entry = ProjectionEntry {
            projection: erased_projection
                as Arc<
                    dyn Projection<
                        State = Box<dyn Any + Send + Sync>,
                        Event = ES::Event,
                        Error = std::io::Error,
                    >,
                >,
            streams,
            config,
        };

        self.projections.write().await.insert(name.clone(), entry);

        // Initialize health status
        let health = ProjectionHealth {
            name: name.clone(),
            status: HealthStatus::Healthy,
            last_checkpoint: None,
            events_processed: 0,
            last_error: None,
            lag: None,
        };
        self.health.write().await.insert(name, health);
    }

    /// Start all registered projections
    pub async fn start(&self) -> Result<(), std::io::Error> {
        let projections = self.projections.read().await;

        for (name, entry) in projections.iter() {
            let name = name.clone();
            let projection = entry.projection.clone();
            let streams = entry.streams.clone();
            let config = entry.config.clone();
            let event_store = self.event_store.clone();
            let mut shutdown_rx = self.shutdown_tx.subscribe();
            let health = self.health.clone();

            // Spawn a task for each projection
            tokio::spawn(async move {
                let runner = RobustProjectionRunner::new(
                    name.clone(),
                    event_store,
                    projection,
                    streams,
                    config,
                    health,
                );

                tokio::select! {
                    result = runner.run() => {
                        if let Err(e) = result {
                            error!("Projection {} failed: {}", name, e);
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        info!("Projection {} shutting down", name);
                    }
                }
            });
        }

        Ok(())
    }

    /// Stop all projections gracefully
    pub async fn stop(&self) {
        info!("Stopping projection supervisor");
        let _ = self.shutdown_tx.send(());
    }

    /// Get health status of all projections
    pub async fn get_health(&self) -> Vec<ProjectionHealth> {
        self.health.read().await.values().cloned().collect()
    }

    /// Get health status of a specific projection
    pub async fn get_projection_health(&self, name: &str) -> Option<ProjectionHealth> {
        self.health.read().await.get(name).cloned()
    }
}

/// A robust projection runner with error recovery and monitoring
struct RobustProjectionRunner<ES>
where
    ES: EventStore,
{
    name: String,
    event_store: Arc<ES>,
    projection: Arc<
        dyn Projection<
            State = Box<dyn Any + Send + Sync>,
            Event = ES::Event,
            Error = std::io::Error,
        >,
    >,
    streams: Vec<StreamId>,
    config: ProjectionConfig,
    health: Arc<RwLock<HashMap<String, ProjectionHealth>>>,
}

impl<ES> RobustProjectionRunner<ES>
where
    ES: EventStore,
    ES::Event: Send + Sync,
{
    fn new(
        name: String,
        event_store: Arc<ES>,
        projection: Arc<
            dyn Projection<
                State = Box<dyn Any + Send + Sync>,
                Event = ES::Event,
                Error = std::io::Error,
            >,
        >,
        streams: Vec<StreamId>,
        config: ProjectionConfig,
        health: Arc<RwLock<HashMap<String, ProjectionHealth>>>,
    ) -> Self {
        Self {
            name,
            event_store,
            projection,
            streams,
            config,
            health,
        }
    }

    async fn run(&self) -> Result<(), std::io::Error> {
        // Optionally rebuild on startup
        if self.config.rebuild_on_startup {
            self.update_health(HealthStatus::Rebuilding, None).await;
            if let Err(e) = self.rebuild().await {
                error!("Failed to rebuild projection {}: {}", self.name, e);
                self.update_health(HealthStatus::Failed, Some(e.to_string()))
                    .await;
                return Err(e);
            }
        }

        // Main polling loop
        let mut ticker = interval(self.config.poll_interval);
        let mut consecutive_failures = 0;

        loop {
            ticker.tick().await;

            match self.process_batch().await {
                Ok(events_processed) => {
                    consecutive_failures = 0;
                    if events_processed > 0 {
                        info!(
                            "Projection {} processed {} events",
                            self.name, events_processed
                        );
                    }
                    self.update_health(HealthStatus::Healthy, None).await;
                }
                Err(e) => {
                    consecutive_failures += 1;
                    warn!(
                        "Projection {} error (attempt {}): {}",
                        self.name, consecutive_failures, e
                    );

                    if consecutive_failures >= self.config.retry_config.max_retries {
                        error!(
                            "Projection {} failed after {} attempts",
                            self.name, consecutive_failures
                        );
                        self.update_health(HealthStatus::Failed, Some(e.to_string()))
                            .await;
                        return Err(e);
                    }

                    // Exponential backoff
                    let delay = self.calculate_retry_delay(consecutive_failures);
                    sleep(delay).await;
                }
            }
        }
    }

    async fn process_batch(&self) -> Result<u64, std::io::Error> {
        // Get last checkpoint
        let last_checkpoint = self.projection.last_checkpoint().await?;

        // Read new events since checkpoint
        let read_options = ReadOptions::default(); // Limited by EventCore API
        let stream_result = self
            .event_store
            .read_streams(&self.streams, &read_options)
            .await
            .map_err(|e| std::io::Error::other(e.to_string()))?;

        let mut events_processed = 0;
        let mut last_timestamp = None;
        let batch_start = Instant::now();

        for event in stream_result
            .events
            .into_iter()
            .take(self.config.batch_size)
        {
            // Skip events we've already processed
            if let Some(checkpoint) = last_checkpoint {
                if event.timestamp <= checkpoint {
                    continue;
                }
            }

            self.projection.apply_event(&event).await?;
            events_processed += 1;
            last_timestamp = Some(event.timestamp);
        }

        // Update checkpoint after processing batch
        if let Some(timestamp) = last_timestamp {
            self.projection.set_checkpoint(timestamp).await?;

            // Calculate lag (simplified - would need proper timestamp handling)
            let processing_time = batch_start.elapsed();
            if processing_time > self.config.max_lag_warning {
                warn!(
                    "Projection {} is lagging: took {:?} to process {} events",
                    self.name, processing_time, events_processed
                );
                self.update_health(HealthStatus::Lagging, None).await;
            }
        }

        Ok(events_processed)
    }

    async fn rebuild(&self) -> Result<(), std::io::Error> {
        info!("Rebuilding projection {}", self.name);

        // Reset projection
        self.projection.reset().await?;

        // Read all events
        let stream_result = self
            .event_store
            .read_streams(&self.streams, &ReadOptions::default())
            .await
            .map_err(|e| std::io::Error::other(e.to_string()))?;

        // Apply all events in batches
        let total_events = stream_result.events.len();
        let mut processed = 0;
        let mut last_timestamp = None;

        for event in stream_result.events {
            self.projection.apply_event(&event).await?;
            processed += 1;
            last_timestamp = Some(event.timestamp);

            // Log progress
            if processed % 1000 == 0 {
                info!(
                    "Projection {} rebuild progress: {}/{}",
                    self.name, processed, total_events
                );
            }
        }

        // Set final checkpoint
        if let Some(timestamp) = last_timestamp {
            self.projection.set_checkpoint(timestamp).await?;
        }

        info!(
            "Projection {} rebuild complete: {} events processed",
            self.name, processed
        );

        Ok(())
    }

    fn calculate_retry_delay(&self, attempt: u32) -> Duration {
        let delay_ms = (self.config.retry_config.initial_delay.as_millis() as f64
            * self
                .config
                .retry_config
                .backoff_factor
                .powi(attempt as i32 - 1)) as u64;

        Duration::from_millis(delay_ms.min(self.config.retry_config.max_delay.as_millis() as u64))
    }

    async fn update_health(&self, status: HealthStatus, error: Option<String>) {
        let mut health_map = self.health.write().await;
        if let Some(health) = health_map.get_mut(&self.name) {
            health.status = status;
            if let Some(e) = error {
                health.last_error = Some(e);
            }
            if let Ok(checkpoint) = self.projection.last_checkpoint().await {
                health.last_checkpoint = checkpoint;
            }
        }
    }
}

/// Type-erased wrapper for projections
struct TypeErasedProjection<P> {
    inner: Arc<P>,
}

impl<P> TypeErasedProjection<P> {
    fn new(inner: Arc<P>) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl<P> Projection for TypeErasedProjection<P>
where
    P: Projection + 'static,
    P::Error: std::error::Error + Send + Sync + 'static,
    P::State: 'static,
{
    type State = Box<dyn Any + Send + Sync>;
    type Event = P::Event;
    type Error = std::io::Error; // Use a concrete error type for type erasure

    async fn get_state(&self) -> Result<Self::State, Self::Error> {
        self.inner
            .get_state()
            .await
            .map(|state| Box::new(state) as Box<dyn Any + Send + Sync>)
            .map_err(|e| std::io::Error::other(e.to_string()))
    }

    async fn apply_event(&self, event: &StoredEvent<Self::Event>) -> Result<(), Self::Error> {
        self.inner
            .apply_event(event)
            .await
            .map_err(|e| std::io::Error::other(e.to_string()))
    }

    async fn last_checkpoint(&self) -> Result<Option<Timestamp>, Self::Error> {
        self.inner
            .last_checkpoint()
            .await
            .map_err(|e| std::io::Error::other(e.to_string()))
    }

    async fn set_checkpoint(&self, timestamp: Timestamp) -> Result<(), Self::Error> {
        self.inner
            .set_checkpoint(timestamp)
            .await
            .map_err(|e| std::io::Error::other(e.to_string()))
    }

    async fn reset(&self) -> Result<(), Self::Error> {
        self.inner
            .reset()
            .await
            .map_err(|e| std::io::Error::other(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::events::DomainEvent;
    use crate::infrastructure::eventcore::projections::core::InMemoryProjection;
    use crate::infrastructure::eventcore::projections::session_summary::SessionSummaryState;

    #[tokio::test]
    async fn test_retry_delay_calculation() {
        let config = ProjectionConfig::default();
        let runner = RobustProjectionRunner::<eventcore_memory::InMemoryEventStore<DomainEvent>> {
            name: "test".to_string(),
            event_store: Arc::new(eventcore_memory::InMemoryEventStore::new()),
            projection: Arc::new(TypeErasedProjection::new(Arc::new(InMemoryProjection::<
                SessionSummaryState,
                DomainEvent,
            >::new(
                |_, _| {}
            )))),
            streams: vec![],
            config: config.clone(),
            health: Arc::new(RwLock::new(HashMap::new())),
        };

        assert_eq!(
            runner.calculate_retry_delay(1),
            config.retry_config.initial_delay
        );
        assert_eq!(
            runner.calculate_retry_delay(2),
            Duration::from_millis(200) // 100ms * 2
        );
        assert_eq!(
            runner.calculate_retry_delay(3),
            Duration::from_millis(400) // 100ms * 2^2
        );

        // Should be capped at max_delay
        assert_eq!(
            runner.calculate_retry_delay(10),
            config.retry_config.max_delay
        );
    }

    #[tokio::test]
    async fn test_projection_supervisor_health() {
        let event_store = Arc::new(eventcore_memory::InMemoryEventStore::<DomainEvent>::new());
        let supervisor = ProjectionSupervisor::new(event_store);

        // Register a test projection
        let projection = Arc::new(InMemoryProjection::<SessionSummaryState, DomainEvent>::new(
            |_, _| {},
        ));
        supervisor
            .register_projection(
                "test_projection".to_string(),
                projection,
                vec![],
                ProjectionConfig::default(),
            )
            .await;

        // Check initial health
        let health = supervisor.get_projection_health("test_projection").await;
        assert!(health.is_some());
        let health = health.unwrap();
        assert_eq!(health.name, "test_projection");
        assert_eq!(health.status, HealthStatus::Healthy);
        assert_eq!(health.events_processed, 0);
    }
}
