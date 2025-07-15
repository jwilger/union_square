use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;
use std::env;

#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    pub application: ApplicationSettings,
    pub database: DatabaseSettings,
    pub eventcore: EventCoreSettings,
    pub logging: LoggingSettings,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ApplicationSettings {
    pub host: String,
    pub port: u16,
    pub environment: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseSettings {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub database_name: String,
    pub max_connections: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct EventCoreSettings {
    pub batch_size: usize,
    pub flush_interval_ms: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LoggingSettings {
    pub level: String,
    pub format: String,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let environment = env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string());
        
        let config = Config::builder()
            // Start with default values
            .set_default("application.host", "0.0.0.0")?
            .set_default("application.port", 8080)?
            .set_default("application.environment", environment.clone())?
            .set_default("database.host", "localhost")?
            .set_default("database.port", 5432)?
            .set_default("database.username", "postgres")?
            .set_default("database.password", "password")?
            .set_default("database.database_name", "union_square")?
            .set_default("database.max_connections", 10)?
            .set_default("eventcore.batch_size", 100)?
            .set_default("eventcore.flush_interval_ms", 1000)?
            .set_default("logging.level", "info")?
            .set_default("logging.format", "json")?
            // Add configuration file if it exists
            .add_source(File::with_name("config/default").required(false))
            .add_source(File::with_name(&format!("config/{}", environment)).required(false))
            .add_source(File::with_name("config/local").required(false))
            // Add environment variables with prefix
            .add_source(Environment::with_prefix("UNION_SQUARE").separator("__"))
            .build()?;

        config.try_deserialize()
    }
    
    pub fn database_url(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.database.username,
            self.database.password,
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
        assert!(url.contains(&settings.database.username));
        assert!(url.contains(&settings.database.database_name));
    }
}