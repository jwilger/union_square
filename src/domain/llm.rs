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

/// Model version information - treats model IDs as opaque strings
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelVersion {
    pub provider: LlmProvider,
    pub model_id: String, // Opaque identifier from the provider
}

/// LLM request represents a single request to an LLM provider
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LlmRequest {
    pub id: RequestId,
    pub session_id: crate::domain::SessionId,
    pub model_version: ModelVersion,
    pub prompt: String,
    pub parameters: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub status: RequestStatus,
}

/// LLM response represents the response from an LLM provider
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LlmResponse {
    pub request_id: RequestId,
    pub response_text: String,
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ResponseMetadata {
    pub tokens_used: Option<u32>,
    pub cost_cents: Option<u32>,
    pub latency_ms: Option<u64>,
    pub finish_reason: Option<String>,
    pub model_used: Option<String>,
}

impl LlmRequest {
    pub fn new(
        session_id: crate::domain::SessionId,
        model_version: ModelVersion,
        prompt: String,
        parameters: serde_json::Value,
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
    pub fn new(request_id: RequestId, response_text: String, metadata: ResponseMetadata) -> Self {
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
            model_id: "gpt-4-turbo-2024-01".to_string(),
        };

        let request = LlmRequest::new(
            session_id,
            model_version,
            "Test prompt".to_string(),
            serde_json::json!({"temperature": 0.7}),
        );

        assert_eq!(request.status, RequestStatus::Pending);
        assert_eq!(request.prompt, "Test prompt");
    }

    #[test]
    fn test_request_status_transitions() {
        let session_id = SessionId::generate();
        let model_version = ModelVersion {
            provider: LlmProvider::Anthropic,
            model_id: "claude-3-opus-20240229".to_string(),
        };

        let mut request = LlmRequest::new(
            session_id,
            model_version,
            "Test prompt".to_string(),
            serde_json::json!({}),
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
            tokens_used: Some(150),
            cost_cents: Some(5),
            latency_ms: Some(1200),
            finish_reason: Some("stop".to_string()),
            model_used: Some("gpt-4".to_string()),
        };

        let response = LlmResponse::new(request_id, "Test response".to_string(), metadata);

        assert_eq!(response.response_text, "Test response");
        assert_eq!(response.metadata.tokens_used, Some(150));
        assert_eq!(response.metadata.cost_cents, Some(5));
    }
}
