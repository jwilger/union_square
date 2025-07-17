//! Model version tracking and comparison types
//!
//! This module provides types for tracking model versions across different providers,
//! capturing version metadata with requests, and enabling version-aware testing.

use chrono::{DateTime, Utc};
use nutype::nutype;
use serde::{Deserialize, Serialize};
use std::fmt;

/// A validated model name that follows provider-specific naming conventions
#[nutype(
    validate(not_empty, len_char_max = 256),
    derive(
        Debug,
        Clone,
        PartialEq,
        Eq,
        Hash,
        AsRef,
        Deref,
        Serialize,
        Deserialize
    )
)]
pub struct ModelName(String);

/// A validated API version string
#[nutype(
    validate(not_empty, len_char_max = 64),
    derive(
        Debug,
        Clone,
        PartialEq,
        Eq,
        Hash,
        AsRef,
        Deref,
        Serialize,
        Deserialize
    )
)]
pub struct ApiVersion(String);

/// A validated model version string
#[nutype(
    validate(not_empty, len_char_max = 128),
    derive(
        Debug,
        Clone,
        PartialEq,
        Eq,
        Hash,
        AsRef,
        Deref,
        Serialize,
        Deserialize
    )
)]
pub struct ModelVersionString(String);

/// Extended model version information with provider-specific formats
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ExtendedModelVersion {
    /// The LLM provider (OpenAI, Anthropic, etc.)
    pub provider: crate::domain::LlmProvider,

    /// The model name (e.g., "gpt-4", "claude-3")
    pub model_name: ModelName,

    /// Provider-specific version information
    pub version_info: ProviderVersionInfo,

    /// When this version was first detected
    pub detected_at: DateTime<Utc>,
}

/// Provider-specific version information
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProviderVersionInfo {
    /// OpenAI version format
    OpenAI {
        /// Model version (e.g., "1106-preview", "0613")
        model_version: Option<ModelVersionString>,
        /// API version (e.g., "2023-12-01")
        api_version: ApiVersion,
        /// System fingerprint for exact model reproducibility
        system_fingerprint: Option<String>,
    },

    /// Anthropic version format
    Anthropic {
        /// Model version (e.g., "20240620")
        model_version: ModelVersionString,
        /// API version (e.g., "2023-06-01")
        api_version: ApiVersion,
        /// Model capabilities version
        capabilities_version: Option<String>,
    },

    /// AWS Bedrock version format
    Bedrock {
        /// Model ID (e.g., "anthropic.claude-3-sonnet-20240229-v1:0")
        model_id: String,
        /// Model ARN if available
        model_arn: Option<String>,
        /// Region where the model is deployed
        region: String,
    },

    /// Google Vertex AI version format
    VertexAI {
        /// Model name (e.g., "gemini-pro")
        model: ModelName,
        /// Model version
        version: ModelVersionString,
        /// Location/region
        location: String,
    },

    /// Generic format for other providers
    Other {
        /// Provider name
        provider_name: String,
        /// Version information as key-value pairs
        version_data: serde_json::Value,
    },
}

/// Version change event for EventCore
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VersionChangeEvent {
    /// Session where the version change was detected
    pub session_id: crate::domain::SessionId,

    /// Previous version (if any)
    pub from_version: Option<ExtendedModelVersion>,

    /// New version
    pub to_version: ExtendedModelVersion,

    /// When the change occurred
    pub occurred_at: DateTime<Utc>,

    /// Reason for version change (if known)
    pub reason: VersionChangeReason,
}

/// Reasons for model version changes
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VersionChangeReason {
    /// Initial version detection for session
    InitialDetection,

    /// Provider updated the model
    ProviderUpdate,

    /// User explicitly selected a different version
    UserSelection,

    /// Failover to different model/region
    Failover,

    /// A/B testing or experimentation
    Experiment { experiment_id: String },

    /// Other reason with description
    Other(String),
}

/// Configuration for version-aware test execution
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VersionTestConfig {
    /// Run tests against original recorded version
    pub use_original_version: bool,

    /// Specific version to test against (overrides original)
    pub target_version: Option<ExtendedModelVersion>,

    /// Compare results between versions
    pub comparison_mode: Option<VersionComparisonMode>,
}

