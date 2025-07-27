use crate::providers::constants::{error_messages, sql};
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
        let row = sqlx::query(sql::HEALTH_CHECK_QUERY)
            .fetch_one(&self.pool)
            .await?;

        let health_check: i32 = row.try_get(sql::HEALTH_CHECK_COLUMN)?;

        if health_check == sql::HEALTH_CHECK_EXPECTED_VALUE {
            Ok(())
        } else {
            Err(Error::application(
                error_messages::DATABASE_HEALTH_CHECK_FAILED,
            ))
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
