//! Parser for extracting LLM request data from HTTP request bodies
//!
//! This module provides functionality to parse various LLM provider request formats
//! and extract the common elements (prompt, model, parameters) from them.

use serde_json::Value;
use std::collections::HashMap;

use crate::domain::{
    config_types::ProviderName,
    llm::{LlmProvider, ModelVersion},
    types::{LlmParameters, ModelId, Prompt},
};

/// Errors that can occur when parsing LLM requests
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("Invalid JSON: {0}")]
    InvalidJson(#[from] serde_json::Error),

    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("Invalid field value for {field}: {reason}")]
    InvalidFieldValue { field: String, reason: String },

    #[error("Unknown request format")]
    UnknownFormat,
}

/// Result type for parsing operations
type ParseResult<T> = Result<T, ParseError>;

/// Parsed LLM request data
#[derive(Debug, Clone)]
pub struct ParsedLlmRequest {
    pub model_version: ModelVersion,
    pub prompt: Prompt,
    pub parameters: LlmParameters,
}

/// Parse an LLM request from raw bytes
pub fn parse_llm_request(
    body: &[u8],
    uri: &str,
    headers: &[(String, String)],
) -> ParseResult<ParsedLlmRequest> {
    // Parse JSON body
    let json: Value = serde_json::from_slice(body)?;

    // Determine provider based on URI patterns
    if uri.contains("/v1/chat/completions") || uri.contains("/v1/completions") {
        parse_openai_format(&json)
    } else if uri.contains("/v1/messages") {
        parse_anthropic_format(&json, headers)
    } else if uri.contains("/bedrock") || uri.contains("/invoke") {
        parse_bedrock_format(&json, uri)
    } else {
        // Try to auto-detect based on content
        if json.get("model").is_some() && json.get("messages").is_some() {
            parse_openai_format(&json)
        } else if json.get("model").is_some() && json.get("prompt").is_some() {
            parse_anthropic_format(&json, headers)
        } else {
            Err(ParseError::UnknownFormat)
        }
    }
}

/// Parse OpenAI-compatible format
fn parse_openai_format(json: &Value) -> ParseResult<ParsedLlmRequest> {
    // Extract model
    let model_str = json
        .get("model")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ParseError::MissingField("model".to_string()))?;

    let model_id =
        ModelId::try_new(model_str.to_string()).map_err(|e| ParseError::InvalidFieldValue {
            field: "model".to_string(),
            reason: e.to_string(),
        })?;

    // Determine provider based on model name patterns
    let provider = if model_str.starts_with("gpt-") || model_str.starts_with("o1-") {
        LlmProvider::OpenAI
    } else {
        LlmProvider::Other(
            ProviderName::try_new("openai-compatible".to_string()).unwrap(), // Safe because we know this is valid
        )
    };

    let model_version = ModelVersion { provider, model_id };

    // Extract prompt from messages
    let prompt_text = if let Some(messages) = json.get("messages").and_then(|v| v.as_array()) {
        // Concatenate all messages into a prompt
        messages
            .iter()
            .filter_map(|msg| {
                let role = msg.get("role")?.as_str()?;
                let content = msg.get("content")?.as_str()?;
                Some(format!("{role}: {content}"))
            })
            .collect::<Vec<_>>()
            .join("\n")
    } else if let Some(prompt) = json.get("prompt").and_then(|v| v.as_str()) {
        prompt.to_string()
    } else {
        return Err(ParseError::MissingField("messages or prompt".to_string()));
    };

    let prompt = Prompt::try_new(prompt_text).map_err(|e| ParseError::InvalidFieldValue {
        field: "prompt".to_string(),
        reason: e.to_string(),
    })?;

    // Extract parameters (everything except model and messages/prompt)
    let mut params = json
        .as_object()
        .ok_or_else(|| ParseError::InvalidFieldValue {
            field: "root".to_string(),
            reason: "Expected JSON object".to_string(),
        })?
        .clone();
    params.remove("model");
    params.remove("messages");
    params.remove("prompt");

    let parameters = LlmParameters::new(Value::Object(params));

    Ok(ParsedLlmRequest {
        model_version,
        prompt,
        parameters,
    })
}

/// Parse Anthropic format
fn parse_anthropic_format(
    json: &Value,
    headers: &[(String, String)],
) -> ParseResult<ParsedLlmRequest> {
    // Get model from JSON or header
    let model_str = json
        .get("model")
        .and_then(|v| v.as_str())
        .or_else(|| {
            headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("anthropic-version"))
                .map(|(_, v)| v.as_str())
        })
        .ok_or_else(|| ParseError::MissingField("model".to_string()))?;

    let model_id =
        ModelId::try_new(model_str.to_string()).map_err(|e| ParseError::InvalidFieldValue {
            field: "model".to_string(),
            reason: e.to_string(),
        })?;

    let model_version = ModelVersion {
        provider: LlmProvider::Anthropic,
        model_id,
    };

    // Extract prompt
    let prompt_text = if let Some(messages) = json.get("messages").and_then(|v| v.as_array()) {
        // Handle messages format
        messages
            .iter()
            .filter_map(|msg| {
                let role = msg.get("role")?.as_str()?;
                let content = msg.get("content")?.as_str()?;
                Some(format!("{role}: {content}"))
            })
            .collect::<Vec<_>>()
            .join("\n")
    } else if let Some(prompt) = json.get("prompt").and_then(|v| v.as_str()) {
        prompt.to_string()
    } else {
        return Err(ParseError::MissingField("messages or prompt".to_string()));
    };

    let prompt = Prompt::try_new(prompt_text).map_err(|e| ParseError::InvalidFieldValue {
        field: "prompt".to_string(),
        reason: e.to_string(),
    })?;

    // Extract parameters
    let mut params = json
        .as_object()
        .ok_or_else(|| ParseError::InvalidFieldValue {
            field: "root".to_string(),
            reason: "Expected JSON object".to_string(),
        })?
        .clone();
    params.remove("model");
    params.remove("messages");
    params.remove("prompt");

    let parameters = LlmParameters::new(Value::Object(params));

    Ok(ParsedLlmRequest {
        model_version,
        prompt,
        parameters,
    })
}