/// How to compare results between versions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VersionComparisonMode {
    /// Side-by-side comparison
    SideBySide,

    /// Statistical analysis of differences
    Statistical {
        /// Confidence level for significance testing
        confidence_level: f64,
    },

    /// Check for specific compatibility issues
    CompatibilityCheck {
        /// Types of checks to perform
        checks: Vec<CompatibilityCheckType>,
    },
}

/// Types of compatibility checks
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CompatibilityCheckType {
    /// Response format compatibility
    ResponseFormat,

    /// Token usage differences
    TokenUsage,

    /// Performance regression
    Performance,

    /// Quality metrics (if available)
    Quality,
}

impl ExtendedModelVersion {
    /// Create a new ExtendedModelVersion
    pub fn new(
        provider: crate::domain::LlmProvider,
        model_name: ModelName,
        version_info: ProviderVersionInfo,
    ) -> Self {
        Self {
            provider,
            model_name,
            version_info,
            detected_at: Utc::now(),
        }
    }

    /// Get a display string for the version
    pub fn display_string(&self) -> String {
        match &self.version_info {
            ProviderVersionInfo::OpenAI {
                model_version,
                api_version,
                ..
            } => {
                format!(
                    "{}/{} (API: {})",
                    self.model_name.as_ref(),
                    model_version
                        .as_ref()
                        .map(|v| v.as_ref())
                        .unwrap_or("latest"),
                    api_version.as_ref()
                )
            }
            ProviderVersionInfo::Anthropic {
                model_version,
                api_version,
                ..
            } => {
                format!(
                    "{}-{} (API: {})",
                    self.model_name.as_ref(),
                    model_version.as_ref(),
                    api_version.as_ref()
                )
            }
            ProviderVersionInfo::Bedrock {
                model_id, region, ..
            } => {
                format!("{model_id} (Region: {region})")
            }
            ProviderVersionInfo::VertexAI {
                model,
                version,
                location,
            } => {
                format!(
                    "{}@{} (Location: {})",
                    model.as_ref(),
                    version.as_ref(),
                    location
                )
            }
            ProviderVersionInfo::Other { provider_name, .. } => {
                format!("{}/{}", provider_name, self.model_name.as_ref())
            }
        }
    }
}

impl fmt::Display for ExtendedModelVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_string())
    }
}

impl Default for VersionTestConfig {
    fn default() -> Self {
        Self {
            use_original_version: true,
            target_version: None,
            comparison_mode: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_name_validation() {
        assert!(ModelName::try_new("gpt-4").is_ok());
        assert!(ModelName::try_new("").is_err());
        assert!(ModelName::try_new("a".repeat(257)).is_err());
    }

    #[test]
    fn test_openai_version_display() {
        let version = ExtendedModelVersion::new(
            crate::domain::LlmProvider::OpenAI,
            ModelName::try_new("gpt-4").unwrap(),
            ProviderVersionInfo::OpenAI {
                model_version: Some(ModelVersionString::try_new("1106-preview").unwrap()),
                api_version: ApiVersion::try_new("2023-12-01").unwrap(),
                system_fingerprint: Some("fp_123456".to_string()),
            },
        );

        assert_eq!(
            version.display_string(),
            "gpt-4/1106-preview (API: 2023-12-01)"
        );
    }

    #[test]
    fn test_version_change_event_creation() {
        let session_id = crate::domain::SessionId::generate();
        let version = ExtendedModelVersion::new(
            crate::domain::LlmProvider::Anthropic,
            ModelName::try_new("claude-3").unwrap(),
            ProviderVersionInfo::Anthropic {
                model_version: ModelVersionString::try_new("20240620").unwrap(),
                api_version: ApiVersion::try_new("2023-06-01").unwrap(),
                capabilities_version: None,
            },
        );

        let event = VersionChangeEvent {
            session_id,
            from_version: None,
            to_version: version,
            occurred_at: Utc::now(),
            reason: VersionChangeReason::InitialDetection,
        };

        assert_eq!(event.reason, VersionChangeReason::InitialDetection);
        assert!(event.from_version.is_none());
    }
}
