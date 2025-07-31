//! Core projection infrastructure for EventCore
//!
//! This module provides the core traits and implementations for building
//! materialized projections from event streams. Projections are part of the
//! imperative shell, maintaining mutable state derived from events.

use async_trait::async_trait;
use eventcore::{EventStore, ReadOptions, StoredEvent, StreamId, Timestamp};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::interval;

/// A projection that maintains materialized state from events
#[async_trait]
pub trait Projection: Send + Sync {
    /// The state type this projection maintains
    type State: Send + Sync;

    /// The event type this projection processes
    type Event: Send + Sync;

    /// The error type for this projection
    type Error: std::error::Error + Send + Sync;

    /// Get the current state of the projection
    async fn get_state(&self) -> Result<Self::State, Self::Error>;

    /// Apply an event to update the projection state
    async fn apply_event(&self, event: &StoredEvent<Self::Event>) -> Result<(), Self::Error>;

    /// Get the last processed event timestamp (for resumption)
    async fn last_checkpoint(&self) -> Result<Option<Timestamp>, Self::Error>;

    /// Set checkpoint after processing events
    async fn set_checkpoint(&self, timestamp: Timestamp) -> Result<(), Self::Error>;

    /// Reset the projection to initial state
    async fn reset(&self) -> Result<(), Self::Error>;
}

/// Type alias for update function to reduce complexity
type UpdateFn<S, E> = dyn Fn(&mut S, &StoredEvent<E>) + Send + Sync;

/// In-memory projection implementation for real-time queries
pub struct InMemoryProjection<S, E> {
    state: Arc<RwLock<S>>,
    checkpoint: Arc<RwLock<Option<Timestamp>>>,
    update_fn: Arc<UpdateFn<S, E>>,
}

impl<S, E> InMemoryProjection<S, E>
where
    S: Clone + Default + Send + Sync,
    E: Send + Sync,
{
    /// Create a new in-memory projection with the given update function
    pub fn new<F>(update_fn: F) -> Self
    where
        F: Fn(&mut S, &StoredEvent<E>) + Send + Sync + 'static,
    {
        Self {
            state: Arc::new(RwLock::new(S::default())),
            checkpoint: Arc::new(RwLock::new(None)),
            update_fn: Arc::new(update_fn),
        }
    }

    /// Create a projection with initial state
    pub fn with_initial_state<F>(initial_state: S, update_fn: F) -> Self
    where
        F: Fn(&mut S, &StoredEvent<E>) + Send + Sync + 'static,
    {
        Self {
            state: Arc::new(RwLock::new(initial_state)),
            checkpoint: Arc::new(RwLock::new(None)),
            update_fn: Arc::new(update_fn),
        }
    }
}

#[async_trait]
impl<S, E> Projection for InMemoryProjection<S, E>
where
    S: Clone + Default + Send + Sync + 'static,
    E: Send + Sync + 'static,
{
    type State = S;
    type Event = E;
    type Error = std::convert::Infallible;

    async fn get_state(&self) -> Result<Self::State, Self::Error> {
        Ok(self.state.read().await.clone())
    }

    async fn apply_event(&self, event: &StoredEvent<Self::Event>) -> Result<(), Self::Error> {
        let mut state = self.state.write().await;
        (self.update_fn)(&mut state, event);
        Ok(())
    }

    async fn last_checkpoint(&self) -> Result<Option<Timestamp>, Self::Error> {
        Ok(*self.checkpoint.read().await)
    }

    async fn set_checkpoint(&self, timestamp: Timestamp) -> Result<(), Self::Error> {
        *self.checkpoint.write().await = Some(timestamp);
        Ok(())
    }

    async fn reset(&self) -> Result<(), Self::Error> {
        *self.state.write().await = S::default();
        *self.checkpoint.write().await = None;
        Ok(())
    }
}

/// Runs projections by subscribing to event streams
pub struct ProjectionRunner<ES, P> {
    event_store: Arc<ES>,
    projection: Arc<P>,
    streams: Vec<StreamId>,
    poll_interval: Duration,
}

