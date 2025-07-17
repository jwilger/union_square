//! Evaluation configuration and execution with version awareness
//!
//! This module provides functionality for configuring and executing evaluations
//! with specific model versions, enabling version-aware evaluation scenarios.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::{
    application::event_store::UnionSquareEventStore,
    domain::{
        DomainEvent, ExtendedModelVersion, LlmProvider, SessionId, VersionChangeReason,
        VersionEvaluationConfig,
    },
    error::Result,
};

/// Evaluation execution context that manages version selection
pub struct EvaluationExecutionContext {
    /// The configuration for this evaluation run
    pub config: VersionEvaluationConfig,

    /// The session being evaluated
    pub session_id: SessionId,
}

/// Result of an evaluation execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationExecutionResult {
    /// The session that was evaluated
    pub session_id: SessionId,

    /// Version used for the evaluation
    pub version_used: ExtendedModelVersion,

    /// Whether the evaluation passed
    pub passed: bool,

    /// Any error message if the evaluation failed
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

/// Service for executing version-aware evaluations
#[async_trait]
pub trait EvaluationExecutor: Send + Sync {
    /// Execute an evaluation with the given context
    async fn execute_evaluation(
        &self,
        context: &EvaluationExecutionContext,
        request_payload: &serde_json::Value,
    ) -> Result<EvaluationExecutionResult>;

    /// Get the appropriate model version for the evaluation
    async fn resolve_version(
        &self,
        context: &EvaluationExecutionContext,
        event_store: &dyn UnionSquareEventStore,
    ) -> Result<ExtendedModelVersion>;
}

/// Default implementation of the evaluation executor
pub struct DefaultEvaluationExecutor {
    /// Available provider clients
    provider_clients: HashMap<LlmProvider, Box<dyn ProviderClient + Send + Sync>>,
    /// Event store for version resolution
    event_store: Box<dyn UnionSquareEventStore + Send + Sync>,
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

impl EvaluationExecutionContext {
    /// Create a new evaluation execution context
    pub fn new(config: VersionEvaluationConfig, session_id: SessionId) -> Self {
        Self { config, session_id }
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

impl DefaultEvaluationExecutor {
    /// Create a new evaluation executor with an event store
    pub fn new(event_store: Box<dyn UnionSquareEventStore + Send + Sync>) -> Self {
        Self {
            provider_clients: HashMap::new(),
            event_store,
        }
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
impl EvaluationExecutor for DefaultEvaluationExecutor {
    async fn execute_evaluation(
        &self,
        context: &EvaluationExecutionContext,
        request_payload: &serde_json::Value,
    ) -> Result<EvaluationExecutionResult> {
        // Get the version to use for this evaluation
        let version = self
            .resolve_version(context, self.event_store.as_ref())
            .await?;

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
            Ok(_response) => Ok(EvaluationExecutionResult {
                session_id: context.session_id.clone(),
                version_used: version,
                passed: true,
                error_message: None,
                comparison_results: None,
            }),
            Err(e) => Ok(EvaluationExecutionResult {
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
        context: &EvaluationExecutionContext,
        event_store: &dyn UnionSquareEventStore,
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
    async fn test_evaluation_context_creation() {
        let config = VersionEvaluationConfig::default();
        let session_id = SessionId::generate();
        let context = EvaluationExecutionContext::new(config, session_id.clone());

        assert_eq!(context.session_id, session_id);
        assert!(context.should_use_original());
        assert!(context.target_version().is_none());
        assert!(!context.is_comparison_enabled());
    }

    #[tokio::test]
    async fn test_evaluation_context_with_target_version() {
        let target_version = ExtendedModelVersion::new(
            LlmProvider::OpenAI,
            ModelName::try_new("gpt-4").unwrap(),
            ProviderVersionInfo::OpenAI {
                model_version: Some(ModelVersionString::try_new("1106-preview").unwrap()),
                api_version: ApiVersion::try_new("2023-12-01").unwrap(),
                system_fingerprint: None,
            },
        );

        let config = VersionEvaluationConfig {
            use_original_version: false,
            target_version: Some(target_version.clone()),
            comparison_mode: None,
        };

        let session_id = SessionId::generate();
        let context = EvaluationExecutionContext::new(config, session_id);

        assert!(!context.should_use_original());
        assert_eq!(context.target_version(), Some(&target_version));
    }

    #[tokio::test]
    async fn test_evaluation_executor_with_mock_client() {
        use crate::application::event_store::test_support::InMemoryEventStore;

        let event_store = Box::new(InMemoryEventStore::new());
        let mut executor = DefaultEvaluationExecutor::new(event_store);

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

        let config = VersionEvaluationConfig {
            use_original_version: false,
            target_version: Some(target_version),
            comparison_mode: None,
        };

        let context = EvaluationExecutionContext::new(config, SessionId::generate());
        let result = executor
            .execute_evaluation(&context, &serde_json::json!({ "prompt": "test" }))
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

        // Test resolution - create a new executor with a new event store instance
        // since we can't clone InMemoryEventStore
        let test_event_store = InMemoryEventStore::new();

        // Re-add the event to the new store
        let event = DomainEvent::VersionChanged {
            session_id: session_id.clone(),
            from_version: None,
            to_version: version.clone(),
            reason: VersionChangeReason::InitialDetection,
            occurred_at: Utc::now(),
        };
        test_event_store.store_event(event).await.unwrap();

        let executor = DefaultEvaluationExecutor::new(Box::new(test_event_store));
        let config = VersionEvaluationConfig::default();
        let context = EvaluationExecutionContext::new(config, session_id);

        let resolved = executor
            .resolve_version(&context, executor.event_store.as_ref())
            .await
            .unwrap();
        assert_eq!(resolved, version);
    }
}
