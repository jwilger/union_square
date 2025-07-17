//! Test configuration and execution with version awareness
//!
//! This module provides functionality for configuring and executing tests
//! with specific model versions, enabling version-aware testing scenarios.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::{
    application::event_store::EventStore,
    domain::{
        DefaultVersionCapture, DomainEvent, ExtendedModelVersion, LlmProvider, SessionId,
        VersionCapture, VersionChangeReason, VersionTestConfig,
    },
    error::Result,
};

/// Test execution context that manages version selection
pub struct TestExecutionContext {
    /// The configuration for this test run
    pub config: VersionTestConfig,

    /// The session being tested
    pub session_id: SessionId,

    /// Version capture implementation
    #[allow(dead_code)]
    version_capture: Box<dyn VersionCapture + Send + Sync>,
}

/// Result of a test execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestExecutionResult {
    /// The session that was tested
    pub session_id: SessionId,

    /// Version used for the test
    pub version_used: ExtendedModelVersion,

    /// Whether the test passed
    pub passed: bool,

    /// Any error message if the test failed
    pub error_message: Option<String>,

    /// Comparison results if comparison mode was enabled
    pub comparison_results: Option<ComparisonResults>,
}

/// Results from comparing different model versions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonResults {
    /// Original version that was recorded
    pub original_version: ExtendedModelVersion,

    /// Target version that was tested
    pub target_version: ExtendedModelVersion,

    /// Detailed comparison data
    pub comparison_data: serde_json::Value,
}

/// Service for executing version-aware tests
#[async_trait]
pub trait TestExecutor: Send + Sync {
    /// Execute a test with the given context
    async fn execute_test(
        &self,
        context: &TestExecutionContext,
        request_payload: &serde_json::Value,
    ) -> Result<TestExecutionResult>;

    /// Get the appropriate model version for the test
    async fn resolve_version(
        &self,
        context: &TestExecutionContext,
        event_store: &dyn EventStore,
    ) -> Result<ExtendedModelVersion>;
}

/// Default implementation of the test executor
#[derive(Default)]
pub struct DefaultTestExecutor {
    /// Available provider clients
    provider_clients: HashMap<LlmProvider, Box<dyn ProviderClient + Send + Sync>>,
}

/// Trait for provider-specific clients
#[async_trait]
pub trait ProviderClient: Send + Sync {
    /// Send a request to the provider with a specific version
    async fn send_request(
        &self,
        version: &ExtendedModelVersion,
        payload: &serde_json::Value,
    ) -> Result<serde_json::Value>;
}

impl TestExecutionContext {
    /// Create a new test execution context
    pub fn new(config: VersionTestConfig, session_id: SessionId) -> Self {
        Self {
            config,
            session_id,
            version_capture: Box::new(DefaultVersionCapture),
        }
    }

    /// Create with a custom version capture implementation
    pub fn with_version_capture(
        config: VersionTestConfig,
        session_id: SessionId,
        version_capture: Box<dyn VersionCapture + Send + Sync>,
    ) -> Self {
        Self {
            config,
            session_id,
            version_capture,
        }
    }

    /// Check if we should use the original version
    pub fn should_use_original(&self) -> bool {
        self.config.use_original_version && self.config.target_version.is_none()
    }

    /// Get the target version if explicitly set
    pub fn target_version(&self) -> Option<&ExtendedModelVersion> {
        self.config.target_version.as_ref()
    }

    /// Check if comparison mode is enabled
    pub fn is_comparison_enabled(&self) -> bool {
        self.config.comparison_mode.is_some()
    }
}

impl DefaultTestExecutor {
    /// Create a new test executor
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a provider client
    pub fn register_provider(
        &mut self,
        provider: LlmProvider,
        client: Box<dyn ProviderClient + Send + Sync>,
    ) {
        self.provider_clients.insert(provider, client);
    }
}

#[async_trait]
impl TestExecutor for DefaultTestExecutor {
    async fn execute_test(
        &self,
        context: &TestExecutionContext,
        request_payload: &serde_json::Value,
    ) -> Result<TestExecutionResult> {
        // Get the version to use for this test
        let version = self.resolve_version(context, &DummyEventStore).await?;

        // Get the appropriate client for the provider
        let client = self
            .provider_clients
            .get(&version.provider)
            .ok_or_else(|| {
                crate::error::Error::application(format!(
                    "No client registered for provider: {:?}",
                    version.provider
                ))
            })?;

        // Execute the request
        match client.send_request(&version, request_payload).await {
            Ok(_response) => Ok(TestExecutionResult {
                session_id: context.session_id.clone(),
                version_used: version,
                passed: true,
                error_message: None,
                comparison_results: None,
            }),
            Err(e) => Ok(TestExecutionResult {
                session_id: context.session_id.clone(),
                version_used: version,
                passed: false,
                error_message: Some(e.to_string()),
                comparison_results: None,
            }),
        }
    }

    async fn resolve_version(
        &self,
        context: &TestExecutionContext,
        event_store: &dyn EventStore,
    ) -> Result<ExtendedModelVersion> {
        // If a target version is explicitly set, use it
        if let Some(target) = context.target_version() {
            return Ok(target.clone());
        }

        // If we should use the original version, get it from the event store
        if context.should_use_original() {
            let events = event_store.get_session_events(&context.session_id).await?;

            // Find the first version detection event
            for event in events {
                if let DomainEvent::VersionChanged {
                    to_version,
                    reason: VersionChangeReason::InitialDetection,
                    ..
                } = event
                {
                    return Ok(to_version);
                }
            }

            return Err(crate::error::Error::application(
                "No original version found for session",
            ));
        }

        // Default: use the latest available version
        // In a real implementation, this would query the provider for the latest version
        Err(crate::error::Error::application(
            "No version resolution strategy matched",
        ))
    }
}

