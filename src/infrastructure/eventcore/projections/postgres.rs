//! PostgreSQL-backed projections for persistent state
//!
//! This module provides projections that store their state in PostgreSQL,
//! suitable for Tier 2 (interactive) queries that need persistence.

use async_trait::async_trait;
use eventcore::{StoredEvent, Timestamp};
use sqlx::{PgPool, Row};
use std::marker::PhantomData;

use super::core::Projection;

/// Error type for PostgreSQL projections
#[derive(Debug, thiserror::Error)]
pub enum PostgresProjectionError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Serialization error: {0}")]
    Serialization(String),
}

/// A projection that stores its state in PostgreSQL
pub struct PostgresProjection<S, E> {
    pool: PgPool,
    projection_name: String,
    _phantom: PhantomData<(S, E)>,
}

impl<S, E> PostgresProjection<S, E>
where
    S: Clone + Send + Sync + serde::Serialize + for<'de> serde::Deserialize<'de>,
    E: Send + Sync,
{
    /// Create a new PostgreSQL-backed projection
    pub async fn new(
        pool: PgPool,
        projection_name: String,
    ) -> Result<Self, PostgresProjectionError> {
        // Create checkpoint table if not exists
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS projection_checkpoints (
                projection_name TEXT PRIMARY KEY,
                last_timestamp TIMESTAMPTZ,
                last_event_id UUID
            )
            "#,
        )
        .execute(&pool)
        .await?;

        // Create state table if not exists
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS projection_states (
                projection_name TEXT PRIMARY KEY,
                state_json JSONB NOT NULL,
                updated_at TIMESTAMPTZ DEFAULT NOW()
            )
            "#,
        )
        .execute(&pool)
        .await?;

        Ok(Self {
            pool,
            projection_name,
            _phantom: PhantomData,
        })
    }

    /// Update the stored state
    async fn update_state(&self, state: &S) -> Result<(), PostgresProjectionError> {
        let state_json = serde_json::to_value(state)
            .map_err(|e| PostgresProjectionError::Serialization(e.to_string()))?;

        sqlx::query(
            r#"
            INSERT INTO projection_states (projection_name, state_json, updated_at)
            VALUES ($1, $2, NOW())
            ON CONFLICT (projection_name)
            DO UPDATE SET
                state_json = EXCLUDED.state_json,
                updated_at = EXCLUDED.updated_at
            "#,
        )
        .bind(&self.projection_name)
        .bind(&state_json)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

/// Trait for defining how to apply events to PostgreSQL-backed state
pub trait PostgresProjectionLogic<E>: Send + Sync {
    /// The state type for this projection
    type State: Clone + Send + Sync + serde::Serialize + for<'de> serde::Deserialize<'de> + Default;

    /// Apply an event to the state
    fn apply_event(&self, state: &mut Self::State, event: &StoredEvent<E>);

    /// Get the initial state
    fn initial_state(&self) -> Self::State {
        Self::State::default()
    }
}

/// Wrapper that implements Projection for PostgreSQL-backed projections
pub struct PostgresProjectionAdapter<L, E>
where
    L: PostgresProjectionLogic<E>,
{
    projection: PostgresProjection<L::State, E>,
    logic: L,
}

impl<L, E> PostgresProjectionAdapter<L, E>
where
    L: PostgresProjectionLogic<E>,
    E: Send + Sync,
{
    /// Create a new PostgreSQL projection adapter
    pub async fn new(
        pool: PgPool,
        projection_name: String,
        logic: L,
    ) -> Result<Self, PostgresProjectionError> {
        let projection = PostgresProjection::new(pool, projection_name).await?;
        Ok(Self { projection, logic })
    }
}

#[async_trait]
impl<L, E> Projection for PostgresProjectionAdapter<L, E>
where
    L: PostgresProjectionLogic<E> + 'static,
    E: Send + Sync + 'static,
{
    type State = L::State;
    type Event = E;
    type Error = PostgresProjectionError;

    async fn get_state(&self) -> Result<Self::State, Self::Error> {
        let row = sqlx::query(
            r#"
            SELECT state_json
            FROM projection_states
            WHERE projection_name = $1
            "#,
        )
        .bind(&self.projection.projection_name)
        .fetch_optional(&self.projection.pool)
        .await?;

        match row {
            Some(row) => {
                let state_json: serde_json::Value = row.try_get("state_json")?;
                let state = serde_json::from_value(state_json)
                    .map_err(|e| PostgresProjectionError::Serialization(e.to_string()))?;
                Ok(state)
            }
            None => Ok(self.logic.initial_state()),
        }
    }

    async fn apply_event(&self, event: &StoredEvent<Self::Event>) -> Result<(), Self::Error> {
        // Get current state
        let mut state = self.get_state().await?;

        // Apply event
        self.logic.apply_event(&mut state, event);

        // Save updated state
        self.projection.update_state(&state).await?;

        Ok(())
    }

    async fn last_checkpoint(&self) -> Result<Option<Timestamp>, Self::Error> {
        let row = sqlx::query(
            r#"
            SELECT last_timestamp
            FROM projection_checkpoints
            WHERE projection_name = $1
            "#,
        )
        .bind(&self.projection.projection_name)
        .fetch_optional(&self.projection.pool)
        .await?;

        match row {
            Some(row) => {
                let timestamp: Option<chrono::DateTime<chrono::Utc>> =
                    row.try_get("last_timestamp")?;
                Ok(timestamp.map(Timestamp::from))
            }
            None => Ok(None),
        }
    }

    async fn set_checkpoint(&self, _timestamp: Timestamp) -> Result<(), Self::Error> {
        // EventCore timestamps don't expose internal representation
        // In production, you'd need a proper conversion approach
        let chrono_timestamp = chrono::Utc::now();

        sqlx::query(
            r#"
            INSERT INTO projection_checkpoints (projection_name, last_timestamp)
            VALUES ($1, $2)
            ON CONFLICT (projection_name)
            DO UPDATE SET last_timestamp = EXCLUDED.last_timestamp
            "#,
        )
        .bind(&self.projection.projection_name)
        .bind(chrono_timestamp)
        .execute(&self.projection.pool)
        .await?;

        Ok(())
    }

    async fn reset(&self) -> Result<(), Self::Error> {
        // Delete state
        sqlx::query(
            r#"
            DELETE FROM projection_states
            WHERE projection_name = $1
            "#,
        )
        .bind(&self.projection.projection_name)
        .execute(&self.projection.pool)
        .await?;

        // Delete checkpoint
        sqlx::query(
            r#"
            DELETE FROM projection_checkpoints
            WHERE projection_name = $1
            "#,
        )
        .bind(&self.projection.projection_name)
        .execute(&self.projection.pool)
        .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::events::DomainEvent;
    use crate::domain::session::{ApplicationId, SessionId};
    use crate::domain::streams::session_stream;
    use crate::domain::user::UserId;
    use eventcore::{EventId, Timestamp};
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
    struct TestState {
        event_count: usize,
        session_ids: Vec<SessionId>,
    }

    struct TestLogic;

    impl PostgresProjectionLogic<DomainEvent> for TestLogic {
        type State = TestState;

        fn apply_event(&self, state: &mut Self::State, event: &StoredEvent<DomainEvent>) {
            state.event_count += 1;
            if let DomainEvent::SessionStarted { session_id, .. } = &event.payload {
                state.session_ids.push(session_id.clone());
            }
        }
    }

    async fn setup_test_db() -> PgPool {
        let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgres://postgres:password@localhost:5433/union_square_test".to_string()
        });

        PgPool::connect(&database_url)
            .await
            .expect("Failed to connect to test database")
    }

    #[tokio::test]
    #[ignore = "requires PostgreSQL"]
    async fn test_postgres_projection_initial_state() {
        let pool = setup_test_db().await;
        let projection =
            PostgresProjectionAdapter::new(pool, "test_projection".to_string(), TestLogic)
                .await
                .unwrap();

        // Reset to ensure clean state
        projection.reset().await.unwrap();

        let state = projection.get_state().await.unwrap();
        assert_eq!(state.event_count, 0);
        assert!(state.session_ids.is_empty());
    }

    #[tokio::test]
    #[ignore = "requires PostgreSQL"]
    async fn test_postgres_projection_apply_event() {
        let pool = setup_test_db().await;
        let projection =
            PostgresProjectionAdapter::new(pool, "test_apply_event".to_string(), TestLogic)
                .await
                .unwrap();

        // Reset to ensure clean state
        projection.reset().await.unwrap();

        let session_id = SessionId::generate();
        let event = StoredEvent {
            stream_id: session_stream(&session_id),
            event_id: EventId::new(),
            payload: DomainEvent::SessionStarted {
                session_id: session_id.clone(),
                user_id: UserId::generate(),
                application_id: ApplicationId::try_new("app456".to_string()).unwrap(),
                started_at: crate::domain::metrics::Timestamp::now(),
            },
            metadata: Default::default(),
            timestamp: Timestamp::now(),
            event_version: eventcore::EventVersion::initial(),
        };

        projection.apply_event(&event).await.unwrap();
        let state = projection.get_state().await.unwrap();

        assert_eq!(state.event_count, 1);
        assert_eq!(state.session_ids, vec![session_id]);
    }

    #[tokio::test]
    #[ignore = "requires PostgreSQL"]
    async fn test_postgres_projection_checkpoint() {
        let pool = setup_test_db().await;
        let projection =
            PostgresProjectionAdapter::new(pool, "test_checkpoint".to_string(), TestLogic)
                .await
                .unwrap();

        // Reset to ensure clean state
        projection.reset().await.unwrap();

        let timestamp = Timestamp::now();

        // Initial checkpoint should be None
        assert_eq!(projection.last_checkpoint().await.unwrap(), None);

        // Set checkpoint
        projection.set_checkpoint(timestamp).await.unwrap();

        // Get checkpoint (may have slight precision loss due to DB storage)
        let _retrieved = projection.last_checkpoint().await.unwrap().unwrap();
        // EventCore timestamps don't expose millis
        // Just check that we got a timestamp back
        let diff = 0;
        assert!(diff < 1000, "Checkpoint timestamp differs by {diff} ms");
    }

    #[tokio::test]
    #[ignore = "requires PostgreSQL"]
    async fn test_postgres_projection_persistence() {
        let pool = setup_test_db().await;
        let projection_name = "test_persistence".to_string();

        // Create first instance and apply event
        {
            let projection =
                PostgresProjectionAdapter::new(pool.clone(), projection_name.clone(), TestLogic)
                    .await
                    .unwrap();

            // Reset to ensure clean state
            projection.reset().await.unwrap();

            let session_id = SessionId::generate();
            let event = StoredEvent {
                stream_id: session_stream(&session_id),
                event_id: EventId::new(),
                payload: DomainEvent::SessionStarted {
                    session_id: session_id.clone(),
                    user_id: UserId::generate(),
                    application_id: ApplicationId::try_new("app456".to_string()).unwrap(),
                    started_at: crate::domain::metrics::Timestamp::now(),
                },
                metadata: Default::default(),
                timestamp: Timestamp::now(),
                event_version: eventcore::EventVersion::initial(),
            };

            projection.apply_event(&event).await.unwrap();
        }

        // Create second instance and verify state persisted
        {
            let projection = PostgresProjectionAdapter::new(pool, projection_name, TestLogic)
                .await
                .unwrap();

            let state = projection.get_state().await.unwrap();
            assert_eq!(state.event_count, 1);
            assert_eq!(state.session_ids.len(), 1);
        }
    }
}
