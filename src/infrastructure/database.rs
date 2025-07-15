use sqlx::{PgPool, Row};
use crate::{Result, Error};

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
    async fn test_database_health_check() {
        // This test requires a running database
        // Skip if not available
        if let Ok(pool) = PgPool::connect("postgres://postgres:password@localhost:5432/union_square").await {
            let db = Database::new(pool);
            let result = db.health_check().await;
            
            // If the database is available, this should succeed
            assert!(result.is_ok());
        }
    }
}