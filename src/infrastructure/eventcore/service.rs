//! EventCore service wrapper
//!
//! This module provides a service wrapper around EventCore's PostgresEventStore
//! and CommandExecutor for easier integration with the application.

#[cfg(test)]
use eventcore::{CommandExecutor, CommandLogic, ExecutionOptions};
#[cfg(test)]
use std::sync::Arc;

#[cfg(test)]
use eventcore_memory::InMemoryEventStore;

use super::EventCoreConfig;
#[cfg(test)]
use crate::domain::events::DomainEvent;
#[cfg(test)]
use crate::domain::types::ErrorMessage;
use crate::error::Result;

/// Service wrapper for EventCore functionality
pub struct EventCoreService {
    // TODO: Add postgres_executor when implementing PostgreSQL backend
    #[cfg(test)]
    memory_executor: Option<Arc<CommandExecutor<InMemoryEventStore<DomainEvent>>>>,
}

impl EventCoreService {
    /// Create a new EventCore service with PostgreSQL backend
    pub async fn new(_config: EventCoreConfig) -> Result<Self> {
        Ok(Self {
            #[cfg(test)]
            memory_executor: None,
        })
    }

    /// Create a new EventCore service with in-memory backend (for testing)
    #[cfg(test)]
    pub fn with_memory_store() -> Self {
        let event_store = InMemoryEventStore::new();
        let command_executor = Arc::new(CommandExecutor::new(event_store));

        Self {
            memory_executor: Some(command_executor),
        }
    }

    /// Execute a command with in-memory store (for testing)
    #[cfg(test)]
    pub async fn execute_command_memory<C>(&self, command: C) -> Result<()>
    where
        C: CommandLogic<Event = crate::domain::events::DomainEvent> + Send + Sync,
    {
        if let Some(executor) = &self.memory_executor {
            executor
                .execute(command, ExecutionOptions::default())
                .await
                .map_err(|e| {
                    crate::error::Error::EventCore(ErrorMessage::try_new(e.to_string()).unwrap())
                })?;
            Ok(())
        } else {
            Err(crate::error::Error::EventCore(
                ErrorMessage::try_new("No memory executor available".to_string()).unwrap(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_eventcore_service_creation() {
        let config = EventCoreConfig::try_new("postgres://localhost/test_db", 10, 30).unwrap();

        let service = EventCoreService::new(config).await;
        assert!(service.is_ok());
    }

    #[test]
    fn test_eventcore_service_with_memory_store() {
        let _service = EventCoreService::with_memory_store();
        // Service should be created successfully
        // Just checking that it compiles and creates properly
    }
}
