use nutype::nutype;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for a user
#[nutype(derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    Display,
    AsRef
))]
pub struct UserId(Uuid);

impl UserId {
    pub fn generate() -> Self {
        // Uuid::now_v7() always generates a valid UUID
        Self::new(Uuid::now_v7())
    }
}

impl Default for UserId {
    fn default() -> Self {
        Self::generate()
    }
}

/// User email address (validated)
#[nutype(
    derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize),
    validate(predicate = |email| email.contains('@') && email.len() > 3)
)]
pub struct EmailAddress(String);

impl EmailAddress {
    pub fn parse(email: String) -> Result<Self, String> {
        Self::try_new(email).map_err(|_| "Invalid email address format".to_string())
    }
}

/// User display name
#[nutype(
    derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize),
    validate(predicate = |name| !name.trim().is_empty() && name.len() <= 255)
)]
pub struct DisplayName(String);

impl DisplayName {
    pub fn parse(name: String) -> Result<Self, String> {
        let trimmed = name.trim().to_string();
        Self::try_new(trimmed).map_err(|e| e.to_string())
    }
}

/// User represents a user of the Union Square system
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct User {
    pub id: UserId,
    pub email: EmailAddress,
    pub display_name: DisplayName,
    pub is_active: bool,
}

impl User {
    pub fn new(email: EmailAddress, display_name: DisplayName) -> Self {
        Self {
            id: UserId::generate(),
            email,
            display_name,
            is_active: true,
        }
    }

    pub fn deactivate(&mut self) {
        self.is_active = false;
    }

    pub fn activate(&mut self) {
        self.is_active = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_user_id_generation() {
        let id1 = UserId::generate();
        let id2 = UserId::generate();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_email_validation() {
        assert!(EmailAddress::parse("test@example.com".to_string()).is_ok());
        assert!(EmailAddress::parse("invalid-email".to_string()).is_err());
        assert!(EmailAddress::parse("a@b".to_string()).is_err());
    }

    #[test]
    fn test_display_name_validation() {
        assert!(DisplayName::parse("John Doe".to_string()).is_ok());
        assert!(DisplayName::parse("".to_string()).is_err());
        assert!(DisplayName::parse("   ".to_string()).is_err());
        assert!(DisplayName::parse("a".repeat(256)).is_err());
    }

    #[test]
    fn test_user_creation() {
        let email = EmailAddress::parse("test@example.com".to_string()).unwrap();
        let name = DisplayName::parse("Test User".to_string()).unwrap();

        let user = User::new(email, name);
        assert!(user.is_active);
        assert_eq!(user.email.into_inner(), "test@example.com");
        assert_eq!(user.display_name.into_inner(), "Test User");
    }

    #[test]
    fn test_user_activation() {
        let email = EmailAddress::parse("test@example.com".to_string()).unwrap();
        let name = DisplayName::parse("Test User".to_string()).unwrap();

        let mut user = User::new(email, name);
        assert!(user.is_active);

        user.deactivate();
        assert!(!user.is_active);

        user.activate();
        assert!(user.is_active);
    }

    // Property-based tests
    proptest! {
        #[test]
        fn prop_user_id_uniqueness(n in 1..100usize) {
            let mut ids = std::collections::HashSet::new();
            for _ in 0..n {
                let id = UserId::generate();
                assert!(ids.insert(id));
            }
        }

        #[test]
        fn prop_email_validation(s in any::<String>()) {
            let result = EmailAddress::parse(s.clone());
            if s.contains('@') && s.len() > 3 {
                // Could be valid
                if result.is_ok() {
                    let email = result.unwrap();
                    assert_eq!(email.into_inner(), s);
                }
            } else {
                // Must be invalid
                assert!(result.is_err());
            }
        }

        #[test]
        fn prop_display_name_validation(s in any::<String>()) {
            let result = DisplayName::parse(s.clone());
            let trimmed = s.trim();
            if !trimmed.is_empty() && trimmed.len() <= 255 {
                assert!(result.is_ok());
                let name = result.unwrap();
                assert_eq!(name.into_inner(), trimmed.to_string());
            } else {
                assert!(result.is_err());
            }
        }

        #[test]
        fn prop_user_serialization_roundtrip(
            email_str in "[a-z]+@[a-z]+\\.[a-z]+",
            name_str in "[a-zA-Z ]{1,100}"
        ) {
            if let (Ok(email), Ok(name)) = (
                EmailAddress::parse(email_str),
                DisplayName::parse(name_str)
            ) {
                let user = User::new(email, name);

                let json = serde_json::to_string(&user).unwrap();
                let deserialized: User = serde_json::from_str(&json).unwrap();

                // IDs will be different, but other fields should match
                assert_eq!(user.email, deserialized.email);
                assert_eq!(user.display_name, deserialized.display_name);
                assert_eq!(user.is_active, deserialized.is_active);
            }
        }
    }
}
