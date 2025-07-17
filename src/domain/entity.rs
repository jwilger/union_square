//! Entity identification types for event sourcing

use nutype::nutype;

/// Unique identifier for an entity in the event store
#[nutype(
    derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize),
    validate(not_empty)
)]
pub struct EntityId(String);

impl EntityId {
    /// Create an EntityId for a session
    pub fn session(id: uuid::Uuid) -> Self {
        Self::try_new(format!("session:{id}")).expect("Session ID format is always valid")
    }

    /// Create an EntityId for a request
    pub fn request(id: uuid::Uuid) -> Self {
        Self::try_new(format!("request:{id}")).expect("Request ID format is always valid")
    }

    /// Create an EntityId for a version
    pub fn version(version_key: &str) -> Self {
        Self::try_new(format!("version:{version_key}")).expect("Version ID format is always valid")
    }

    /// Create an EntityId for a version change
    pub fn version_change(id: uuid::Uuid) -> Self {
        Self::try_new(format!("version_change:{id}"))
            .expect("Version change ID format is always valid")
    }

    /// Create an EntityId for a user
    pub fn user(id: uuid::Uuid) -> Self {
        Self::try_new(format!("user:{id}")).expect("User ID format is always valid")
    }
}
