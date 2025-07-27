use crate::domain::types::{
    Cost, FinishReason, Latency, LlmParameters, ModelId, Prompt, ResponseText, TokenCount,
};
use chrono::{DateTime, Utc};
use nutype::nutype;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for an LLM request
#[nutype(derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize))]
pub struct RequestId(Uuid);

impl RequestId {
    pub fn generate() -> Self {
        Self::new(Uuid::now_v7())
    }
}

impl Default for RequestId {
    fn default() -> Self {
        Self::generate()
    }
}

/// LLM provider identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LlmProvider {
    OpenAI,
    Anthropic,
    Google,
    Azure,
    Other(String),
}

impl LlmProvider {
    /// Get the string representation of the provider
    pub fn as_str(&self) -> &str {
        match self {
            LlmProvider::OpenAI => "openai",
            LlmProvider::Anthropic => "anthropic",
            LlmProvider::Google => "google",
            LlmProvider::Azure => "azure",
            LlmProvider::Other(name) => name,
        }
    }
}

/// Model version information - treats model IDs as opaque strings
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModelVersion {
    pub provider: LlmProvider,
    pub model_id: ModelId, // Opaque identifier from the provider
}

/// LLM request represents a single request to an LLM provider
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LlmRequest {
    pub id: RequestId,
    pub session_id: crate::domain::SessionId,
    pub model_version: ModelVersion,
    pub prompt: Prompt,
    pub parameters: LlmParameters,
    pub created_at: DateTime<Utc>,
    pub status: RequestStatus,
}

/// LLM response represents the response from an LLM provider
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LlmResponse {
    pub request_id: RequestId,
    pub response_text: ResponseText,
    pub metadata: ResponseMetadata,
    pub created_at: DateTime<Utc>,
}

/// Status of an LLM request
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RequestStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

/// Metadata associated with an LLM response
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ResponseMetadata {
    pub tokens_used: Option<TokenCount>,
    pub cost_cents: Option<Cost>,
    pub latency_ms: Option<Latency>,
    pub finish_reason: Option<FinishReason>,
    pub model_used: Option<ModelId>,
}

impl LlmRequest {
    pub fn new(
        session_id: crate::domain::SessionId,
        model_version: ModelVersion,
        prompt: Prompt,
        parameters: LlmParameters,
    ) -> Self {
        Self {
            id: RequestId::generate(),
            session_id,
            model_version,
            prompt,
            parameters,
            created_at: Utc::now(),
            status: RequestStatus::Pending,
        }
    }

    pub fn start(&mut self) {
        self.status = RequestStatus::InProgress;
    }

    pub fn complete(&mut self) {
        self.status = RequestStatus::Completed;
    }

    pub fn fail(&mut self) {
        self.status = RequestStatus::Failed;
    }

    pub fn cancel(&mut self) {
        self.status = RequestStatus::Cancelled;
    }
}