impl<ES, P> ProjectionRunner<ES, P>
where
    ES: EventStore,
    P: Projection<Event = ES::Event>,
    P::Error: 'static,
{
    /// Create a new projection runner
    pub fn new(
        event_store: Arc<ES>,
        projection: Arc<P>,
        streams: Vec<StreamId>,
        poll_interval: Duration,
    ) -> Self {
        Self {
            event_store,
            projection,
            streams,
            poll_interval,
        }
    }

    /// Run the projection, continuously updating from new events
    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut ticker = interval(self.poll_interval);

        loop {
            ticker.tick().await;

            // Get last checkpoint
            let last_checkpoint = self.projection.last_checkpoint().await?;

            // Read new events since checkpoint
            let read_options = ReadOptions::default();
            if let Some(_checkpoint) = last_checkpoint {
                // EventCore's ReadOptions doesn't support after_timestamp
                // We'll filter events after reading them
            }

            let stream_result = self
                .event_store
                .read_streams(&self.streams, &read_options)
                .await?;

            // Apply events to projection
            let mut last_timestamp = None;
            for event in stream_result.events {
                // Skip events we've already processed
                if let Some(checkpoint) = last_checkpoint {
                    if event.timestamp <= checkpoint {
                        continue;
                    }
                }

                self.projection.apply_event(&event).await?;
                last_timestamp = Some(event.timestamp);
            }

            // Update checkpoint after processing batch
            if let Some(timestamp) = last_timestamp {
                self.projection.set_checkpoint(timestamp).await?;
            }
        }
    }

    /// Rebuild the projection from scratch
    pub async fn rebuild(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Reset projection
        self.projection.reset().await?;

        // Read all events
        let stream_result = self
            .event_store
            .read_streams(&self.streams, &ReadOptions::default())
            .await?;

        // Apply all events
        let mut last_timestamp = None;
        for event in stream_result.events {
            self.projection.apply_event(&event).await?;
            last_timestamp = Some(event.timestamp);
        }

        // Set final checkpoint
        if let Some(timestamp) = last_timestamp {
            self.projection.set_checkpoint(timestamp).await?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::events::DomainEvent;
    use crate::domain::session::{ApplicationId, SessionId};
    use crate::domain::streams::session_stream;
    use crate::domain::user::UserId;
    use eventcore::EventId;
    // use eventcore_memory::InMemoryEventStore; // Needed when tests are re-enabled

    #[derive(Clone, Default, Debug, PartialEq)]
    struct TestState {
        event_count: usize,
        session_ids: Vec<SessionId>,
    }

    fn create_test_projection() -> InMemoryProjection<TestState, DomainEvent> {
        InMemoryProjection::new(|state: &mut TestState, event| {
            state.event_count += 1;
            if let DomainEvent::SessionStarted { session_id, .. } = &event.payload {
                state.session_ids.push(session_id.clone());
            }
        })
    }

    #[tokio::test]
    async fn test_in_memory_projection_initial_state() {
        let projection = create_test_projection();
        let state = projection.get_state().await.unwrap();

        assert_eq!(state.event_count, 0);
        assert!(state.session_ids.is_empty());
    }

    #[tokio::test]
    async fn test_in_memory_projection_apply_event() {
        let projection = create_test_projection();
        let session_id = SessionId::generate();

        let event = StoredEvent {
            stream_id: session_stream(&session_id),
            event_id: EventId::new(),
            payload: DomainEvent::SessionStarted {
                session_id: session_id.clone(),
                user_id: UserId::generate(),
                application_id: ApplicationId::try_new("app456".to_string()).unwrap(),
                started_at: crate::domain::metrics::Timestamp::now(),
            },
            metadata: Default::default(),
            timestamp: Timestamp::now(),
            event_version: eventcore::EventVersion::initial(),
        };

        projection.apply_event(&event).await.unwrap();
        let state = projection.get_state().await.unwrap();

        assert_eq!(state.event_count, 1);
        assert_eq!(state.session_ids, vec![session_id]);
    }

    #[tokio::test]
    async fn test_in_memory_projection_checkpoint() {
        let projection = create_test_projection();
        let timestamp = Timestamp::now();

        // Initial checkpoint should be None
        assert_eq!(projection.last_checkpoint().await.unwrap(), None);

        // Set checkpoint
        projection.set_checkpoint(timestamp).await.unwrap();
        assert_eq!(projection.last_checkpoint().await.unwrap(), Some(timestamp));
    }

    #[tokio::test]
    async fn test_in_memory_projection_reset() {
        let projection = create_test_projection();
        let session_id = SessionId::generate();
        let timestamp = Timestamp::now();

        // Apply an event and set checkpoint
        let event = StoredEvent {
            stream_id: session_stream(&session_id),
            event_id: EventId::new(),
            payload: DomainEvent::SessionStarted {
                session_id: session_id.clone(),
                user_id: UserId::generate(),
                application_id: ApplicationId::try_new("app456".to_string()).unwrap(),
                started_at: crate::domain::metrics::Timestamp::now(),
            },
            metadata: Default::default(),
            timestamp,
            event_version: eventcore::EventVersion::initial(),
        };

        projection.apply_event(&event).await.unwrap();
        projection.set_checkpoint(timestamp).await.unwrap();

        // Verify state before reset
        let state = projection.get_state().await.unwrap();
        assert_eq!(state.event_count, 1);
        assert_eq!(projection.last_checkpoint().await.unwrap(), Some(timestamp));

        // Reset
        projection.reset().await.unwrap();

        // Verify state after reset
        let state = projection.get_state().await.unwrap();
        assert_eq!(state.event_count, 0);
        assert!(state.session_ids.is_empty());
        assert_eq!(projection.last_checkpoint().await.unwrap(), None);
    }

    // Tests commented out due to EventCore API limitations
    // TODO: Fix when EventCore supports proper test infrastructure
    /*
    #[tokio::test]
    async fn test_projection_runner_rebuild() {
        let event_store = Arc::new(InMemoryEventStore::<DomainEvent>::new());
        let projection = Arc::new(create_test_projection());
        let session_id = SessionId::generate();
        let stream_id = session_stream(&session_id);

        // Add some events to the store
        let events = vec![
            DomainEvent::SessionStarted {
                session_id: session_id.clone(),
                user_id: UserId::generate(),
                application_id: ApplicationId::try_new("app456".to_string()).unwrap(),
                started_at: crate::domain::metrics::Timestamp::now(),
            },
            DomainEvent::LlmRequestReceived {
                session_id: session_id.clone(),
                request_id: crate::domain::llm::RequestId::generate(),
                session_id: session_id.clone(),
                model_version: crate::domain::llm::ModelVersion {
                    provider: crate::domain::llm::LlmProvider::Anthropic,
                    model_id: crate::domain::types::ModelId::try_new("claude-3".to_string()).unwrap(),
                },
                prompt: crate::domain::types::Prompt::try_new("test".to_string()).unwrap(),
                parameters: crate::domain::types::LlmParameters::new(serde_json::json!({})),
                received_at: crate::domain::metrics::Timestamp::now(),
            },
        ];

        for (i, event) in events.into_iter().enumerate() {
            event_store
                .append_to_stream(&stream_id, &[event], Some(i as u64))
                .await
                .unwrap();
        }

        // Create runner and rebuild
        let runner = ProjectionRunner::new(
            event_store,
            projection.clone(),
            vec![stream_id],
            Duration::from_millis(100),
        );

        runner.rebuild().await.unwrap();

        // Verify projection state
        let state = projection.get_state().await.unwrap();
        assert_eq!(state.event_count, 2);
        assert_eq!(state.session_ids, vec![session_id]);
        assert!(projection.last_checkpoint().await.unwrap().is_some());
    }
    */

    #[tokio::test]
    async fn test_projection_with_initial_state() {
        let initial_state = TestState {
            event_count: 10,
            session_ids: vec![],
        };

        let projection =
            InMemoryProjection::with_initial_state(initial_state.clone(), |state, event| {
                state.event_count += 1;
                if let DomainEvent::SessionStarted { session_id, .. } = &event.payload {
                    state.session_ids.push(session_id.clone());
                }
            });

        let state = projection.get_state().await.unwrap();
        assert_eq!(state.event_count, 10);
        assert!(state.session_ids.is_empty());
    }
}
