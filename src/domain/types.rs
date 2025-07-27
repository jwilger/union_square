//! Additional domain types for stronger type safety
//!
//! This module provides newtypes for common domain concepts to avoid
//! primitive obsession and ensure validation at boundaries.

use nutype::nutype;
#[allow(unused_imports)] // These are used by nutype derive macros
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// User agent string from HTTP headers
///
/// Limited to 1000 characters to prevent DoS attacks and database storage issues
/// while accommodating most real-world user agent strings.
#[nutype(
    validate(not_empty, len_char_max = 1000),
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
pub struct UserAgent(String);

/// IP address (v4 or v6)
#[nutype(
    validate(predicate = |s| std::net::IpAddr::from_str(s).is_ok()),
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
pub struct IpAddress(String);

/// A tag for categorizing sessions, requests, etc.
///
/// Limited to 100 characters to maintain reasonable tag lengths for display
/// and indexing purposes while allowing descriptive categorization.
#[nutype(
    validate(
        not_empty,
        len_char_max = 100,
        regex = r"^[a-zA-Z0-9][a-zA-Z0-9:._-]*$"
    ),
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
pub struct Tag(String);

/// Test case description
///
/// Limited to 1000 characters to ensure descriptions remain concise and focused
/// while providing enough space for detailed test scenarios.
#[nutype(
    validate(not_empty, len_char_max = 1000),
    derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, AsRef, Display)
)]
pub struct TestCaseDescription(String);

/// Prompt template for test cases
///
/// Limited to 10,000 characters to support complex prompts with multiple examples
/// while preventing excessive memory usage. This aligns with typical LLM context limits.
#[nutype(
    validate(not_empty, len_char_max = 10000),
    derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, AsRef, Display)
)]
pub struct PromptTemplate(String);

/// Pattern for matching expected or forbidden content
///
/// Limited to 1000 characters to support reasonable regex/text patterns
/// while preventing performance issues with overly complex patterns.
#[nutype(
    validate(not_empty, len_char_max = 1000),
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
pub struct Pattern(String);

/// LLM response text
///
/// Limited to 100,000 characters (~25k tokens) to accommodate extensive LLM responses
/// while maintaining reasonable memory bounds. Most LLM APIs have lower limits than this.
#[nutype(
    validate(len_char_max = 100000),
    derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, AsRef, Display)
)]
pub struct ResponseText(String);

/// Assertion description
///
/// Limited to 500 characters to ensure clear, focused assertion descriptions
/// suitable for test result reporting and debugging.
#[nutype(
    validate(not_empty, len_char_max = 500),
    derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, AsRef, Display)
)]
pub struct AssertionDescription(String);

/// Error message
///
/// Limited to 5000 characters to capture detailed error information including
/// stack traces while preventing excessive log/storage usage.
#[nutype(
    validate(not_empty, len_char_max = 5000),
    derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, AsRef, Display)
)]
pub struct ErrorMessage(String);

/// Model identifier from LLM provider
///
/// Limited to 200 characters to accommodate all known LLM model identifiers
/// (e.g., "claude-3-opus-20240229") with room for future expansion.
#[nutype(
    validate(not_empty, len_char_max = 200),
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
pub struct ModelId(String);

/// LLM prompt text
///
/// Limited to 100,000 characters to support extensive prompts with context
/// while staying within typical LLM context window limits.
#[nutype(
    validate(not_empty, len_char_max = 100000),
    derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, AsRef, Display)
)]
pub struct Prompt(String);

/// Finish reason from LLM response
///
/// Limited to 100 characters to accommodate all known finish reason values
/// (e.g., "stop", "length", "content_filter") with margin for new reasons.
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
pub struct FinishReason(String);

/// Token count for LLM usage
#[nutype(
    validate(less_or_equal = 1000000),
    derive(
        Debug,
        Clone,
        Copy,
        PartialEq,
        Eq,
        PartialOrd,
        Ord,
        Hash,
        Serialize,
        Deserialize,
        AsRef,
        Display
    )
)]
pub struct TokenCount(u32);

