use crate::domain::config_types::{
    BatchSize, DatabaseName, DatabasePassword, DatabaseUsername, FlushIntervalMs, Host, LogFormat,
    LogLevel, MaxConnections, Port,
};
use crate::domain::session::EnvironmentId;
use crate::providers::constants::{config_defaults, config_paths, environments};
use config::{Config, Environment, File};
use serde::Deserialize;
use std::env;

// Re-export ConfigError so it can be used in error.rs
pub use config::ConfigError;

#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    pub application: ApplicationSettings,
    pub database: DatabaseSettings,
    pub eventcore: EventCoreSettings,
    pub logging: LoggingSettings,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ApplicationSettings {
    pub host: Host,
    pub port: Port,
    pub environment: EnvironmentId,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseSettings {
    pub host: Host,
    pub port: Port,
    pub username: DatabaseUsername,
    pub password: DatabasePassword,
    pub database_name: DatabaseName,
    pub max_connections: MaxConnections,
}

#[derive(Debug, Deserialize, Clone)]
pub struct EventCoreSettings {
    pub batch_size: BatchSize,
    pub flush_interval_ms: FlushIntervalMs,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LoggingSettings {
    pub level: LogLevel,
    pub format: LogFormat,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let environment =
            env::var("ENVIRONMENT").unwrap_or_else(|_| environments::DEVELOPMENT.to_string());

        let config = Config::builder()
            // Start with default values
            .set_default("application.host", config_defaults::APP_HOST)?
            .set_default("application.port", config_defaults::APP_PORT)?
            .set_default("application.environment", environment.clone())?
            .set_default("database.host", config_defaults::DB_HOST)?
            .set_default("database.port", config_defaults::DB_PORT)?
            .set_default("database.username", config_defaults::DB_USERNAME)?
            .set_default("database.password", config_defaults::DB_PASSWORD)?
            .set_default("database.database_name", config_defaults::DB_NAME)?
            .set_default(
                "database.max_connections",
                config_defaults::DB_MAX_CONNECTIONS,
            )?
            .set_default(
                "eventcore.batch_size",
                config_defaults::EVENTCORE_BATCH_SIZE,
            )?
            .set_default(
                "eventcore.flush_interval_ms",
                config_defaults::EVENTCORE_FLUSH_INTERVAL,
            )?
            .set_default("logging.level", config_defaults::LOG_LEVEL)?
            .set_default("logging.format", config_defaults::LOG_FORMAT)?
            // Add configuration file if it exists
            .add_source(File::with_name(config_paths::DEFAULT).required(false))
            .add_source(File::with_name(&format!("config/{environment}")).required(false))
            .add_source(File::with_name(config_paths::LOCAL).required(false))
            // Add environment variables with prefix
            .add_source(
                Environment::with_prefix(config_paths::ENV_PREFIX)
                    .separator(config_paths::ENV_SEPARATOR),
            )
            .build()?;

        config.try_deserialize()
    }

    pub fn database_url(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.database.username,
            self.database.password.as_ref(),
            self.database.host,
            self.database.port,
            self.database.database_name
        )
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self::new().expect("Failed to load configuration")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_can_be_loaded() {
        let settings = Settings::new();
        assert!(settings.is_ok());
    }

    #[test]
    fn test_database_url_format() {
        let settings = Settings::new().unwrap();
        let url = settings.database_url();
        assert!(url.starts_with("postgres://"));
        assert!(url.contains(settings.database.username.as_ref()));
        assert!(url.contains(settings.database.database_name.as_ref()));
    }
}
