//! Response processor for extracting provider metadata from responses

use crate::providers::bedrock::{
    models::extract_token_usage,
    types::{ModelFamily, ModelId, ModelPricing},
};
use crate::providers::{ProviderId, ProviderMetadata};
use bytes::Bytes;
use serde_json::Value;

/// Process response body and extract provider metadata
pub struct ProviderResponseProcessor {
    provider_id: ProviderId,
    model_id: Option<ModelId>,
    base_metadata: ProviderMetadata,
}

impl ProviderResponseProcessor {
    /// Create a new response processor with base metadata
    pub fn new(base_metadata: ProviderMetadata) -> Self {
        Self {
            provider_id: base_metadata.provider_id.clone(),
            model_id: base_metadata.model_id.clone(),
            base_metadata,
        }
    }

    /// Process a response body chunk and extract metadata
    pub fn process_body_chunk(&self, chunk: &Bytes) -> Option<ProviderMetadata> {
        // Only process if we have a model ID
        let model_id = self.model_id.as_ref()?;

        // Try to parse JSON from the chunk
        let json_value: Value = serde_json::from_slice(chunk).ok()?;

        // Extract metadata based on provider
        if self.provider_id == ProviderId::bedrock() {
            self.process_bedrock_response(model_id, &json_value)
        } else {
            None
        }
    }

    /// Process complete response body and extract final metadata
    pub fn process_complete_body(&self, body: &[u8]) -> ProviderMetadata {
        let mut metadata = self.base_metadata.clone();

        // Only process if we have a model ID
        if let Some(model_id) = &self.model_id {
            // Try to parse JSON from the body
            if let Ok(json_value) = serde_json::from_slice::<Value>(body) {
                // Extract metadata based on provider
                if let Some(extracted) = if self.provider_id == ProviderId::bedrock() {
                    self.process_bedrock_response(model_id, &json_value)
                } else {
                    None
                } {
                    // Merge extracted metadata
                    metadata.request_tokens = extracted.request_tokens.or(metadata.request_tokens);
                    metadata.response_tokens =
                        extracted.response_tokens.or(metadata.response_tokens);
                    metadata.total_tokens = extracted.total_tokens.or(metadata.total_tokens);
                    metadata.cost_estimate = extracted.cost_estimate.or(metadata.cost_estimate);
                }
            }
        }

        metadata
    }

    /// Process Bedrock response and extract metadata
    fn process_bedrock_response(
        &self,
        model_id: &ModelId,
        json_value: &Value,
    ) -> Option<ProviderMetadata> {
        let mut metadata = self.base_metadata.clone();

        // Determine model family
        let model_family = ModelFamily::from_model_id(model_id);

        // Extract token usage based on model family
        if let Some(token_usage) = extract_token_usage(&model_family, json_value) {
            metadata.request_tokens = Some(token_usage.input_tokens);
            metadata.response_tokens = Some(token_usage.output_tokens);
            metadata.total_tokens = Some(token_usage.total_tokens);

            // Calculate cost if pricing is available
            if let Some(pricing) = ModelPricing::for_model(model_id.as_ref()) {
                metadata.cost_estimate = Some(
                    pricing.calculate_cost(token_usage.input_tokens, token_usage.output_tokens),
                );
            }
        }

        Some(metadata)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::bedrock::types::{InputTokens, OutputTokens};
    use serde_json::json;

    #[test]
    fn test_process_claude_response() {
        let base_metadata = ProviderMetadata {
            provider_id: ProviderId::bedrock(),
            model_id: Some(
                ModelId::try_new("anthropic.claude-3-sonnet-20240229".to_string()).unwrap(),
            ),
            ..Default::default()
        };

        let processor = ProviderResponseProcessor::new(base_metadata);

        let response_body = json!({
            "id": "msg_123",
            "type": "message",
            "role": "assistant",
            "content": [{"type": "text", "text": "Hello!"}],
            "usage": {
                "input_tokens": 10,
                "output_tokens": 5
            }
        });

        let body_bytes = response_body.to_string().into_bytes();
        let metadata = processor.process_complete_body(&body_bytes);

        assert_eq!(
            metadata.request_tokens,
            Some(InputTokens::try_new(10).unwrap())
        );
        assert_eq!(
            metadata.response_tokens,
            Some(OutputTokens::try_new(5).unwrap())
        );
        assert_eq!(metadata.total_tokens.unwrap().into_inner(), 15);
        assert!(metadata.cost_estimate.is_some());
    }

    #[test]
    fn test_process_titan_response() {
        let base_metadata = ProviderMetadata {
            provider_id: ProviderId::bedrock(),
            model_id: Some(ModelId::try_new("amazon.titan-text-express-v1".to_string()).unwrap()),
            ..Default::default()
        };

        let processor = ProviderResponseProcessor::new(base_metadata);

        let response_body = json!({
            "inputTextTokenCount": 12,
            "results": [{
                "tokenCount": 20,
                "outputText": "Hello from Titan!",
                "completionReason": "FINISH"
            }]
        });

        let body_bytes = response_body.to_string().into_bytes();
        let metadata = processor.process_complete_body(&body_bytes);

        assert_eq!(
            metadata.request_tokens,
            Some(InputTokens::try_new(12).unwrap())
        );
        assert_eq!(
            metadata.response_tokens,
            Some(OutputTokens::try_new(20).unwrap())
        );
        assert_eq!(metadata.total_tokens.unwrap().into_inner(), 32);
        assert!(metadata.cost_estimate.is_some());
    }

    #[test]
    fn test_process_invalid_json() {
        let base_metadata = ProviderMetadata {
            provider_id: ProviderId::bedrock(),
            model_id: Some(ModelId::try_new("test-model".to_string()).unwrap()),
            ..Default::default()
        };

        let processor = ProviderResponseProcessor::new(base_metadata.clone());

        let body_bytes = b"invalid json";
        let metadata = processor.process_complete_body(body_bytes);

        // Should return base metadata unchanged
        assert_eq!(metadata.provider_id, base_metadata.provider_id);
        assert_eq!(metadata.model_id, base_metadata.model_id);
        assert_eq!(metadata.request_tokens, None);
        assert_eq!(metadata.response_tokens, None);
    }
}
