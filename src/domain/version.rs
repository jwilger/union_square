//! Version tracking and management for model/API versions
//!
//! This module provides comprehensive version tracking capabilities for LLM models
//! and APIs, supporting version comparison, change detection, and test configuration.

use chrono::{DateTime, Utc};
use nutype::nutype;
use serde::{Deserialize, Serialize};

use crate::domain::llm::{LlmProvider, ModelVersion};
use crate::domain::types::{ChangeReason, ModelId, RequestCount};

/// Unique identifier for a version change event
#[nutype(derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize))]
pub struct VersionChangeId(uuid::Uuid);

impl VersionChangeId {
    pub fn generate() -> Self {
        Self::new(uuid::Uuid::now_v7())
    }
}

/// Represents a tracked version with additional metadata
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrackedVersion {
    pub version: ModelVersion,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub request_count: RequestCount,
    pub is_active: bool,
}

impl TrackedVersion {
    pub fn new(version: ModelVersion) -> Self {
        let now = Utc::now();
        Self {
            version,
            first_seen: now,
            last_seen: now,
            request_count: RequestCount::new(1),
            is_active: true,
        }
    }

    pub fn record_usage(&mut self) {
        self.last_seen = Utc::now();
        self.request_count = self.request_count.increment();
    }

    pub fn deactivate(&mut self) {
        self.is_active = false;
    }
}

/// Version comparison result
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VersionComparison {
    Same,
    Changed {
        from_provider: LlmProvider,
        from_model_id: ModelId,
        to_provider: LlmProvider,
        to_model_id: ModelId,
    },
}

impl ModelVersion {
    /// Compare this version with another
    pub fn compare(&self, other: &ModelVersion) -> VersionComparison {
        if self.provider == other.provider && self.model_id == other.model_id {
            VersionComparison::Same
        } else {
            VersionComparison::Changed {
                from_provider: self.provider.clone(),
                from_model_id: self.model_id.clone(),
                to_provider: other.provider.clone(),
                to_model_id: other.model_id.clone(),
            }
        }
    }

    /// Create a version identifier string for display
    pub fn to_version_string(&self) -> String {
        format!("{}/{}", self.provider.as_str(), self.model_id.as_ref())
    }
}

/// Configuration for version-aware test execution
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VersionTestConfig {
    /// Version to use for replay (if different from original)
    pub target_version: Option<ModelVersion>,
    /// Whether to compare results between versions
    pub compare_mode: bool,
    /// Original version for comparison
    pub baseline_version: Option<ModelVersion>,
    /// Test execution mode
    pub mode: TestExecutionMode,
}

/// How to execute version-aware tests
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TestExecutionMode {
    /// Run against original version only
    Original,
    /// Run against target version only
    Target,
    /// Run against both and compare
    Comparison,
    /// Run against all available versions
    AllVersions,
}

impl Default for VersionTestConfig {
    fn default() -> Self {
        Self {
            target_version: None,
            compare_mode: false,
            baseline_version: None,
            mode: TestExecutionMode::Original,
        }
    }
}

/// Version change event for tracking in the system
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VersionChangeEvent {
    pub id: VersionChangeId,
    pub session_id: crate::domain::SessionId,
    pub from_version: ModelVersion,
    pub to_version: ModelVersion,
    pub change_type: VersionComparison,
    pub occurred_at: DateTime<Utc>,
    pub reason: Option<ChangeReason>,
}

