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

        // TODO: Implement database migrations once the migration scripts are finalized.
        // sqlx::migrate!("./migrations").run(&db_pool).await?;

        Ok(Self { settings, db_pool })
    }

    #[instrument(skip(self))]
    pub async fn run(self) -> Result<()> {
        info!(
            "Starting Union Square server on {}:{}",
            self.settings.application.host, self.settings.application.port
        );

        // TODO: Implement the server startup logic here.
        // Consider using an HTTP framework like `axum` or `warp` to handle requests.
        // This should include routing, middleware, and graceful shutdown handling.
        // Timeline: Prioritize this implementation before the first production release.
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
    #[ignore = "requires database connection"]
    async fn test_application_can_be_created() {
        let app = Application::new()
            .await
            .expect("Failed to create application");
        assert!(*app.settings().application.port.as_ref() > 0);
    }
}
