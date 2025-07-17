//! Version tracking service for managing model/API version information
//!
//! This service handles capturing, storing, and querying version information
//! for LLM models and APIs throughout the system.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::domain::{
    llm::ModelVersion,
    version::{TrackedVersion, VersionChangeEvent, VersionTestConfig},
    SessionId,
};
use crate::error::ApplicationError;

/// Service for tracking and managing model versions
pub struct VersionTrackingService {
    /// In-memory storage of tracked versions (will be replaced with EventCore)
    versions: Arc<RwLock<HashMap<String, TrackedVersion>>>,
    /// Version change events (will be stored in EventCore)
    change_events: Arc<RwLock<Vec<VersionChangeEvent>>>,
}

impl Default for VersionTrackingService {
    fn default() -> Self {
        Self::new()
    }
}

impl VersionTrackingService {
    /// Create a new version tracking service
    pub fn new() -> Self {
        Self {
            versions: Arc::new(RwLock::new(HashMap::new())),
            change_events: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Record a version being used in a request
    pub async fn record_version_usage(
        &self,
        _session_id: &SessionId,
        version: &ModelVersion,
    ) -> Result<(), ApplicationError> {
        let version_key = version.to_version_string();
        let mut versions = self.versions.write().await;

        if let Some(tracked) = versions.get_mut(&version_key) {
            tracked.record_usage();
        } else {
            versions.insert(version_key, TrackedVersion::new(version.clone()));
        }

        Ok(())
    }

    /// Record a version change during a session
    pub async fn record_version_change(
        &self,
        session_id: &SessionId,
        from_version: ModelVersion,
        to_version: ModelVersion,
        reason: Option<String>,
    ) -> Result<VersionChangeEvent, ApplicationError> {
        let event = VersionChangeEvent::new(session_id.clone(), from_version, to_version, reason);

        self.change_events.write().await.push(event.clone());

        Ok(event)
    }

    /// Get all tracked versions
    pub async fn get_tracked_versions(&self) -> Vec<TrackedVersion> {
        self.versions.read().await.values().cloned().collect()
    }

    /// Get active versions only
    pub async fn get_active_versions(&self) -> Vec<TrackedVersion> {
        self.versions
            .read()
            .await
            .values()
            .filter(|v| v.is_active)
            .cloned()
            .collect()
    }

    /// Get version change history for a session
    pub async fn get_session_version_changes(
        &self,
        session_id: &SessionId,
    ) -> Vec<VersionChangeEvent> {
        self.change_events
            .read()
            .await
            .iter()
            .filter(|e| &e.session_id == session_id)
            .cloned()
            .collect()
    }

    /// Deactivate a version
    pub async fn deactivate_version(&self, version: &ModelVersion) -> Result<(), ApplicationError> {
        let version_key = version.to_version_string();
        let mut versions = self.versions.write().await;

        if let Some(tracked) = versions.get_mut(&version_key) {
            tracked.deactivate();
            Ok(())
        } else {
            Err(ApplicationError::NotFound(format!(
                "Version {version_key} not found"
            )))
        }
    }

    /// Get version statistics
    pub async fn get_version_stats(&self) -> VersionStats {
        let versions = self.versions.read().await;
        let total_versions = versions.len();
        let active_versions = versions.values().filter(|v| v.is_active).count();
        let total_requests: u64 = versions.values().map(|v| v.request_count).sum();

        VersionStats {
            total_versions,
            active_versions,
            total_requests,
        }
    }
}

/// Version tracking statistics
#[derive(Debug, Clone, PartialEq)]
pub struct VersionStats {
    pub total_versions: usize,
    pub active_versions: usize,
    pub total_requests: u64,
}

/// Service for handling version-aware test execution
pub struct VersionTestService {
    tracking_service: Arc<VersionTrackingService>,
}

impl VersionTestService {
    /// Create a new version test service
    pub fn new(tracking_service: Arc<VersionTrackingService>) -> Self {
        Self { tracking_service }
    }

    /// Validate test configuration
    pub fn validate_config(&self, config: &VersionTestConfig) -> Result<(), ApplicationError> {
        use crate::domain::version::TestExecutionMode;

        match config.mode {
            TestExecutionMode::Target => {
                if config.target_version.is_none() {
                    return Err(ApplicationError::ValidationError(
                        "Target version required for Target mode".to_string(),
                    ));
                }
            }
            TestExecutionMode::Comparison => {
                if config.baseline_version.is_none() {
                    return Err(ApplicationError::ValidationError(
                        "Baseline version required for Comparison mode".to_string(),
                    ));
                }
                if config.target_version.is_none() {
                    return Err(ApplicationError::ValidationError(
                        "Target version required for Comparison mode".to_string(),
                    ));
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Determine which versions to test based on configuration
    pub async fn get_test_versions(
        &self,
        config: &VersionTestConfig,
        original_version: &ModelVersion,
    ) -> Result<Vec<ModelVersion>, ApplicationError> {
        use crate::domain::version::TestExecutionMode;

        self.validate_config(config)?;

        let versions = match config.mode {
            TestExecutionMode::Original => vec![original_version.clone()],
            TestExecutionMode::Target => vec![config
                .target_version
                .as_ref()
                .ok_or_else(|| {
                    ApplicationError::ValidationError("Target version not specified".to_string())
                })?
                .clone()],
            TestExecutionMode::Comparison => {
                let baseline = config.baseline_version.as_ref().unwrap_or(original_version);
                let target = config.target_version.as_ref().ok_or_else(|| {
                    ApplicationError::ValidationError("Target version not specified".to_string())
                })?;
                vec![baseline.clone(), target.clone()]
            }
            TestExecutionMode::AllVersions => {
                // Get all active versions
                let active_versions = self.tracking_service.get_active_versions().await;
                active_versions.into_iter().map(|tv| tv.version).collect()
            }
        };

        Ok(versions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::llm::LlmProvider;
    use crate::domain::version::TestExecutionMode;

    #[tokio::test]
    async fn test_version_tracking_service() {
        let service = VersionTrackingService::new();
        let session_id = SessionId::generate();
        let version = ModelVersion {
            provider: LlmProvider::OpenAI,
            model_name: "gpt-4".to_string(),
            version: Some("2024-01".to_string()),
            api_version: Some("v1".to_string()),
        };

        // Record usage
        service
            .record_version_usage(&session_id, &version)
            .await
            .unwrap();

        let tracked = service.get_tracked_versions().await;
        assert_eq!(tracked.len(), 1);
        assert_eq!(tracked[0].request_count, 1);

        // Record another usage
        service
            .record_version_usage(&session_id, &version)
            .await
            .unwrap();

        let tracked = service.get_tracked_versions().await;
        assert_eq!(tracked.len(), 1);
        assert_eq!(tracked[0].request_count, 2);
    }

    #[tokio::test]
    async fn test_version_change_tracking() {
        let service = VersionTrackingService::new();
        let session_id = SessionId::generate();

        let v1 = ModelVersion {
            provider: LlmProvider::OpenAI,
            model_name: "gpt-3.5-turbo".to_string(),
            version: Some("2024-01".to_string()),
            api_version: Some("v1".to_string()),
        };

        let v2 = ModelVersion {
            provider: LlmProvider::OpenAI,
            model_name: "gpt-4".to_string(),
            version: Some("2024-01".to_string()),
            api_version: Some("v1".to_string()),
        };

        let event = service
            .record_version_change(
                &session_id,
                v1.clone(),
                v2.clone(),
                Some("Upgrade for testing".to_string()),
            )
            .await
            .unwrap();

        assert_eq!(event.from_version, v1);
        assert_eq!(event.to_version, v2);

        let changes = service.get_session_version_changes(&session_id).await;
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].id, event.id);
    }

    #[tokio::test]
    async fn test_version_deactivation() {
        let service = VersionTrackingService::new();
        let session_id = SessionId::generate();
        let version = ModelVersion {
            provider: LlmProvider::OpenAI,
            model_name: "gpt-4".to_string(),
            version: Some("2024-01".to_string()),
            api_version: Some("v1".to_string()),
        };

        service
            .record_version_usage(&session_id, &version)
            .await
            .unwrap();

        let active = service.get_active_versions().await;
        assert_eq!(active.len(), 1);

        service.deactivate_version(&version).await.unwrap();

        let active = service.get_active_versions().await;
        assert_eq!(active.len(), 0);
    }

    #[tokio::test]
    async fn test_version_stats() {
        let service = VersionTrackingService::new();
        let session_id = SessionId::generate();

        let v1 = ModelVersion {
            provider: LlmProvider::OpenAI,
            model_name: "gpt-3.5-turbo".to_string(),
            version: Some("2024-01".to_string()),
            api_version: Some("v1".to_string()),
        };

        let v2 = ModelVersion {
            provider: LlmProvider::OpenAI,
            model_name: "gpt-4".to_string(),
            version: Some("2024-01".to_string()),
            api_version: Some("v1".to_string()),
        };

        service
            .record_version_usage(&session_id, &v1)
            .await
            .unwrap();
        service
            .record_version_usage(&session_id, &v1)
            .await
            .unwrap();
        service
            .record_version_usage(&session_id, &v2)
            .await
            .unwrap();

        let stats = service.get_version_stats().await;
        assert_eq!(stats.total_versions, 2);
        assert_eq!(stats.active_versions, 2);
        assert_eq!(stats.total_requests, 3);
    }

    #[tokio::test]
    async fn test_version_test_config_validation() {
        let tracking_service = Arc::new(VersionTrackingService::new());
        let test_service = VersionTestService::new(tracking_service);

        // Valid config for Target mode
        let config = VersionTestConfig {
            target_version: Some(ModelVersion {
                provider: LlmProvider::OpenAI,
                model_name: "gpt-4".to_string(),
                version: Some("2024-01".to_string()),
                api_version: Some("v1".to_string()),
            }),
            mode: TestExecutionMode::Target,
            ..Default::default()
        };
        assert!(test_service.validate_config(&config).is_ok());

        // Invalid config for Target mode (missing target)
        let config = VersionTestConfig {
            mode: TestExecutionMode::Target,
            ..Default::default()
        };
        assert!(test_service.validate_config(&config).is_err());

        // Invalid config for Comparison mode (missing versions)
        let config = VersionTestConfig {
            mode: TestExecutionMode::Comparison,
            ..Default::default()
        };
        assert!(test_service.validate_config(&config).is_err());
    }

    #[tokio::test]
    async fn test_get_test_versions() {
        let tracking_service = Arc::new(VersionTrackingService::new());
        let test_service = VersionTestService::new(tracking_service.clone());

        let original = ModelVersion {
            provider: LlmProvider::OpenAI,
            model_name: "gpt-3.5-turbo".to_string(),
            version: Some("2024-01".to_string()),
            api_version: Some("v1".to_string()),
        };

        let target = ModelVersion {
            provider: LlmProvider::OpenAI,
            model_name: "gpt-4".to_string(),
            version: Some("2024-01".to_string()),
            api_version: Some("v1".to_string()),
        };

        // Test Original mode
        let config = VersionTestConfig {
            mode: TestExecutionMode::Original,
            ..Default::default()
        };
        let versions = test_service
            .get_test_versions(&config, &original)
            .await
            .unwrap();
        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0], original);

        // Test Target mode
        let config = VersionTestConfig {
            target_version: Some(target.clone()),
            mode: TestExecutionMode::Target,
            ..Default::default()
        };
        let versions = test_service
            .get_test_versions(&config, &original)
            .await
            .unwrap();
        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0], target);

        // Test Comparison mode
        let config = VersionTestConfig {
            target_version: Some(target.clone()),
            baseline_version: Some(original.clone()),
            mode: TestExecutionMode::Comparison,
            compare_mode: true,
        };
        let versions = test_service
            .get_test_versions(&config, &original)
            .await
            .unwrap();
        assert_eq!(versions.len(), 2);
        assert_eq!(versions[0], original);
        assert_eq!(versions[1], target);
    }
}