// Dummy event store for testing
struct DummyEventStore;

#[async_trait]
impl EventStore for DummyEventStore {
    async fn store_event(&self, _event: DomainEvent) -> Result<()> {
        Ok(())
    }

    async fn get_session_events(&self, _session_id: &SessionId) -> Result<Vec<DomainEvent>> {
        Ok(vec![])
    }

    async fn get_version_changes(
        &self,
        _session_id: &SessionId,
    ) -> Result<Vec<crate::domain::VersionChangeEvent>> {
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{ApiVersion, ModelName, ModelVersionString, ProviderVersionInfo};
    use chrono::Utc;

    struct MockProviderClient {
        responses: HashMap<String, serde_json::Value>,
    }

    impl MockProviderClient {
        fn new() -> Self {
            Self {
                responses: HashMap::new(),
            }
        }

        fn with_response(mut self, version: &str, response: serde_json::Value) -> Self {
            self.responses.insert(version.to_string(), response);
            self
        }
    }

    #[async_trait]
    impl ProviderClient for MockProviderClient {
        async fn send_request(
            &self,
            version: &ExtendedModelVersion,
            _payload: &serde_json::Value,
        ) -> Result<serde_json::Value> {
            let version_str = version.display_string();
            self.responses.get(&version_str).cloned().ok_or_else(|| {
                crate::error::Error::application(format!(
                    "No mock response for version: {version_str}"
                ))
            })
        }
    }

    #[tokio::test]
    async fn test_execution_context_creation() {
        let config = VersionTestConfig::default();
        let session_id = SessionId::generate();
        let context = TestExecutionContext::new(config, session_id.clone());

        assert_eq!(context.session_id, session_id);
        assert!(context.should_use_original());
        assert!(context.target_version().is_none());
        assert!(!context.is_comparison_enabled());
    }

    #[tokio::test]
    async fn test_execution_context_with_target_version() {
        let target_version = ExtendedModelVersion::new(
            LlmProvider::OpenAI,
            ModelName::try_new("gpt-4").unwrap(),
            ProviderVersionInfo::OpenAI {
                model_version: Some(ModelVersionString::try_new("1106-preview").unwrap()),
                api_version: ApiVersion::try_new("2023-12-01").unwrap(),
                system_fingerprint: None,
            },
        );

        let config = VersionTestConfig {
            use_original_version: false,
            target_version: Some(target_version.clone()),
            comparison_mode: None,
        };

        let session_id = SessionId::generate();
        let context = TestExecutionContext::new(config, session_id);

        assert!(!context.should_use_original());
        assert_eq!(context.target_version(), Some(&target_version));
    }

    #[tokio::test]
    async fn test_executor_with_mock_client() {
        let mut executor = DefaultTestExecutor::new();

        let mock_client = MockProviderClient::new().with_response(
            "gpt-4/1106-preview (API: 2023-12-01)",
            serde_json::json!({ "response": "test" }),
        );

        executor.register_provider(LlmProvider::OpenAI, Box::new(mock_client));

        let target_version = ExtendedModelVersion::new(
            LlmProvider::OpenAI,
            ModelName::try_new("gpt-4").unwrap(),
            ProviderVersionInfo::OpenAI {
                model_version: Some(ModelVersionString::try_new("1106-preview").unwrap()),
                api_version: ApiVersion::try_new("2023-12-01").unwrap(),
                system_fingerprint: None,
            },
        );

        let config = VersionTestConfig {
            use_original_version: false,
            target_version: Some(target_version),
            comparison_mode: None,
        };

        let context = TestExecutionContext::new(config, SessionId::generate());
        let result = executor
            .execute_test(&context, &serde_json::json!({ "prompt": "test" }))
            .await
            .unwrap();

        assert!(result.passed);
        assert!(result.error_message.is_none());
    }

    #[tokio::test]
    async fn test_version_resolution_from_event_store() {
        use crate::application::event_store::test_support::InMemoryEventStore;

        let event_store = InMemoryEventStore::new();
        let session_id = SessionId::generate();

        // Store an initial version detection event
        let version = ExtendedModelVersion::new(
            LlmProvider::Anthropic,
            ModelName::try_new("claude-3").unwrap(),
            ProviderVersionInfo::Anthropic {
                model_version: ModelVersionString::try_new("20240229").unwrap(),
                api_version: ApiVersion::try_new("2023-06-01").unwrap(),
                capabilities_version: None,
            },
        );

        let event = DomainEvent::VersionChanged {
            session_id: session_id.clone(),
            from_version: None,
            to_version: version.clone(),
            reason: VersionChangeReason::InitialDetection,
            occurred_at: Utc::now(),
        };

        event_store.store_event(event).await.unwrap();

        // Test resolution
        let executor = DefaultTestExecutor::new();
        let config = VersionTestConfig::default();
        let context = TestExecutionContext::new(config, session_id);

        let resolved = executor
            .resolve_version(&context, &event_store)
            .await
            .unwrap();
        assert_eq!(resolved, version);
    }
}
