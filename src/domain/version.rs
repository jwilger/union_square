//! Version tracking and management for model/API versions
//!
//! This module provides comprehensive version tracking capabilities for LLM models
//! and APIs, supporting version comparison, change detection, and test configuration.

use chrono::{DateTime, Utc};
use nutype::nutype;
use serde::{Deserialize, Serialize};

use crate::domain::llm::{LlmProvider, ModelVersion};

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
    pub request_count: u64,
    pub is_active: bool,
}

impl TrackedVersion {
    pub fn new(version: ModelVersion) -> Self {
        let now = Utc::now();
        Self {
            version,
            first_seen: now,
            last_seen: now,
            request_count: 1,
            is_active: true,
        }
    }

    pub fn record_usage(&mut self) {
        self.last_seen = Utc::now();
        self.request_count += 1;
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
        from_model_id: String,
        to_provider: LlmProvider,
        to_model_id: String,
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
        format!("{}/{}", self.provider.as_str(), self.model_id)
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
    pub reason: Option<String>,
}

impl VersionChangeEvent {
    pub fn new(
        session_id: crate::domain::SessionId,
        from_version: ModelVersion,
        to_version: ModelVersion,
        reason: Option<String>,
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

    #[test]
    fn test_version_comparison_same() {
        let v1 = ModelVersion {
            provider: LlmProvider::OpenAI,
            model_id: "gpt-4-turbo-2024-01".to_string(),
        };

        let v2 = v1.clone();
        assert_eq!(v1.compare(&v2), VersionComparison::Same);
    }

    #[test]
    fn test_version_comparison_changed() {
        let v1 = ModelVersion {
            provider: LlmProvider::OpenAI,
            model_id: "gpt-3.5-turbo".to_string(),
        };

        let v2 = ModelVersion {
            provider: LlmProvider::OpenAI,
            model_id: "gpt-4-turbo-2024-01".to_string(),
        };

        assert_eq!(
            v1.compare(&v2),
            VersionComparison::Changed {
                from_provider: LlmProvider::OpenAI,
                from_model_id: "gpt-3.5-turbo".to_string(),
                to_provider: LlmProvider::OpenAI,
                to_model_id: "gpt-4-turbo-2024-01".to_string(),
            }
        );
    }

    #[test]
    fn test_version_comparison_provider_changed() {
        let v1 = ModelVersion {
            provider: LlmProvider::OpenAI,
            model_id: "gpt-4-turbo".to_string(),
        };

        let v2 = ModelVersion {
            provider: LlmProvider::Anthropic,
            model_id: "claude-3-opus-20240229".to_string(),
        };

        assert_eq!(
            v1.compare(&v2),
            VersionComparison::Changed {
                from_provider: LlmProvider::OpenAI,
                from_model_id: "gpt-4-turbo".to_string(),
                to_provider: LlmProvider::Anthropic,
                to_model_id: "claude-3-opus-20240229".to_string(),
            }
        );
    }

    #[test]
    fn test_tracked_version_usage() {
        let version = ModelVersion {
            provider: LlmProvider::OpenAI,
            model_id: "gpt-4-turbo-2024-01".to_string(),
        };

        let mut tracked = TrackedVersion::new(version);
        assert_eq!(tracked.request_count, 1);
        assert!(tracked.is_active);

        tracked.record_usage();
        assert_eq!(tracked.request_count, 2);

        tracked.deactivate();
        assert!(!tracked.is_active);
    }

    #[test]
    fn test_version_string_formatting() {
        let version = ModelVersion {
            provider: LlmProvider::OpenAI,
            model_id: "gpt-4-turbo-2024-01".to_string(),
        };

        assert_eq!(version.to_version_string(), "openai/gpt-4-turbo-2024-01");

        let version_anthropic = ModelVersion {
            provider: LlmProvider::Anthropic,
            model_id: "claude-3-opus-20240229".to_string(),
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
            model_id: "gpt-3.5-turbo".to_string(),
        };

        let v2 = ModelVersion {
            provider: LlmProvider::OpenAI,
            model_id: "gpt-4-turbo-2024-01".to_string(),
        };

        let event = VersionChangeEvent::new(
            session_id,
            v1.clone(),
            v2.clone(),
            Some("Upgrade for better performance".to_string()),
        );

        assert_eq!(event.from_version, v1);
        assert_eq!(event.to_version, v2);
        assert_eq!(
            event.change_type,
            VersionComparison::Changed {
                from_provider: LlmProvider::OpenAI,
                from_model_id: "gpt-3.5-turbo".to_string(),
                to_provider: LlmProvider::OpenAI,
                to_model_id: "gpt-4-turbo-2024-01".to_string(),
            }
        );
        assert_eq!(
            event.reason,
            Some("Upgrade for better performance".to_string())
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
}
