use crate::domain::types::{IpAddress, Tag, UserAgent};
use chrono::{DateTime, Utc};
use nutype::nutype;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for a session
#[nutype(derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, AsRef))]
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
    )
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
    )
)]
pub struct EnvironmentId(String);

/// Error returned when an invalid session state transition is attempted.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("invalid session transition from {from:?} to {to:?}")]
pub struct TransitionError {
    pub from: SessionStatus,
    pub to: SessionStatus,
}

/// Session represents a complete interaction session with an LLM
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Session {
    id: SessionId,
    user_id: Option<crate::domain::UserId>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    status: SessionStatus,
    metadata: SessionMetadata,
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
    application_id: Option<ApplicationId>,
    environment_id: Option<EnvironmentId>,
    user_agent: Option<UserAgent>,
    ip_address: Option<IpAddress>,
    tags: Vec<Tag>,
}

impl SessionMetadata {
    pub fn new() -> Self {
        Self {
            application_id: None,
            environment_id: None,
            user_agent: None,
            ip_address: None,
            tags: Vec::new(),
        }
    }

    pub fn with_application_id(mut self, id: ApplicationId) -> Self {
        self.application_id = Some(id);
        self
    }

    pub fn with_environment_id(mut self, id: EnvironmentId) -> Self {
        self.environment_id = Some(id);
        self
    }

    pub fn with_user_agent(mut self, agent: UserAgent) -> Self {
        self.user_agent = Some(agent);
        self
    }

    pub fn with_ip_address(mut self, ip: IpAddress) -> Self {
        self.ip_address = Some(ip);
        self
    }

    pub fn with_tag(mut self, tag: Tag) -> Self {
        self.tags.push(tag);
        self
    }

    pub fn application_id(&self) -> Option<&ApplicationId> {
        self.application_id.as_ref()
    }

    pub fn environment_id(&self) -> Option<&EnvironmentId> {
        self.environment_id.as_ref()
    }

    pub fn user_agent(&self) -> Option<&UserAgent> {
        self.user_agent.as_ref()
    }

    pub fn ip_address(&self) -> Option<&IpAddress> {
        self.ip_address.as_ref()
    }

    pub fn tags(&self) -> &[Tag] {
        &self.tags
    }
}

impl Default for SessionMetadata {
    fn default() -> Self {
        Self::new()
    }
}

impl Session {
    pub fn new(user_id: Option<crate::domain::UserId>, created_at: DateTime<Utc>) -> Self {
        Self {
            id: SessionId::generate(),
            user_id,
            created_at,
            updated_at: created_at,
            status: SessionStatus::Active,
            metadata: SessionMetadata::new(),
        }
    }

    pub fn with_metadata(mut self, metadata: SessionMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Consuming transition: complete the session.
    /// Returns `Err` if the session is not in `Active` status.
    pub fn complete(self, at: DateTime<Utc>) -> Result<Self, TransitionError> {
        if !self.is_active() {
            return Err(TransitionError {
                from: self.status.clone(),
                to: SessionStatus::Completed,
            });
        }
        Ok(Self {
            status: SessionStatus::Completed,
            updated_at: at,
            ..self
        })
    }

    /// Consuming transition: mark the session as failed.
    /// Returns `Err` if the session is not in `Active` status.
    pub fn fail(self, at: DateTime<Utc>) -> Result<Self, TransitionError> {
        if !self.is_active() {
            return Err(TransitionError {
                from: self.status.clone(),
                to: SessionStatus::Failed,
            });
        }
        Ok(Self {
            status: SessionStatus::Failed,
            updated_at: at,
            ..self
        })
    }

    /// Consuming transition: cancel the session.
    /// Returns `Err` if the session is not in `Active` status.
    pub fn cancel(self, at: DateTime<Utc>) -> Result<Self, TransitionError> {
        if !self.is_active() {
            return Err(TransitionError {
                from: self.status.clone(),
                to: SessionStatus::Cancelled,
            });
        }
        Ok(Self {
            status: SessionStatus::Cancelled,
            updated_at: at,
            ..self
        })
    }

    pub fn is_active(&self) -> bool {
        matches!(self.status, SessionStatus::Active)
    }

    pub fn id(&self) -> &SessionId {
        &self.id
    }

    pub fn user_id(&self) -> Option<&crate::domain::UserId> {
        self.user_id.as_ref()
    }

    pub fn created_at(&self) -> &DateTime<Utc> {
        &self.created_at
    }

    pub fn updated_at(&self) -> &DateTime<Utc> {
        &self.updated_at
    }

    pub fn status(&self) -> &SessionStatus {
        &self.status
    }

    pub fn metadata(&self) -> &SessionMetadata {
        &self.metadata
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
        let now = Utc::now();
        let session = Session::new(None, now);
        assert!(session.is_active());
        assert_eq!(session.status(), &SessionStatus::Active);
        assert!(session.metadata().tags().is_empty());
    }

    #[test]
    fn test_session_status_transitions() {
        let now = Utc::now();
        let session = Session::new(None, now);

        assert!(session.is_active());

        let session = session.complete(now).unwrap();
        assert_eq!(session.status(), &SessionStatus::Completed);
        assert!(!session.is_active());
    }

    #[test]
    fn test_session_invalid_transition_from_completed_to_failed() {
        let now = Utc::now();
        let session = Session::new(None, now);
        let completed = session.complete(now).unwrap();

        let err = completed.fail(now).unwrap_err();
        assert_eq!(err.from, SessionStatus::Completed);
        assert_eq!(err.to, SessionStatus::Failed);
    }

    #[test]
    fn test_session_cancel_transition() {
        let now = Utc::now();
        let session = Session::new(None, now);
        let cancelled = session.cancel(now).unwrap();
        assert_eq!(cancelled.status(), &SessionStatus::Cancelled);
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
            user_agent in prop::option::of("[a-zA-Z0-9 /;.()]+") ,
            ip in prop::option::of("(25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\\.(25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\\.(25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\\.(25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)"),
            tags in prop::collection::vec("[a-zA-Z0-9][a-zA-Z0-9:._-]*", 0..10)
        ) {
            let user_id = user_id_seed.map(|_| crate::domain::UserId::generate());
            let now = Utc::now();
            let mut metadata = SessionMetadata::new();

            if let Some(app) = app_name
                .filter(|s| !s.is_empty())
                .and_then(|s| ApplicationId::try_new(s).ok()) {
                metadata = metadata.with_application_id(app);
            }
            if let Some(env) = env_name
                .filter(|s| !s.is_empty() && !s.ends_with('-') && !s.contains("--"))
                .and_then(|s| EnvironmentId::try_new(s).ok()) {
                metadata = metadata.with_environment_id(env);
            }
            if let Some(agent) = user_agent
                .filter(|s| !s.is_empty())
                .and_then(|s| UserAgent::try_new(s).ok()) {
                metadata = metadata.with_user_agent(agent);
            }
            if let Some(addr) = ip.and_then(|s| IpAddress::try_new(s).ok()) {
                metadata = metadata.with_ip_address(addr);
            }
            for tag in tags.into_iter().filter_map(|s| Tag::try_new(s).ok()) {
                metadata = metadata.with_tag(tag);
            }

            let session = Session::new(user_id, now).with_metadata(metadata);

            let json = serde_json::to_string(&session).unwrap();
            let deserialized: Session = serde_json::from_str(&json).unwrap();
            assert_eq!(session, deserialized);
        }
    }
}
