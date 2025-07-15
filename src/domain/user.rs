use nutype::nutype;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for a user
#[nutype(
    derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize),
    new_unchecked
)]
pub struct UserId(Uuid);

impl UserId {
    pub fn generate() -> Self {
        // Safety: Uuid::now_v7() always generates a valid UUID
        unsafe { Self::new_unchecked(Uuid::now_v7()) }
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
    validate(predicate = |email| email.contains('@') && email.len() > 3),
    new_unchecked
)]
pub struct EmailAddress(String);

impl EmailAddress {
    pub fn parse(email: String) -> Result<Self, String> {
        if email.contains('@') && email.len() > 3 {
            // Safety: We validated the email format above
            Ok(unsafe { Self::new_unchecked(email) })
        } else {
            Err("Invalid email address format".to_string())
        }
    }
}

/// User display name
#[nutype(
    derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize),
    validate(predicate = |name| !name.trim().is_empty() && name.len() <= 255),
    new_unchecked
)]
pub struct DisplayName(String);

impl DisplayName {
    pub fn parse(name: String) -> Result<Self, String> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            Err("Display name cannot be empty".to_string())
        } else if trimmed.len() > 255 {
            Err("Display name cannot exceed 255 characters".to_string())
        } else {
            // Safety: We validated the display name above
            Ok(unsafe { Self::new_unchecked(trimmed.to_string()) })
        }
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
}