impl LlmResponse {
    pub fn new(
        request_id: RequestId,
        response_text: ResponseText,
        metadata: ResponseMetadata,
    ) -> Self {
        Self {
            request_id,
            response_text,
            metadata,
            created_at: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::SessionId;
    use proptest::prelude::*;

    #[test]
    fn test_request_id_generation() {
        let id1 = RequestId::generate();
        let id2 = RequestId::generate();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_llm_request_creation() {
        let session_id = SessionId::generate();
        let model_version = ModelVersion {
            provider: LlmProvider::OpenAI,
            model_id: ModelId::try_new("gpt-4-turbo-2024-01".to_string()).unwrap(),
        };

        let request = LlmRequest::new(
            session_id,
            model_version,
            Prompt::try_new("Test prompt".to_string()).unwrap(),
            LlmParameters::new(serde_json::json!({"temperature": 0.7})),
        );

        assert_eq!(request.status, RequestStatus::Pending);
        assert_eq!(request.prompt.as_ref(), "Test prompt");
    }

    #[test]
    fn test_request_status_transitions() {
        let session_id = SessionId::generate();
        let model_version = ModelVersion {
            provider: LlmProvider::Anthropic,
            model_id: ModelId::try_new("claude-3-opus-20240229".to_string()).unwrap(),
        };

        let mut request = LlmRequest::new(
            session_id,
            model_version,
            Prompt::try_new("Test prompt".to_string()).unwrap(),
            LlmParameters::new(serde_json::json!({})),
        );

        assert_eq!(request.status, RequestStatus::Pending);

        request.start();
        assert_eq!(request.status, RequestStatus::InProgress);

        request.complete();
        assert_eq!(request.status, RequestStatus::Completed);
    }

    #[test]
    fn test_llm_response_creation() {
        let request_id = RequestId::generate();
        let metadata = ResponseMetadata {
            tokens_used: Some(unsafe { TokenCount::new_unchecked(150) }),
            cost_cents: Some(unsafe { Cost::new_unchecked(5) }),
            latency_ms: Some(unsafe { Latency::new_unchecked(1200) }),
            finish_reason: Some(FinishReason::try_new("stop".to_string()).unwrap()),
            model_used: Some(ModelId::try_new("gpt-4".to_string()).unwrap()),
        };

        let response = LlmResponse::new(
            request_id,
            ResponseText::try_new("Test response".to_string()).unwrap(),
            metadata,
        );

        assert_eq!(response.response_text.as_ref(), "Test response");
        assert_eq!(
            response.metadata.tokens_used,
            Some(unsafe { TokenCount::new_unchecked(150) })
        );
        assert_eq!(
            response.metadata.cost_cents,
            Some(unsafe { Cost::new_unchecked(5) })
        );
    }

    // Property-based tests
    proptest! {
        #[test]
        fn prop_request_id_uniqueness(n in 1..100usize) {
            let mut ids = std::collections::HashSet::new();
            for _ in 0..n {
                let id = RequestId::generate();
                assert!(ids.insert(id));
            }
        }

        #[test]
        fn prop_model_version_serialization(
            provider_choice in 0..5u8,
            model_id in "[a-zA-Z0-9-]+",
            custom_name in "[a-zA-Z0-9-]+"
        ) {
            let provider = match provider_choice {
                0 => LlmProvider::OpenAI,
                1 => LlmProvider::Anthropic,
                2 => LlmProvider::Google,
                3 => LlmProvider::Azure,
                _ => LlmProvider::Other(custom_name),
            };

            let model_version = ModelVersion {
                provider: provider.clone(),
                model_id: ModelId::try_new(model_id.clone()).unwrap(),
            };

            let json = serde_json::to_string(&model_version).unwrap();
            let deserialized: ModelVersion = serde_json::from_str(&json).unwrap();

            assert_eq!(model_version, deserialized);
        }

        #[test]
        fn prop_llm_request_serialization(
            prompt in any::<String>(),
            model_id in "[a-zA-Z0-9-]+",
            temp in 0.0..2.0f64,
            max_tokens in 1..4000u32
        ) {
            let session_id = SessionId::generate();
            let model_version = ModelVersion {
                provider: LlmProvider::OpenAI,
                model_id: ModelId::try_new(model_id).unwrap(),
            };
            // Round temperature to avoid floating point precision issues
            let rounded_temp = (temp * 1000.0).round() / 1000.0;
            let parameters = LlmParameters::new(serde_json::json!({
                "temperature": rounded_temp,
                "max_tokens": max_tokens
            }));

            let request = if prompt.is_empty() {
                return Ok(()); // Skip empty prompts as they're invalid
            } else {
                LlmRequest::new(
                    session_id,
                    model_version,
                    Prompt::try_new(prompt).unwrap(),
                    parameters
                )
            };

            let json = serde_json::to_string(&request).unwrap();
            let deserialized: LlmRequest = serde_json::from_str(&json).unwrap();

            assert_eq!(request, deserialized);
        }

        #[test]
        fn prop_response_metadata_defaults(
            tokens in prop::option::of(0..10000u32),
            cost in prop::option::of(0..100000u32),
            latency in prop::option::of(0..60000u64),
            finish_reason in prop::option::of("[a-zA-Z_]+"),
            model_used in prop::option::of("[a-zA-Z0-9-]+")
        ) {
            let metadata = ResponseMetadata {
                tokens_used: tokens.map(|t| unsafe { TokenCount::new_unchecked(t) }),
                cost_cents: cost.map(|c| unsafe { Cost::new_unchecked(c) }),
                latency_ms: latency.map(|l| unsafe { Latency::new_unchecked(l) }),
                finish_reason: finish_reason.and_then(|s| FinishReason::try_new(s).ok()),
                model_used: model_used.and_then(|s| ModelId::try_new(s).ok()),
            };

            let json = serde_json::to_string(&metadata).unwrap();
            let deserialized: ResponseMetadata = serde_json::from_str(&json).unwrap();

            assert_eq!(metadata, deserialized);
        }
    }
}
