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

/// Application identifier
#[nutype(
    validate(not_empty, len_char_max = 100),
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
    ),
    new_unchecked
)]
pub struct ApplicationId(String);

/// Environment identifier (e.g., "production", "staging", "development")
#[nutype(
    validate(not_empty, regex = r"^[a-z][a-z0-9-]*$"),
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
    ),
    new_unchecked
)]
pub struct EnvironmentId(String);

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
    pub application_id: Option<ApplicationId>,
    pub environment_id: Option<EnvironmentId>,
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
                application_id: None,
                environment_id: None,
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
    use proptest::prelude::*;

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

    #[test]
    fn test_application_id_validation() {
        // Valid cases
        assert!(ApplicationId::try_new("my-app".to_string()).is_ok());
        assert!(ApplicationId::try_new("MyApplication".to_string()).is_ok());
        assert!(ApplicationId::try_new("app_123".to_string()).is_ok());
        assert!(ApplicationId::try_new("a".to_string()).is_ok());

        // Invalid cases
        assert!(ApplicationId::try_new("".to_string()).is_err());
        assert!(ApplicationId::try_new("a".repeat(101)).is_err());
    }

    #[test]
    fn test_environment_id_validation() {
        // Valid cases
        assert!(EnvironmentId::try_new("production".to_string()).is_ok());
        assert!(EnvironmentId::try_new("staging".to_string()).is_ok());
        assert!(EnvironmentId::try_new("dev-123".to_string()).is_ok());
        assert!(EnvironmentId::try_new("qa".to_string()).is_ok());

        // Invalid cases
        assert!(EnvironmentId::try_new("".to_string()).is_err());
        assert!(EnvironmentId::try_new("Production".to_string()).is_err()); // uppercase
        assert!(EnvironmentId::try_new("123prod".to_string()).is_err()); // starts with number
        assert!(EnvironmentId::try_new("-prod".to_string()).is_err()); // starts with hyphen
        assert!(EnvironmentId::try_new("prod_us".to_string()).is_err()); // underscore
    }

    // Property-based tests
    proptest! {
        #[test]
        fn prop_session_id_uniqueness(n in 1..100usize) {
            let mut ids = std::collections::HashSet::new();
            for _ in 0..n {
                let id = SessionId::generate();
                assert!(ids.insert(id));
            }
        }

        #[test]
        fn prop_application_id_roundtrip(s in ".{1,100}") {
            if let Ok(app_id) = ApplicationId::try_new(s.clone()) {
                assert_eq!(app_id.as_ref(), &s);

                // Test serialization roundtrip
                let json = serde_json::to_string(&app_id).unwrap();
                let deserialized: ApplicationId = serde_json::from_str(&json).unwrap();
                assert_eq!(app_id, deserialized);
            }
        }

        #[test]
        fn prop_environment_id_roundtrip(s in "[a-z][a-z0-9-]*") {
            if !s.is_empty() && !s.ends_with('-') && !s.contains("--") {
                if let Ok(env_id) = EnvironmentId::try_new(s.clone()) {
                    assert_eq!(env_id.as_ref(), &s);

                    // Test serialization roundtrip
                    let json = serde_json::to_string(&env_id).unwrap();
                    let deserialized: EnvironmentId = serde_json::from_str(&json).unwrap();
                    assert_eq!(env_id, deserialized);
                }
            }
        }

        #[test]
        fn prop_session_serialization_roundtrip(
            user_id_seed in prop::option::of(any::<u128>()),
            app_name in prop::option::of(".{1,100}"),
            env_name in prop::option::of("[a-z][a-z0-9-]*"),
            user_agent in prop::option::of(any::<String>()),
            ip in prop::option::of(any::<String>()),
            tags in prop::collection::vec(any::<String>(), 0..10)
        ) {
            let user_id = user_id_seed.map(|_| crate::domain::UserId::generate());
            let mut session = Session::new(user_id);

            session.metadata.application_id = app_name
                .filter(|s| !s.is_empty())
                .and_then(|s| ApplicationId::try_new(s).ok());
            session.metadata.environment_id = env_name
                .filter(|s| !s.is_empty() && !s.ends_with('-') && !s.contains("--"))
                .and_then(|s| EnvironmentId::try_new(s).ok());
            session.metadata.user_agent = user_agent;
            session.metadata.ip_address = ip;
            session.metadata.tags = tags;

            let json = serde_json::to_string(&session).unwrap();
            let deserialized: Session = serde_json::from_str(&json).unwrap();
            assert_eq!(session, deserialized);
        }
    }
}
