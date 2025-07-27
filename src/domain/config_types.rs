//! Type-safe configuration types
//!
//! This module provides domain-specific types for configuration values,
//! ensuring validation at boundaries and preventing primitive obsession.

use nutype::nutype;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Host address for network services
#[nutype(
    validate(not_empty, len_char_max = 255),
    derive(
        Debug,
        Clone,
        PartialEq,
        Eq,
        Hash,
        Serialize,
        Deserialize,
        AsRef,
        Display
    )
)]
pub struct Host(String);

impl Default for Host {
    fn default() -> Self {
        Self::try_new("0.0.0.0".to_string()).expect("Default host is valid")
    }
}

/// Port number for network services
#[nutype(
    validate(predicate = |port| (1..=65535).contains(port)),
    derive(
        Debug,
        Clone,
        Copy,
        PartialEq,
        Eq,
        PartialOrd,
        Ord,
        Hash,
        Serialize,
        Deserialize,
        AsRef,
        Display
    )
)]
pub struct Port(u16);

impl Default for Port {
    fn default() -> Self {
        Self::try_new(8080).expect("Default port is valid")
    }
}

/// Database username
#[nutype(
    validate(not_empty, len_char_max = 128),
    derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, AsRef)
)]
pub struct DatabaseUsername(String);

impl fmt::Display for DatabaseUsername {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_ref())
    }
}

/// Database password (secured)
#[nutype(validate(not_empty), derive(Clone, Serialize, Deserialize, AsRef))]
pub struct DatabasePassword(String);

impl fmt::Debug for DatabasePassword {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DatabasePassword(***)")
    }
}

impl fmt::Display for DatabasePassword {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "***")
    }
}

/// Database name
#[nutype(
    validate(not_empty, len_char_max = 128, regex = r"^[a-zA-Z][a-zA-Z0-9_]*$"),
    derive(
        Debug,
        Clone,
        PartialEq,
        Eq,
        Hash,
        Serialize,
        Deserialize,
        AsRef,
        Display
    )
)]
pub struct DatabaseName(String);

/// Maximum number of database connections
#[nutype(
    validate(predicate = |count| *count > 0 && *count <= 1000),
    derive(
        Debug,
        Clone,
        Copy,
        PartialEq,
        Eq,
        PartialOrd,
        Ord,
        Hash,
        Serialize,
        Deserialize,
        AsRef,
        Display
    )
)]
pub struct MaxConnections(u32);

impl Default for MaxConnections {
    fn default() -> Self {
        Self::try_new(10).expect("Default max connections is valid")
    }
}

/// Log level configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum LogLevel {
    Trace,
    Debug,
    #[default]
    Info,
    Warn,
    Error,
}

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Trace => "trace",
            LogLevel::Debug => "debug",
            LogLevel::Info => "info",
            LogLevel::Warn => "warn",
            LogLevel::Error => "error",
        }
    }
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for LogLevel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "trace" => Ok(LogLevel::Trace),
            "debug" => Ok(LogLevel::Debug),
            "info" => Ok(LogLevel::Info),
            "warn" | "warning" => Ok(LogLevel::Warn),
            "error" => Ok(LogLevel::Error),
            _ => Err(format!("Invalid log level: {s}")),
        }
    }
}

/// Log format configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum LogFormat {
    #[default]
    Json,
    Pretty,
    Compact,
}

impl LogFormat {
    pub fn as_str(&self) -> &'static str {
        match self {
            LogFormat::Json => "json",
            LogFormat::Pretty => "pretty",
            LogFormat::Compact => "compact",
        }
    }
}

impl fmt::Display for LogFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for LogFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "json" => Ok(LogFormat::Json),
            "pretty" => Ok(LogFormat::Pretty),
            "compact" => Ok(LogFormat::Compact),
            _ => Err(format!("Invalid log format: {s}")),
        }
    }
}

/// EventCore batch size
#[nutype(
    validate(predicate = |size| *size > 0 && *size <= 10000),
    derive(
        Debug,
        Clone,
        Copy,
        PartialEq,
        Eq,
        PartialOrd,
        Ord,
        Hash,
        Serialize,
        Deserialize,
        AsRef,
        Display
    )
)]
pub struct BatchSize(usize);

impl Default for BatchSize {
    fn default() -> Self {
        Self::try_new(100).expect("Default batch size is valid")
    }
}

/// EventCore flush interval in milliseconds
#[nutype(
    validate(predicate = |ms| *ms > 0 && *ms <= 60000), // max 1 minute
    derive(
        Debug,
        Clone,
        Copy,
        PartialEq,
        Eq,
        PartialOrd,
        Ord,
        Hash,
        Serialize,
        Deserialize,
        AsRef,
        Display
    )
)]
pub struct FlushIntervalMs(u64);

impl Default for FlushIntervalMs {
    fn default() -> Self {
        Self::try_new(1000).expect("Default flush interval is valid")
    }
}

// AwsRegion is already defined in providers::bedrock::types

