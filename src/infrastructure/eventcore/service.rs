//! EventCore service wrapper
//!
//! This module provides a service wrapper around EventCore's PostgresEventStore
//! for easier integration with the application.

use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Duration;

use eventcore::{CommandLogic, RetryPolicy};
#[cfg(test)]
use eventcore_memory::InMemoryEventStore;
use eventcore_postgres::{MaxConnections, PostgresConfig, PostgresEventStore};
use eventcore_types::{Event, EventStore};

use super::EventCoreConfig;
use crate::domain::events::DomainEvent;
use crate::domain::types::ErrorMessage;
use crate::error::Error;

/// Service wrapper for EventCore functionality
pub struct EventCoreService {
    postgres_store: Option<Arc<PostgresEventStore>>,
    #[cfg(test)]
    memory_store: Option<Arc<InMemoryEventStore>>,
}

impl EventCoreService {
    /// Create a new EventCore service with PostgreSQL backend
    pub async fn new(config: EventCoreConfig) -> crate::error::Result<Self> {
        let pool_size = NonZeroU32::new(*config.pool_size.as_ref())
            .ok_or_else(|| Error::application("Pool size must be non-zero"))?;

        let postgres_config = PostgresConfig {
            max_connections: MaxConnections::new(pool_size),
            acquire_timeout: config.connection_timeout,
            idle_timeout: Duration::from_secs(600),
        };

        let store =
            PostgresEventStore::with_config(config.connection_string.as_ref(), postgres_config)
                .await
                .map_err(|e| eventcore_error(e.to_string()))?;

        Ok(Self {
            postgres_store: Some(Arc::new(store)),
            #[cfg(test)]
            memory_store: None,
        })
    }

    /// Create a new EventCore service with in-memory backend (for testing)
    #[cfg(test)]
    pub fn with_memory_store() -> Self {
        let store = InMemoryEventStore::new();
        Self {
            postgres_store: None,
            memory_store: Some(Arc::new(store)),
        }
    }

    /// Execute a command against the configured event store
    pub async fn execute_command<C>(&self, command: C) -> crate::error::Result<()>
    where
        C: CommandLogic<Event = DomainEvent> + Send + Sync,
    {
        #[cfg(test)]
        if let Some(store) = &self.memory_store {
            eventcore::execute(store.as_ref(), command, RetryPolicy::default())
                .await
                .map_err(|e| eventcore_error(e.to_string()))?;
            return Ok(());
        }

        if let Some(store) = &self.postgres_store {
            eventcore::execute(store.as_ref(), command, RetryPolicy::default())
                .await
                .map_err(|e| eventcore_error(e.to_string()))?;
            return Ok(());
        }

        Err(eventcore_error("No event store configured".to_string()))
    }

    /// Read events from a stream
    pub async fn read_stream<E: Event>(
        &self,
        stream_id: eventcore_types::StreamId,
    ) -> crate::error::Result<eventcore_types::EventStreamReader<E>> {
        #[cfg(test)]
        if let Some(store) = &self.memory_store {
            return store
                .read_stream(stream_id)
                .await
                .map_err(|e| eventcore_error(e.to_string()));
        }

        if let Some(store) = &self.postgres_store {
            return store
                .read_stream(stream_id)
                .await
                .map_err(|e| eventcore_error(e.to_string()));
        }

        Err(eventcore_error("No event store configured".to_string()))
    }

    /// Run database migrations
    pub async fn migrate(&self) -> crate::error::Result<()> {
        if let Some(store) = &self.postgres_store {
            let store = Arc::clone(store);
            tokio::task::spawn(async move {
                store.migrate().await;
            })
            .await
            .map_err(|e| eventcore_error(format!("Migration task failed: {e}")))?;
            Ok(())
        } else {
            Err(eventcore_error(
                "No PostgreSQL store available for migration".to_string(),
            ))
        }
    }
}

/// Convert a string into an EventCore error variant
fn eventcore_error(s: String) -> Error {
    // ErrorMessage rejects empty strings; eventcore never emits them.
    ErrorMessage::try_new(s)
        .map(Error::EventCore)
        .unwrap_or(Error::Internal)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_eventcore_service_creation_fails_for_invalid_host() {
        // Use a 1-second timeout so the test fails fast when the host is unreachable.
        let config = EventCoreConfig::try_new("postgres://invalid_host/test_db", 10, 1).unwrap();

        let service = EventCoreService::new(config).await;
        assert!(service.is_err());
    }

    #[test]
    fn test_eventcore_service_with_memory_store() {
        let _service = EventCoreService::with_memory_store();
    }
}
