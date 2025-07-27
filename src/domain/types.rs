//! Additional domain types for stronger type safety
//!
//! This module provides newtypes for common domain concepts to avoid
//! primitive obsession and ensure validation at boundaries.

use nutype::nutype;
#[allow(unused_imports)] // These are used by nutype derive macros
use serde::{Deserialize, Serialize};

/// User agent string from HTTP headers
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
    ),
    new_unchecked
)]
pub struct UserAgent(String);

/// IP address (v4 or v6)
#[nutype(
    validate(
        regex = r"^(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)$|^(([0-9a-fA-F]{1,4}:){7,7}[0-9a-fA-F]{1,4}|([0-9a-fA-F]{1,4}:){1,7}:|([0-9a-fA-F]{1,4}:){1,6}:[0-9a-fA-F]{1,4}|([0-9a-fA-F]{1,4}:){1,5}(:[0-9a-fA-F]{1,4}){1,2}|([0-9a-fA-F]{1,4}:){1,4}(:[0-9a-fA-F]{1,4}){1,3}|([0-9a-fA-F]{1,4}:){1,3}(:[0-9a-fA-F]{1,4}){1,4}|([0-9a-fA-F]{1,4}:){1,2}(:[0-9a-fA-F]{1,4}){1,5}|[0-9a-fA-F]{1,4}:((:[0-9a-fA-F]{1,4}){1,6})|:((:[0-9a-fA-F]{1,4}){1,7}|:)|fe80:(:[0-9a-fA-F]{0,4}){0,4}%[0-9a-zA-Z]{1,}|::(ffff(:0{1,4}){0,1}:){0,1}((25[0-5]|(2[0-4]|1{0,1}[0-9]){0,1}[0-9])\.){3,3}(25[0-5]|(2[0-4]|1{0,1}[0-9]){0,1}[0-9])|([0-9a-fA-F]{1,4}:){1,4}:((25[0-5]|(2[0-4]|1{0,1}[0-9]){0,1}[0-9])\.){3,3}(25[0-5]|(2[0-4]|1{0,1}[0-9]){0,1}[0-9]))$"
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
    ),
    new_unchecked
)]
pub struct IpAddress(String);

/// A tag for categorizing sessions, requests, etc.
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
    ),
    new_unchecked
)]
pub struct Tag(String);

/// Test case description
#[nutype(
    validate(not_empty, len_char_max = 1000),
    derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, AsRef, Display),
    new_unchecked
)]
pub struct TestCaseDescription(String);

/// Prompt template for test cases
#[nutype(
    validate(not_empty, len_char_max = 10000),
    derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, AsRef, Display),
    new_unchecked
)]
pub struct PromptTemplate(String);

/// Pattern for matching expected or forbidden content
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
    ),
    new_unchecked
)]
pub struct Pattern(String);

/// LLM response text
#[nutype(
    validate(len_char_max = 100000),
    derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, AsRef, Display),
    new_unchecked
)]
pub struct ResponseText(String);

/// Assertion description
#[nutype(
    validate(not_empty, len_char_max = 500),
    derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, AsRef, Display),
    new_unchecked
)]
pub struct AssertionDescription(String);

/// Error message
#[nutype(
    validate(not_empty, len_char_max = 5000),
    derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, AsRef, Display),
    new_unchecked
)]
pub struct ErrorMessage(String);

/// Model identifier from LLM provider
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
    ),
    new_unchecked
)]
pub struct ModelId(String);

/// LLM prompt text
#[nutype(
    validate(not_empty, len_char_max = 100000),
    derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, AsRef, Display),
    new_unchecked
)]
pub struct Prompt(String);

/// Finish reason from LLM response
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
    ),
    new_unchecked
)]
pub struct TokenCount(u32);

/// Cost in cents (to avoid floating point for money)
#[nutype(
    validate(less_or_equal = 100000000), // $1,000,000 max
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
    ),
    new_unchecked
)]
pub struct Cost(u32);

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
    ),
    new_unchecked
)]
pub struct Latency(u64);

/// Request count for tracking usage
#[nutype(
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
    ),
    new_unchecked
)]
pub struct RequestCount(u64);

/// Reason for version change or deactivation
#[nutype(
    validate(not_empty, len_char_max = 1000),
    derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, AsRef, Display),
    new_unchecked
)]
pub struct ChangeReason(String);

/// LLM request parameters as JSON
#[nutype(
    derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize),
    new_unchecked
)]
pub struct LlmParameters(serde_json::Value);

/// Test case metadata assertions as JSON
#[nutype(derive(Debug, Clone, PartialEq, Serialize, Deserialize), new_unchecked)]
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
