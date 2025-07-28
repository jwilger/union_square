//! EventCore integration infrastructure
//!
//! This module provides the integration layer between Union Square and EventCore,
//! including configuration, service wrappers, and projection management.

use nutype::nutype;
use std::time::Duration;

pub mod projections;
pub mod service;

#[nutype(
    validate(len_char_min = 1, len_char_max = 512),
    derive(Debug, Clone, Display, PartialEq, Eq, AsRef)
)]
pub struct ConnectionString(String);

#[nutype(
    validate(greater = 0, less_or_equal = 100),
    derive(Debug, Clone, Copy, PartialEq, Eq, AsRef)
)]
pub struct PoolSize(u32);

#[nutype(
    validate(greater = 0, less_or_equal = 300),
    derive(Debug, Clone, Copy, PartialEq, Eq, AsRef)
)]
pub struct ConnectionTimeoutSeconds(u32);

/// Configuration for EventCore integration
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventCoreConfig {
    pub connection_string: ConnectionString,
    pub pool_size: PoolSize,
    pub connection_timeout: Duration,
}

impl EventCoreConfig {
    /// Try to create a new EventCore configuration with validation
    pub fn try_new(
        connection_string: &str,
        pool_size: u32,
        connection_timeout_seconds: u32,
    ) -> Result<Self, EventCoreConfigError> {
        let connection_string = ConnectionString::try_new(connection_string.to_string())
            .map_err(|e| EventCoreConfigError::InvalidConnectionString(e.to_string()))?;

        let pool_size = PoolSize::try_new(pool_size)
            .map_err(|e| EventCoreConfigError::InvalidPoolSize(e.to_string()))?;

        let timeout_seconds = ConnectionTimeoutSeconds::try_new(connection_timeout_seconds)
            .map_err(|e| EventCoreConfigError::InvalidTimeout(e.to_string()))?;

        Ok(Self {
            connection_string,
            pool_size,
            connection_timeout: Duration::from_secs(*timeout_seconds.as_ref() as u64),
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum EventCoreConfigError {
    #[error("Invalid connection string: {0}")]
    InvalidConnectionString(String),

    #[error("Invalid pool size: {0}")]
    InvalidPoolSize(String),

    #[error("Invalid timeout: {0}")]
    InvalidTimeout(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eventcore_config_creation() {
        // Test valid configuration
        let config = EventCoreConfig::try_new("postgres://localhost/test_db", 10, 30);
        assert!(config.is_ok());
        let config = config.unwrap();
        assert_eq!(*config.pool_size.as_ref(), 10);
        assert_eq!(config.connection_timeout, Duration::from_secs(30));

        // Test invalid connection string (empty)
        let invalid = EventCoreConfig::try_new("", 10, 30);
        assert!(invalid.is_err());
        assert!(matches!(
            invalid.unwrap_err(),
            EventCoreConfigError::InvalidConnectionString(_)
        ));

        // Test invalid pool size (zero)
        let invalid = EventCoreConfig::try_new("postgres://localhost/test_db", 0, 30);
        assert!(invalid.is_err());
        assert!(matches!(
            invalid.unwrap_err(),
            EventCoreConfigError::InvalidPoolSize(_)
        ));

        // Test invalid pool size (too large)
        let invalid = EventCoreConfig::try_new("postgres://localhost/test_db", 101, 30);
        assert!(invalid.is_err());
        assert!(matches!(
            invalid.unwrap_err(),
            EventCoreConfigError::InvalidPoolSize(_)
        ));

        // Test invalid timeout (zero)
        let invalid = EventCoreConfig::try_new("postgres://localhost/test_db", 10, 0);
        assert!(invalid.is_err());
        assert!(matches!(
            invalid.unwrap_err(),
            EventCoreConfigError::InvalidTimeout(_)
        ));
    }
}
