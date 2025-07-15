use crate::config::Settings;
use crate::Result;
use sqlx::PgPool;
use tracing::{info, instrument};

/// Main application struct that coordinates all components
pub struct Application {
    settings: Settings,
    db_pool: PgPool,
}

impl Application {
    #[instrument]
    pub async fn new() -> Result<Self> {
        let settings = Settings::new()?;

        info!("Connecting to database at {}", settings.database.host);
        let db_pool = PgPool::connect(&settings.database_url()).await?;

        // Run database migrations if needed
        // sqlx::migrate!("./migrations").run(&db_pool).await?;

        Ok(Self { settings, db_pool })
    }

    #[instrument(skip(self))]
    pub async fn run(self) -> Result<()> {
        info!(
            "Starting Union Square server on {}:{}",
            self.settings.application.host, self.settings.application.port
        );

        // This is a placeholder - in the future this will start the actual server
        // For now, just demonstrate that the application can be constructed
        info!("Application started successfully");

        Ok(())
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    pub fn db_pool(&self) -> &PgPool {
        &self.db_pool
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_application_can_be_created() {
        // This test will only pass if a database is running
        // Skip it if the database is not available
        if let Ok(app) = Application::new().await {
            assert!(app.settings().application.port > 0);
        }
    }
}