impl VersionChangeEvent {
    pub fn new(
        session_id: crate::domain::SessionId,
        from_version: ModelVersion,
        to_version: ModelVersion,
        reason: Option<ChangeReason>,
    ) -> Self {
        let change_type = from_version.compare(&to_version);
        Self {
            id: VersionChangeId::generate(),
            session_id,
            from_version,
            to_version,
            change_type,
            occurred_at: Utc::now(),
            reason,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::SessionId;
    use quickcheck::{Arbitrary, Gen};
    use quickcheck_macros::quickcheck;

    #[test]
    fn test_version_comparison_same() {
        let v1 = ModelVersion {
            provider: LlmProvider::OpenAI,
            model_id: ModelId::try_new("gpt-4-turbo-2024-01".to_string()).unwrap(),
        };

        let v2 = v1.clone();
        assert_eq!(v1.compare(&v2), VersionComparison::Same);
    }

    #[test]
    fn test_version_comparison_changed() {
        let v1 = ModelVersion {
            provider: LlmProvider::OpenAI,
            model_id: ModelId::try_new("gpt-3.5-turbo".to_string()).unwrap(),
        };

        let v2 = ModelVersion {
            provider: LlmProvider::OpenAI,
            model_id: ModelId::try_new("gpt-4-turbo-2024-01".to_string()).unwrap(),
        };

        assert_eq!(
            v1.compare(&v2),
            VersionComparison::Changed {
                from_provider: LlmProvider::OpenAI,
                from_model_id: ModelId::try_new("gpt-3.5-turbo".to_string()).unwrap(),
                to_provider: LlmProvider::OpenAI,
                to_model_id: ModelId::try_new("gpt-4-turbo-2024-01".to_string()).unwrap(),
            }
        );
    }

    #[test]
    fn test_version_comparison_provider_changed() {
        let v1 = ModelVersion {
            provider: LlmProvider::OpenAI,
            model_id: ModelId::try_new("gpt-4-turbo".to_string()).unwrap(),
        };

        let v2 = ModelVersion {
            provider: LlmProvider::Anthropic,
            model_id: ModelId::try_new("claude-3-opus-20240229".to_string()).unwrap(),
        };

        assert_eq!(
            v1.compare(&v2),
            VersionComparison::Changed {
                from_provider: LlmProvider::OpenAI,
                from_model_id: ModelId::try_new("gpt-4-turbo".to_string()).unwrap(),
                to_provider: LlmProvider::Anthropic,
                to_model_id: ModelId::try_new("claude-3-opus-20240229".to_string()).unwrap(),
            }
        );
    }

    #[test]
    fn test_tracked_version_usage() {
        let version = ModelVersion {
            provider: LlmProvider::OpenAI,
            model_id: ModelId::try_new("gpt-4-turbo-2024-01".to_string()).unwrap(),
        };

        let mut tracked = TrackedVersion::new(version);
        assert_eq!(tracked.request_count, RequestCount::new(1));
        assert!(tracked.is_active);

        tracked.record_usage();
        assert_eq!(tracked.request_count, RequestCount::new(2));

        tracked.deactivate();
        assert!(!tracked.is_active);
    }

    #[test]
    fn test_version_string_formatting() {
        let version = ModelVersion {
            provider: LlmProvider::OpenAI,
            model_id: ModelId::try_new("gpt-4-turbo-2024-01".to_string()).unwrap(),
        };

        assert_eq!(version.to_version_string(), "openai/gpt-4-turbo-2024-01");

        let version_anthropic = ModelVersion {
            provider: LlmProvider::Anthropic,
            model_id: ModelId::try_new("claude-3-opus-20240229".to_string()).unwrap(),
        };

        assert_eq!(
            version_anthropic.to_version_string(),
            "anthropic/claude-3-opus-20240229"
        );
    }

    #[test]
    fn test_version_change_event_creation() {
        let session_id = SessionId::generate();
        let v1 = ModelVersion {
            provider: LlmProvider::OpenAI,
            model_id: ModelId::try_new("gpt-3.5-turbo".to_string()).unwrap(),
        };

        let v2 = ModelVersion {
            provider: LlmProvider::OpenAI,
            model_id: ModelId::try_new("gpt-4-turbo-2024-01".to_string()).unwrap(),
        };

        let event = VersionChangeEvent::new(
            session_id,
            v1.clone(),
            v2.clone(),
            Some(ChangeReason::try_new("Upgrade for better performance".to_string()).unwrap()),
        );

        assert_eq!(event.from_version, v1);
        assert_eq!(event.to_version, v2);
        assert_eq!(
            event.change_type,
            VersionComparison::Changed {
                from_provider: LlmProvider::OpenAI,
                from_model_id: ModelId::try_new("gpt-3.5-turbo".to_string()).unwrap(),
                to_provider: LlmProvider::OpenAI,
                to_model_id: ModelId::try_new("gpt-4-turbo-2024-01".to_string()).unwrap(),
            }
        );
        assert_eq!(
            event.reason.as_ref().map(|r| r.as_ref()),
            Some("Upgrade for better performance")
        );
    }

    #[test]
    fn test_version_test_config_default() {
        let config = VersionTestConfig::default();
        assert_eq!(config.mode, TestExecutionMode::Original);
        assert!(!config.compare_mode);
        assert!(config.target_version.is_none());
        assert!(config.baseline_version.is_none());
    }

    // New comprehensive tests for type-safe VersionComparison

    #[test]
    fn test_version_comparison_should_use_model_id_not_string() {
        // This test demonstrates that VersionComparison should contain ModelId types,
        // not raw strings. The current implementation incorrectly uses String.
        let v1 = ModelVersion {
            provider: LlmProvider::OpenAI,
            model_id: ModelId::try_new("gpt-4".to_string()).unwrap(),
        };
        let v2 = ModelVersion {
            provider: LlmProvider::Anthropic,
            model_id: ModelId::try_new("claude-3-opus".to_string()).unwrap(),
        };

        match v1.compare(&v2) {
            VersionComparison::Changed {
                ref from_model_id,
                ref to_model_id,
                ..
            } => {
                // Now these are ModelId types, preserving all validation guarantees
                assert_eq!(from_model_id.as_ref(), "gpt-4");
                assert_eq!(to_model_id.as_ref(), "claude-3-opus");

                // We CAN now do this - they are proper ModelId types!
                let _model_id: &ModelId = from_model_id; // This compiles!

                // The type system now prevents invalid VersionComparisons
            }
            _ => panic!("Expected Changed variant"),
        }
    }

    #[test]
    fn test_version_comparison_preserves_model_id_validation() {
        // Test that the ModelId validation is preserved in VersionComparison
        let v1 = ModelVersion {
            provider: LlmProvider::OpenAI,
            model_id: ModelId::try_new("gpt-4-turbo-2024-01".to_string()).unwrap(),
        };
        let v2 = ModelVersion {
            provider: LlmProvider::OpenAI,
            model_id: ModelId::try_new("gpt-4-turbo-2024-02".to_string()).unwrap(),
        };

        let comparison = v1.compare(&v2);

        // Once we fix the implementation, this should work with ModelId types
        match comparison {
            VersionComparison::Changed {
                ref from_model_id,
                ref to_model_id,
                ..
            } => {
                // Now using ModelId types with proper validation
                assert_eq!(from_model_id.as_ref(), "gpt-4-turbo-2024-01");
                assert_eq!(to_model_id.as_ref(), "gpt-4-turbo-2024-02");
            }
            _ => panic!("Expected Changed variant"),
        }
    }

    #[test]
    fn test_version_comparison_same_preserves_types() {
        // Ensure Same variant works correctly (no changes needed for this variant)
        let model_id = ModelId::try_new("gpt-4".to_string()).unwrap();
        let v1 = ModelVersion {
            provider: LlmProvider::OpenAI,
            model_id: model_id.clone(),
        };
        let v2 = v1.clone();

        assert_eq!(v1.compare(&v2), VersionComparison::Same);
    }

    #[test]
    fn test_version_comparison_serialization_with_model_id() {
        // Test that serialization works correctly with ModelId types
        let v1 = ModelVersion {
            provider: LlmProvider::OpenAI,
            model_id: ModelId::try_new("gpt-3.5".to_string()).unwrap(),
        };
        let v2 = ModelVersion {
            provider: LlmProvider::Anthropic,
            model_id: ModelId::try_new("claude-2".to_string()).unwrap(),
        };

        let comparison = v1.compare(&v2);

        // Serialize and deserialize
        let json = serde_json::to_string(&comparison).unwrap();
        let deserialized: VersionComparison = serde_json::from_str(&json).unwrap();

        assert_eq!(comparison, deserialized);

        // Verify the JSON format is as expected (ModelId serializes as a string)
        let expected_json = r#"{"Changed":{"from_provider":"OpenAI","from_model_id":"gpt-3.5","to_provider":"Anthropic","to_model_id":"claude-2"}}"#;
        assert_eq!(json, expected_json);
    }

    // Property-based tests for type safety

    #[derive(Clone, Debug)]
    struct ValidModelIdString(String);

    impl Arbitrary for ValidModelIdString {
        fn arbitrary(g: &mut Gen) -> Self {
            // Generate valid model IDs that pass validation
            let providers = vec![
                "gpt-3.5-turbo",
                "gpt-4",
                "claude-2",
                "claude-3-opus",
                "llama-2-70b",
            ];
            let base = g.choose(&providers).unwrap();
            let suffix = if bool::arbitrary(g) {
                format!("-{}", u32::arbitrary(g) % 100)
            } else {
                String::new()
            };
            ValidModelIdString(format!("{base}{suffix}"))
        }
    }

    #[quickcheck]
    fn test_property_version_comparison_type_safety(
        id1: ValidModelIdString,
        id2: ValidModelIdString,
    ) -> bool {
        // Property: All valid ModelIds should be preserved in VersionComparison
        let model_id1 = ModelId::try_new(id1.0.clone()).unwrap();
        let model_id2 = ModelId::try_new(id2.0.clone()).unwrap();

        let v1 = ModelVersion {
            provider: LlmProvider::OpenAI,
            model_id: model_id1.clone(),
        };
        let v2 = ModelVersion {
            provider: LlmProvider::Anthropic,
            model_id: model_id2.clone(),
        };

        match v1.compare(&v2) {
            VersionComparison::Same => {
                // Should only be Same if providers and model_ids are equal
                v1.provider == v2.provider && model_id1 == model_id2
            }
            VersionComparison::Changed {
                ref from_model_id,
                ref to_model_id,
                ..
            } => {
                // Now these are ModelId types with proper equality
                from_model_id == &model_id1 && to_model_id == &model_id2
            }
        }
    }

    #[quickcheck]
    fn test_property_version_comparison_roundtrip_serialization(
        id1: ValidModelIdString,
        id2: ValidModelIdString,
        same_provider: bool,
    ) -> bool {
        // Property: VersionComparison should roundtrip through serialization
        let model_id1 = ModelId::try_new(id1.0).unwrap();
        let model_id2 = ModelId::try_new(id2.0).unwrap();

        let provider1 = LlmProvider::OpenAI;
        let provider2 = if same_provider {
            LlmProvider::OpenAI
        } else {
            LlmProvider::Anthropic
        };

        let v1 = ModelVersion {
            provider: provider1,
            model_id: model_id1,
        };
        let v2 = ModelVersion {
            provider: provider2,
            model_id: model_id2,
        };

        let original = v1.compare(&v2);
        let json = serde_json::to_string(&original).unwrap();
        let deserialized: VersionComparison = serde_json::from_str(&json).unwrap();

        original == deserialized
    }

    #[test]
    fn test_version_change_event_with_type_safe_comparison() {
        // Test that VersionChangeEvent correctly uses the type-safe comparison
        let session_id = SessionId::generate();
        let v1 = ModelVersion {
            provider: LlmProvider::OpenAI,
            model_id: ModelId::try_new("gpt-3.5-turbo".to_string()).unwrap(),
        };
        let v2 = ModelVersion {
            provider: LlmProvider::OpenAI,
            model_id: ModelId::try_new("gpt-4".to_string()).unwrap(),
        };

        let event = VersionChangeEvent::new(
            session_id,
            v1.clone(),
            v2.clone(),
            Some(ChangeReason::try_new("Upgrade".to_string()).unwrap()),
        );

        // The change_type should use the type-safe comparison
        match &event.change_type {
            VersionComparison::Changed {
                from_model_id,
                to_model_id,
                ..
            } => {
                // Now properly using ModelId types
                assert_eq!(from_model_id.as_ref(), "gpt-3.5-turbo");
                assert_eq!(to_model_id.as_ref(), "gpt-4");
            }
            _ => panic!("Expected Changed variant"),
        }
    }

    #[test]
    fn test_empty_model_id_should_not_be_possible_in_comparison() {
        // This test verifies that empty model IDs cannot exist in VersionComparison
        // because ModelId validation prevents empty strings
        assert!(ModelId::try_new("".to_string()).is_err());

        // Therefore, it's impossible to create a VersionComparison with empty model IDs
        // when using the type-safe approach
    }

    #[test]
    fn test_version_comparison_cannot_bypass_validation_through_deserialization() {
        // This test verifies that we CANNOT create invalid VersionComparisons
        // with empty or invalid model IDs by deserializing JSON

        // This JSON contains empty model IDs which are invalid
        let invalid_json = r#"{
            "Changed": {
                "from_provider": "OpenAI",
                "from_model_id": "",
                "to_provider": "Anthropic",
                "to_model_id": ""
            }
        }"#;

        // This now CORRECTLY fails because ModelId validation prevents empty strings
        let result: Result<VersionComparison, _> = serde_json::from_str(invalid_json);

        // The fix works: Deserialization fails for invalid model IDs!
        assert!(
            result.is_err(),
            "Deserialization correctly rejects empty model IDs"
        );
    }

    #[test]
    fn test_excessively_long_model_id_should_not_be_possible() {
        // ModelId has a max length of 200 characters
        let long_id = "a".repeat(201);
        assert!(ModelId::try_new(long_id).is_err());

        // Therefore, VersionComparison cannot contain invalid long model IDs
        // when using the type-safe approach
    }

    #[test]
    fn test_version_comparison_cannot_bypass_length_validation() {
        // This test verifies that we CANNOT create model IDs longer than 200 chars
        let long_id = "a".repeat(201);

        let invalid_json = format!(
            r#"{{
            "Changed": {{
                "from_provider": "OpenAI",
                "from_model_id": "{long_id}",
                "to_provider": "Anthropic",
                "to_model_id": "{long_id}"
            }}
        }}"#
        );

        // This now CORRECTLY fails due to ModelId length validation
        let result: Result<VersionComparison, _> = serde_json::from_str(&invalid_json);

        // The fix works: Deserialization fails for model IDs exceeding the limit!
        assert!(
            result.is_err(),
            "Deserialization correctly rejects model IDs exceeding length limit"
        );
    }
}
