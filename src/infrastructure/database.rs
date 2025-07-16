use crate::{Error, Result};
use sqlx::{PgPool, Row};

/// Database connection pool wrapper
pub struct Database {
    pool: PgPool,
}

impl Database {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Health check for the database connection
    pub async fn health_check(&self) -> Result<()> {
        let row = sqlx::query("SELECT 1 as health_check")
            .fetch_one(&self.pool)
            .await?;

        let health_check: i32 = row.try_get("health_check")?;

        if health_check == 1 {
            Ok(())
        } else {
            Err(Error::application("Database health check failed"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "requires database connection"]
    async fn test_database_health_check() {
        let pool = PgPool::connect("postgres://postgres:password@localhost:5432/union_square")
            .await
            .expect("Failed to connect to database");

        let db = Database::new(pool);
        let result = db.health_check().await;
        assert!(result.is_ok());
    }
}
