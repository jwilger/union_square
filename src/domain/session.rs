use chrono::{DateTime, Utc};
use nutype::nutype;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for a session
#[nutype(derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize))]
pub struct SessionId(Uuid);

impl SessionId {
    pub fn generate() -> Self {
        // Uuid::now_v7() always generates a valid UUID
        Self::new(Uuid::now_v7())
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::generate()
    }
}

/// Session represents a complete interaction session with an LLM
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Session {
    pub id: SessionId,
    pub user_id: Option<crate::domain::UserId>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub status: SessionStatus,
    pub metadata: SessionMetadata,
}

/// Status of a session
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionStatus {
    Active,
    Completed,
    Failed,
    Cancelled,
}

/// Metadata associated with a session
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionMetadata {
    pub application_name: Option<String>,
    pub environment: Option<String>,
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
    pub tags: Vec<String>,
}

impl Session {
    pub fn new(user_id: Option<crate::domain::UserId>) -> Self {
        let now = Utc::now();
        Self {
            id: SessionId::generate(),
            user_id,
            created_at: now,
            updated_at: now,
            status: SessionStatus::Active,
            metadata: SessionMetadata {
                application_name: None,
                environment: None,
                user_agent: None,
                ip_address: None,
                tags: Vec::new(),
            },
        }
    }

    pub fn complete(&mut self) {
        self.status = SessionStatus::Completed;
        self.updated_at = Utc::now();
    }

    pub fn fail(&mut self) {
        self.status = SessionStatus::Failed;
        self.updated_at = Utc::now();
    }

    pub fn cancel(&mut self) {
        self.status = SessionStatus::Cancelled;
        self.updated_at = Utc::now();
    }

    pub fn is_active(&self) -> bool {
        matches!(self.status, SessionStatus::Active)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_id_generation() {
        let id1 = SessionId::generate();
        let id2 = SessionId::generate();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_session_creation() {
        let session = Session::new(None);
        assert!(session.is_active());
        assert_eq!(session.status, SessionStatus::Active);
        assert!(session.metadata.tags.is_empty());
    }

    #[test]
    fn test_session_status_transitions() {
        let mut session = Session::new(None);

        assert!(session.is_active());

        session.complete();
        assert_eq!(session.status, SessionStatus::Completed);
        assert!(!session.is_active());

        session.fail();
        assert_eq!(session.status, SessionStatus::Failed);
    }
}
