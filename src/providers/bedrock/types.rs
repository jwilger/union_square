//! Type definitions for AWS Bedrock provider

use nutype::nutype;
use serde::{Deserialize, Serialize};

/// AWS region for Bedrock
#[nutype(
    sanitize(trim, lowercase),
    validate(not_empty, regex = r"^[a-z]{2}-[a-z]+-\d{1}$"),
    derive(Debug, Clone, PartialEq, Serialize, Deserialize, AsRef)
)]
pub struct AwsRegion(String);

/// Model ID as used by Bedrock
#[nutype(
    sanitize(trim),
    validate(not_empty),
    derive(Debug, Clone, PartialEq, Serialize, Deserialize, AsRef)
)]
pub struct ModelId(String);

/// Bedrock endpoint types
#[derive(Debug, Clone, PartialEq)]
pub enum BedrockEndpoint {
    InvokeModel,
    InvokeModelWithResponseStream,
}

impl BedrockEndpoint {
    /// Parse endpoint from path
    pub fn from_path(path: &str) -> Option<Self> {
        if path.ends_with("/invoke") {
            Some(Self::InvokeModel)
        } else if path.ends_with("/invoke-with-response-stream") {
            Some(Self::InvokeModelWithResponseStream)
        } else {
            None
        }
    }

    /// Get the endpoint suffix
    pub fn suffix(&self) -> &'static str {
        match self {
            Self::InvokeModel => "invoke",
            Self::InvokeModelWithResponseStream => "invoke-with-response-stream",
        }
    }
}

/// Supported Bedrock model families
#[derive(Debug, Clone, PartialEq)]
pub enum ModelFamily {
    Claude,   // Anthropic Claude models
    Titan,    // Amazon Titan models
    Llama,    // Meta Llama models
    Jurassic, // AI21 Jurassic models
    Command,  // Cohere Command models
    Stable,   // Stability AI models
    Unknown,
}

impl ModelFamily {
    /// Determine model family from model ID
    pub fn from_model_id(model_id: &ModelId) -> Self {
        let id_str = model_id.as_ref();
        if id_str.contains("claude") {
            Self::Claude
        } else if id_str.contains("titan") {
            Self::Titan
        } else if id_str.contains("llama") {
            Self::Llama
        } else if id_str.contains("j2") || id_str.contains("jurassic") {
            Self::Jurassic
        } else if id_str.contains("command") {
            Self::Command
        } else if id_str.contains("stable") {
            Self::Stable
        } else {
            Self::Unknown
        }
    }

    /// Determine model family from model ID string (for backwards compatibility)
    pub fn from_model_id_str(model_id: &str) -> Self {
        if model_id.contains("claude") {
            Self::Claude
        } else if model_id.contains("titan") {
            Self::Titan
        } else if model_id.contains("llama") {
            Self::Llama
        } else if model_id.contains("j2") || model_id.contains("jurassic") {
            Self::Jurassic
        } else if model_id.contains("command") {
            Self::Command
        } else if model_id.contains("stable") {
            Self::Stable
        } else {
            Self::Unknown
        }
    }
}

/// Token count for input tokens
#[nutype(
    validate(greater = 0),
    derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)
)]
pub struct InputTokens(u32);

/// Token count for output tokens
#[nutype(
    validate(greater = 0),
    derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)
)]
pub struct OutputTokens(u32);

/// Token count for total tokens
#[nutype(
    validate(greater = 0),
    derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)
)]
pub struct TotalTokens(u32);

/// Token usage information using type-safe token counts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: InputTokens,
    pub output_tokens: OutputTokens,
    pub total_tokens: TotalTokens,
}

impl TokenUsage {
    /// Create new token usage with automatic total calculation
    pub fn new(input_tokens: InputTokens, output_tokens: OutputTokens) -> Self {
        let total =
            TotalTokens::try_new(input_tokens.into_inner() + output_tokens.into_inner()).unwrap();
        Self {
            input_tokens,
            output_tokens,
            total_tokens: total,
        }
    }
}

/// Bedrock error response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BedrockError {
    #[serde(rename = "__type")]
    pub error_type: String,
    pub message: String,
}

// Model-specific request/response types will be added as needed
// For now, we'll use generic JSON values and add specific types
// as we implement model-specific handling