/// Parse Bedrock format
fn parse_bedrock_format(json: &Value, uri: &str) -> ParseResult<ParsedLlmRequest> {
    // Extract model from URI (e.g., /model/anthropic.claude-v2/invoke)
    let model_str = uri
        .split('/')
        .find(|segment| segment.contains('.'))
        .ok_or_else(|| ParseError::MissingField("model in URI".to_string()))?;

    let model_id =
        ModelId::try_new(model_str.to_string()).map_err(|e| ParseError::InvalidFieldValue {
            field: "model".to_string(),
            reason: e.to_string(),
        })?;

    let model_version = ModelVersion {
        provider: LlmProvider::Other(ProviderName::try_new("bedrock".to_string()).unwrap()),
        model_id,
    };

    // Extract prompt - Bedrock format varies by model
    let prompt_text = if let Some(prompt) = json.get("prompt").and_then(|v| v.as_str()) {
        prompt.to_string()
    } else if let Some(input_text) = json.get("inputText").and_then(|v| v.as_str()) {
        input_text.to_string()
    } else if let Some(messages) = json.get("messages").and_then(|v| v.as_array()) {
        messages
            .iter()
            .filter_map(|msg| {
                let content = msg.get("content")?.as_str()?;
                Some(content.to_string())
            })
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        return Err(ParseError::MissingField(
            "prompt, inputText, or messages".to_string(),
        ));
    };

    let prompt = Prompt::try_new(prompt_text).map_err(|e| ParseError::InvalidFieldValue {
        field: "prompt".to_string(),
        reason: e.to_string(),
    })?;

    // All fields are parameters for Bedrock
    let parameters = LlmParameters::new(json.clone());

    Ok(ParsedLlmRequest {
        model_version,
        prompt,
        parameters,
    })
}

/// Create a fallback parsed request when parsing fails
pub fn create_fallback_request(error: &ParseError) -> ParsedLlmRequest {
    ParsedLlmRequest {
        model_version: ModelVersion {
            provider: LlmProvider::Other(ProviderName::try_new("unknown".to_string()).unwrap()),
            model_id: ModelId::try_new("unknown-model".to_string()).unwrap(),
        },
        prompt: Prompt::try_new(format!("Failed to parse request: {error}")).unwrap(),
        parameters: LlmParameters::new(Value::Object(HashMap::new().into_iter().collect())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_openai_chat_format() {
        let body = json!({
            "model": "gpt-4",
            "messages": [
                {"role": "system", "content": "You are a helpful assistant."},
                {"role": "user", "content": "Hello!"}
            ],
            "temperature": 0.7,
            "max_tokens": 100
        });

        let result =
            parse_llm_request(body.to_string().as_bytes(), "/v1/chat/completions", &[]).unwrap();

        assert_eq!(result.model_version.model_id.as_ref(), "gpt-4");
        assert!(matches!(result.model_version.provider, LlmProvider::OpenAI));
        assert!(result
            .prompt
            .as_ref()
            .contains("system: You are a helpful assistant"));
        assert!(result.prompt.as_ref().contains("user: Hello!"));
    }

    #[test]
    fn test_parse_anthropic_format() {
        let body = json!({
            "model": "claude-3-opus-20240229",
            "messages": [
                {"role": "user", "content": "What is 2+2?"}
            ],
            "max_tokens": 1024
        });

        let result = parse_llm_request(body.to_string().as_bytes(), "/v1/messages", &[]).unwrap();

        assert_eq!(
            result.model_version.model_id.as_ref(),
            "claude-3-opus-20240229"
        );
        assert!(matches!(
            result.model_version.provider,
            LlmProvider::Anthropic
        ));
        assert!(result.prompt.as_ref().contains("user: What is 2+2?"));
    }

    #[test]
    fn test_parse_bedrock_format() {
        let body = json!({
            "prompt": "Hello, Claude!",
            "max_tokens_to_sample": 100,
            "temperature": 0.5
        });

        let result = parse_llm_request(
            body.to_string().as_bytes(),
            "/model/anthropic.claude-v2/invoke",
            &[],
        )
        .unwrap();

        assert_eq!(
            result.model_version.model_id.as_ref(),
            "anthropic.claude-v2"
        );
        assert!(matches!(
            result.model_version.provider,
            LlmProvider::Other(_)
        ));
        assert_eq!(result.prompt.as_ref(), "Hello, Claude!");
    }

    #[test]
    fn test_fallback_on_parse_error() {
        let error = ParseError::MissingField("model".to_string());
        let fallback = create_fallback_request(&error);

        assert_eq!(fallback.model_version.model_id.as_ref(), "unknown-model");
        assert!(fallback.prompt.as_ref().contains("Failed to parse request"));
    }
}
