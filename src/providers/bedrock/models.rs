//! Model-specific request/response handling for Bedrock
//!
//! This module handles the different request/response formats for various
//! model families supported by Bedrock.

use crate::providers::bedrock::types::{
    InputTokens, ModelFamily, ModelId, OutputTokens, TokenUsage,
};
use crate::providers::ProviderError;
use serde_json::Value;

/// Extract model ID from the request path
pub fn extract_model_id(path: &str) -> Option<ModelId> {
    // Path format: /bedrock/model/{model-id}/invoke
    let parts: Vec<&str> = path.split('/').collect();

    // Find "model" in path and get the next element
    parts
        .iter()
        .position(|&p| p == "model")
        .and_then(|i| parts.get(i + 1))
        .and_then(|&s| ModelId::try_new(s.to_string()).ok())
}

/// Extract token usage from response based on model family
pub fn extract_token_usage(
    model_family: &ModelFamily,
    response_body: &Value,
) -> Option<TokenUsage> {
    match model_family {
        ModelFamily::Claude => extract_claude_tokens(response_body),
        ModelFamily::Titan => extract_titan_tokens(response_body),
        ModelFamily::Llama => extract_llama_tokens(response_body),
        ModelFamily::Jurassic => extract_jurassic_tokens(response_body),
        ModelFamily::Command => extract_command_tokens(response_body),
        _ => None,
    }
}

/// Extract token usage from Claude response
fn extract_claude_tokens(response: &Value) -> Option<TokenUsage> {
    let usage = response.get("usage")?;
    let input_count = usage.get("input_tokens")?.as_u64()? as u32;
    let output_count = usage.get("output_tokens")?.as_u64()? as u32;

    let input_tokens = InputTokens::try_new(input_count).ok()?;
    let output_tokens = OutputTokens::try_new(output_count).ok()?;

    Some(TokenUsage::new(input_tokens, output_tokens))
}

/// Extract token usage from Titan response
fn extract_titan_tokens(response: &Value) -> Option<TokenUsage> {
    let input_count = response.get("inputTextTokenCount")?.as_u64()? as u32;
    let results = response.get("results")?.as_array()?;

    let output_count = results
        .iter()
        .filter_map(|r| r.get("tokenCount")?.as_u64())
        .sum::<u64>() as u32;

    let input_tokens = InputTokens::try_new(input_count).ok()?;
    let output_tokens = OutputTokens::try_new(output_count).ok()?;

    Some(TokenUsage::new(input_tokens, output_tokens))
}

/// Extract token usage from Llama response
fn extract_llama_tokens(response: &Value) -> Option<TokenUsage> {
    // Llama models return token counts in generation_token_count and prompt_token_count
    let output_count = response.get("generation_token_count")?.as_u64()? as u32;
    let input_count = response.get("prompt_token_count")?.as_u64()? as u32;

    let input_tokens = InputTokens::try_new(input_count).ok()?;
    let output_tokens = OutputTokens::try_new(output_count).ok()?;

    Some(TokenUsage::new(input_tokens, output_tokens))
}

/// Extract token usage from Jurassic response
fn extract_jurassic_tokens(response: &Value) -> Option<TokenUsage> {
    let completions = response.get("completions")?.as_array()?;
    if let Some(first) = completions.first() {
        let data = first.get("data")?;
        let input_count = data.get("tokens")?.as_array()?.len() as u32;
        let output_count = data.get("generated_tokens")?.as_u64()? as u32;

        let input_tokens = InputTokens::try_new(input_count).ok()?;
        let output_tokens = OutputTokens::try_new(output_count).ok()?;

        Some(TokenUsage::new(input_tokens, output_tokens))
    } else {
        None
    }
}

/// Extract token usage from Command response
fn extract_command_tokens(response: &Value) -> Option<TokenUsage> {
    // Cohere Command models include token counts in the response
    let input_count = response.get("prompt_tokens")?.as_u64()? as u32;
    let output_count = response.get("completion_tokens")?.as_u64()? as u32;

    let input_tokens = InputTokens::try_new(input_count).ok()?;
    let output_tokens = OutputTokens::try_new(output_count).ok()?;

    Some(TokenUsage::new(input_tokens, output_tokens))
}

/// Transform request body if needed for specific model families
pub fn transform_request_body(
    _model_family: &ModelFamily,
    body: Value,
) -> Result<Value, ProviderError> {
    // For MVP, we pass through requests as-is
    // Future enhancement: Add model-specific transformations if needed
    Ok(body)
}

/// Transform response body if needed for specific model families
pub fn transform_response_body(
    _model_family: &ModelFamily,
    body: Value,
) -> Result<Value, ProviderError> {
    // For MVP, we pass through responses as-is
    // Future enhancement: Add model-specific transformations if needed
    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_extract_model_id() {
        let path = "/bedrock/model/anthropic.claude-3-sonnet-20240229/invoke";
        let model_id = extract_model_id(path);
        assert_eq!(
            model_id,
            Some(ModelId::try_new("anthropic.claude-3-sonnet-20240229".to_string()).unwrap())
        );

        let path = "/bedrock/model/amazon.titan-text-express-v1/invoke-with-response-stream";
        let model_id = extract_model_id(path);
        assert_eq!(
            model_id,
            Some(ModelId::try_new("amazon.titan-text-express-v1".to_string()).unwrap())
        );

        let path = "/bedrock/invoke";
        let model_id = extract_model_id(path);
        assert_eq!(model_id, None);
    }

    #[test]
    fn test_extract_claude_tokens() {
        let response = json!({
            "content": [{
                "type": "text",
                "text": "Hello!"
            }],
            "usage": {
                "input_tokens": 10,
                "output_tokens": 5
            }
        });

        let tokens = extract_token_usage(&ModelFamily::Claude, &response);
        assert!(tokens.is_some());

        let tokens = tokens.unwrap();
        assert_eq!(tokens.input_tokens, InputTokens::try_new(10).unwrap());
        assert_eq!(tokens.output_tokens, OutputTokens::try_new(5).unwrap());
        assert_eq!(tokens.total_tokens.into_inner(), 15);
    }

    #[test]
    fn test_extract_titan_tokens() {
        let response = json!({
            "inputTextTokenCount": 8,
            "results": [{
                "tokenCount": 12,
                "outputText": "Hello world!"
            }]
        });

        let tokens = extract_token_usage(&ModelFamily::Titan, &response);
        assert!(tokens.is_some());

        let tokens = tokens.unwrap();
        assert_eq!(tokens.input_tokens, InputTokens::try_new(8).unwrap());
        assert_eq!(tokens.output_tokens, OutputTokens::try_new(12).unwrap());
        assert_eq!(tokens.total_tokens.into_inner(), 20);
    }

    #[test]
    fn test_extract_llama_tokens() {
        let response = json!({
            "generation": "Hello!",
            "prompt_token_count": 15,
            "generation_token_count": 7
        });

        let tokens = extract_token_usage(&ModelFamily::Llama, &response);
        assert!(tokens.is_some());

        let tokens = tokens.unwrap();
        assert_eq!(tokens.input_tokens, InputTokens::try_new(15).unwrap());
        assert_eq!(tokens.output_tokens, OutputTokens::try_new(7).unwrap());
        assert_eq!(tokens.total_tokens.into_inner(), 22);
    }
}