/// Latency in milliseconds
#[nutype(
    validate(less_or_equal = 300000), // 5 minutes max
    derive(
        Debug,
        Clone,
        Copy,
        PartialEq,
        Eq,
        PartialOrd,
        Ord,
        Hash,
        Serialize,
        Deserialize,
        AsRef,
        Display
    )
)]
pub struct Latency(u64);

/// Request count for tracking usage
#[nutype(derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    AsRef,
    Display
))]
pub struct RequestCount(u64);

impl RequestCount {
    /// Increment the count by one, saturating at the maximum value
    pub fn increment(self) -> Self {
        Self::new(self.as_ref().saturating_add(1))
    }
}

/// Reason for version change or deactivation
///
/// Limited to 1000 characters to capture meaningful change rationales
/// for audit purposes while maintaining reasonable storage constraints.
#[nutype(
    validate(not_empty, len_char_max = 1000),
    derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, AsRef, Display)
)]
pub struct ChangeReason(String);

/// LLM request parameters as JSON
#[nutype(derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize))]
pub struct LlmParameters(serde_json::Value);

/// Test case metadata assertions as JSON
#[nutype(derive(Debug, Clone, PartialEq, Serialize, Deserialize))]
pub struct MetadataAssertions(serde_json::Value);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_agent_validation() {
        assert!(UserAgent::try_new("Mozilla/5.0".to_string()).is_ok());
        assert!(UserAgent::try_new("".to_string()).is_err());
        assert!(UserAgent::try_new("a".repeat(1001)).is_err());
    }

    #[test]
    fn test_ip_address_validation() {
        // Valid IPv4
        assert!(IpAddress::try_new("192.168.1.1".to_string()).is_ok());
        assert!(IpAddress::try_new("10.0.0.0".to_string()).is_ok());
        assert!(IpAddress::try_new("255.255.255.255".to_string()).is_ok());

        // Valid IPv6
        assert!(IpAddress::try_new("2001:0db8:85a3:0000:0000:8a2e:0370:7334".to_string()).is_ok());
        assert!(IpAddress::try_new("::1".to_string()).is_ok());
        assert!(IpAddress::try_new("fe80::1".to_string()).is_ok());

        // Invalid
        assert!(IpAddress::try_new("256.1.1.1".to_string()).is_err());
        assert!(IpAddress::try_new("192.168.1".to_string()).is_err());
        assert!(IpAddress::try_new("not-an-ip".to_string()).is_err());
        assert!(IpAddress::try_new("".to_string()).is_err());
    }

    #[test]
    fn test_tag_validation() {
        assert!(Tag::try_new("production".to_string()).is_ok());
        assert!(Tag::try_new("api:v2".to_string()).is_ok());
        assert!(Tag::try_new("test-case_1".to_string()).is_ok());
        assert!(Tag::try_new("feature.enabled".to_string()).is_ok());

        assert!(Tag::try_new("".to_string()).is_err());
        assert!(Tag::try_new("-invalid".to_string()).is_err());
        assert!(Tag::try_new("invalid ".to_string()).is_err());
        assert!(Tag::try_new("a".repeat(101)).is_err());
    }

    #[test]
    fn test_prompt_template_validation() {
        assert!(PromptTemplate::try_new("Hello {name}!".to_string()).is_ok());
        assert!(PromptTemplate::try_new("".to_string()).is_err());
        assert!(PromptTemplate::try_new("a".repeat(10001)).is_err());
    }

    #[test]
    fn test_pattern_validation() {
        assert!(Pattern::try_new("expected output".to_string()).is_ok());
        assert!(Pattern::try_new("".to_string()).is_err());
        assert!(Pattern::try_new("a".repeat(1001)).is_err());
    }
}