/// Custom LLM provider name
#[nutype(
    validate(not_empty, len_char_max = 100, regex = r"^[a-zA-Z][a-zA-Z0-9_-]*$"),
    derive(
        Debug,
        Clone,
        PartialEq,
        Eq,
        Hash,
        Serialize,
        Deserialize,
        AsRef,
        Display
    )
)]
pub struct ProviderName(String);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_host_validation() {
        assert!(Host::try_new("localhost".to_string()).is_ok());
        assert!(Host::try_new("0.0.0.0".to_string()).is_ok());
        assert!(Host::try_new("192.168.1.1".to_string()).is_ok());
        assert!(Host::try_new("example.com".to_string()).is_ok());
        assert!(Host::try_new("".to_string()).is_err());
        assert!(Host::try_new("a".repeat(256)).is_err());
    }

    #[test]
    fn test_port_validation() {
        assert!(Port::try_new(80).is_ok());
        assert!(Port::try_new(8080).is_ok());
        assert!(Port::try_new(65535).is_ok());
        assert!(Port::try_new(0).is_err());
        // 65536 would overflow u16, so we can't test it
    }

    #[test]
    fn test_database_username_validation() {
        assert!(DatabaseUsername::try_new("postgres".to_string()).is_ok());
        assert!(DatabaseUsername::try_new("user_123".to_string()).is_ok());
        assert!(DatabaseUsername::try_new("".to_string()).is_err());
        assert!(DatabaseUsername::try_new("a".repeat(129)).is_err());
    }

    #[test]
    fn test_database_password_security() {
        let password = DatabasePassword::try_new("secret123".to_string()).unwrap();
        assert_eq!(format!("{password:?}"), "DatabasePassword(***)");
        assert_eq!(format!("{password}"), "***");
        assert_eq!(password.as_ref(), "secret123");
    }

    #[test]
    fn test_database_name_validation() {
        assert!(DatabaseName::try_new("union_square".to_string()).is_ok());
        assert!(DatabaseName::try_new("testdb".to_string()).is_ok());
        assert!(DatabaseName::try_new("DB123".to_string()).is_ok());
        assert!(DatabaseName::try_new("".to_string()).is_err());
        assert!(DatabaseName::try_new("123db".to_string()).is_err()); // can't start with number
        assert!(DatabaseName::try_new("db-name".to_string()).is_err()); // no hyphens
    }

    #[test]
    fn test_max_connections_validation() {
        assert!(MaxConnections::try_new(10).is_ok());
        assert!(MaxConnections::try_new(100).is_ok());
        assert!(MaxConnections::try_new(1000).is_ok());
        assert!(MaxConnections::try_new(0).is_err());
        assert!(MaxConnections::try_new(1001).is_err());
    }

    #[test]
    fn test_log_level_parsing() {
        use std::str::FromStr;

        assert_eq!(LogLevel::from_str("info").unwrap(), LogLevel::Info);
        assert_eq!(LogLevel::from_str("INFO").unwrap(), LogLevel::Info);
        assert_eq!(LogLevel::from_str("warn").unwrap(), LogLevel::Warn);
        assert_eq!(LogLevel::from_str("warning").unwrap(), LogLevel::Warn);
        assert!(LogLevel::from_str("invalid").is_err());
    }

    #[test]
    fn test_log_format_parsing() {
        use std::str::FromStr;

        assert_eq!(LogFormat::from_str("json").unwrap(), LogFormat::Json);
        assert_eq!(LogFormat::from_str("JSON").unwrap(), LogFormat::Json);
        assert_eq!(LogFormat::from_str("pretty").unwrap(), LogFormat::Pretty);
        assert!(LogFormat::from_str("invalid").is_err());
    }

    #[test]
    fn test_batch_size_validation() {
        assert!(BatchSize::try_new(100).is_ok());
        assert!(BatchSize::try_new(1).is_ok());
        assert!(BatchSize::try_new(10000).is_ok());
        assert!(BatchSize::try_new(0).is_err());
        assert!(BatchSize::try_new(10001).is_err());
    }

    #[test]
    fn test_flush_interval_validation() {
        assert!(FlushIntervalMs::try_new(1000).is_ok());
        assert!(FlushIntervalMs::try_new(100).is_ok());
        assert!(FlushIntervalMs::try_new(60000).is_ok());
        assert!(FlushIntervalMs::try_new(0).is_err());
        assert!(FlushIntervalMs::try_new(60001).is_err());
    }

    // AWS region tests removed - AwsRegion is now in providers::bedrock::types

    #[test]
    fn test_provider_name_validation() {
        assert!(ProviderName::try_new("custom_provider".to_string()).is_ok());
        assert!(ProviderName::try_new("Provider123".to_string()).is_ok());
        assert!(ProviderName::try_new("my-provider".to_string()).is_ok());
        assert!(ProviderName::try_new("".to_string()).is_err());
        assert!(ProviderName::try_new("123provider".to_string()).is_err()); // can't start with number
        assert!(ProviderName::try_new("provider name".to_string()).is_err()); // no spaces
    }

    #[test]
    fn test_defaults() {
        assert_eq!(Host::default().as_ref(), "0.0.0.0");
        assert_eq!(*Port::default().as_ref(), 8080);
        assert_eq!(*MaxConnections::default().as_ref(), 10);
        assert_eq!(LogLevel::default(), LogLevel::Info);
        assert_eq!(LogFormat::default(), LogFormat::Json);
        assert_eq!(*BatchSize::default().as_ref(), 100);
        assert_eq!(*FlushIntervalMs::default().as_ref(), 1000);
        // AwsRegion default test removed - type moved to providers::bedrock::types
    }
}
