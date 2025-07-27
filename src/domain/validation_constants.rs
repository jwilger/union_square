//! Validation constants for domain types
//!
//! This module centralizes all validation constants, limits, and magic numbers
//! used throughout the domain layer to ensure consistency and maintainability.

/// Email validation constants
pub mod email {
    /// Minimum email length (e.g., "a@b.c")
    pub const MIN_LENGTH: usize = 5;

    /// Maximum email length for practical use
    pub const MAX_LENGTH: usize = 255;
}

/// Display name validation constants
pub mod display_name {
    /// Maximum display name length
    pub const MAX_LENGTH: usize = 255;
}

/// Test case validation constants
pub mod test_case {
    /// Maximum test case name length
    pub const MAX_NAME_LENGTH: usize = 200;

    /// Placeholder template for draft test cases
    pub const PLACEHOLDER_PROMPT_TEMPLATE: &str = "PLACEHOLDER";
}

/// User agent validation constants
pub mod user_agent {
    /// Maximum user agent string length
    pub const MAX_LENGTH: usize = 1000;
}

/// Tag validation constants
pub mod tag {
    /// Maximum tag length
    pub const MAX_LENGTH: usize = 100;

    /// Regex pattern for valid tags
    pub const VALIDATION_PATTERN: &str = r"^[a-zA-Z0-9][a-zA-Z0-9:._-]*$";
}

/// IP address validation constants
pub mod ip_address {
    /// Example IPv4 addresses for testing
    pub const TEST_IPV4_VALID: &[&str] = &["192.168.1.1", "10.0.0.0", "255.255.255.255"];

    /// Example IPv6 addresses for testing
    pub const TEST_IPV6_VALID: &[&str] =
        &["2001:0db8:85a3:0000:0000:8a2e:0370:7334", "::1", "fe80::1"];

    /// Invalid IP addresses for testing
    pub const TEST_INVALID: &[&str] = &["256.1.1.1", "192.168.1", "not-an-ip", ""];
}

/// Prompt and response size limits
pub mod content_limits {
    /// Soft limit for prompts and responses (logs warning)
    pub const SOFT_LIMIT_CHARS: usize = 100_000;

    /// Hard limit for prompts and responses (rejects)
    pub const HARD_LIMIT_CHARS: usize = 10_485_760; // 10MB

    /// Estimated characters per token for warnings
    pub const CHARS_PER_TOKEN_ESTIMATE: usize = 4;
}

/// Pattern validation constants
pub mod pattern {
    /// Maximum pattern length for regex/text matching
    pub const MAX_LENGTH: usize = 1000;
}

/// Test run size constants
pub mod test_runs {
    /// Default range for property test iterations
    pub const DEFAULT_PROPTEST_CASES: u32 = 100;

    /// Small test range for quick validation
    pub const SMALL_PROPTEST_RANGE: std::ops::Range<usize> = 1..100;
}

/// Environment ID validation
pub mod environment_id {
    /// Regex pattern for valid environment IDs
    pub const VALIDATION_PATTERN: &str = r"^[a-z][a-z0-9-]*$";

    /// Common environment names for testing
    pub const TEST_VALID: &[&str] = &["production", "staging", "dev-123", "qa"];

    /// Invalid environment names for testing
    pub const TEST_INVALID: &[&str] = &[
        "",
        "Production", // uppercase
        "123prod",    // starts with number
        "-prod",      // starts with hyphen
        "prod_us",    // underscore
    ];
}

/// Token count limits
pub mod tokens {
    /// Maximum token count for validation
    pub const MAX_COUNT: u32 = 1_000_000;
}

/// Latency limits
pub mod latency {
    /// Maximum latency in milliseconds (5 minutes)
    pub const MAX_MS: u64 = 300_000;
}

/// Error message limits
pub mod error_message {
    /// Maximum error message length
    pub const MAX_LENGTH: usize = 5000;
}

/// Field name limits
pub mod field_name {
    /// Maximum field name length
    pub const MAX_LENGTH: usize = 100;
}

/// Resource ID limits
pub mod resource_id {
    /// Maximum resource identifier length
    pub const MAX_LENGTH: usize = 200;
}

/// Application ID limits
pub mod application_id {
    /// Maximum application ID length
    pub const MAX_LENGTH: usize = 100;
}

/// Change reason limits
pub mod change_reason {
    /// Maximum change reason length
    pub const MAX_LENGTH: usize = 1000;
}

/// Assertion description limits
pub mod assertion_description {
    /// Maximum assertion description length
    pub const MAX_LENGTH: usize = 500;
}

/// Test case description limits
pub mod test_case_description {
    /// Maximum test case description length
    pub const MAX_LENGTH: usize = 1000;
}

/// Model ID limits
pub mod model_id {
    /// Maximum model identifier length
    pub const MAX_LENGTH: usize = 200;
}

/// Finish reason limits
pub mod finish_reason {
    /// Maximum finish reason length
    pub const MAX_LENGTH: usize = 100;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_limits_are_reasonable() {
        // Test specific values instead of comparisons
        assert_eq!(content_limits::SOFT_LIMIT_CHARS, 100_000);
        assert_eq!(content_limits::HARD_LIMIT_CHARS, 10_485_760);
        assert_eq!(content_limits::CHARS_PER_TOKEN_ESTIMATE, 4);
    }

    #[test]
    fn test_email_limits_are_reasonable() {
        // Test specific values instead of comparisons
        assert_eq!(email::MIN_LENGTH, 5);
        assert_eq!(email::MAX_LENGTH, 255);
    }

    #[test]
    fn test_test_data_is_not_empty() {
        // Test data arrays have expected lengths
        assert_eq!(ip_address::TEST_IPV4_VALID.len(), 3); // 3 valid IPv4 addresses
        assert_eq!(ip_address::TEST_IPV6_VALID.len(), 3); // 3 valid IPv6 addresses
        assert_eq!(ip_address::TEST_INVALID.len(), 4); // 4 invalid addresses
        assert_eq!(environment_id::TEST_VALID.len(), 4); // 4 valid environment IDs
        assert_eq!(environment_id::TEST_INVALID.len(), 5); // 5 invalid environment IDs
    }

    #[test]
    fn test_validation_patterns_are_not_empty() {
        // Ensure patterns are defined with expected lengths
        assert!(tag::VALIDATION_PATTERN.len() > 10); // Regex patterns are longer
        assert!(environment_id::VALIDATION_PATTERN.len() > 10);
    }
}
