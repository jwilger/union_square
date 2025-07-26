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
    derive(Debug, Clone, PartialEq, Serialize, Deserialize)
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
    pub fn from_model_id(model_id: &str) -> Self {
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

/// Model pricing information
#[derive(Debug, Clone)]
pub struct ModelPricing {
    pub input_price_per_1k_tokens: f64,
    pub output_price_per_1k_tokens: f64,
}

impl ModelPricing {
    /// Get pricing for a specific model
    pub fn for_model(model_id: &str) -> Option<Self> {
        // Pricing as of 2025 - should be configurable in production
        match model_id {
            // Claude models
            "anthropic.claude-3-opus-20240229" => Some(Self {
                input_price_per_1k_tokens: 0.015,
                output_price_per_1k_tokens: 0.075,
            }),
            "anthropic.claude-3-sonnet-20240229" => Some(Self {
                input_price_per_1k_tokens: 0.003,
                output_price_per_1k_tokens: 0.015,
            }),
            "anthropic.claude-3-haiku-20240307" => Some(Self {
                input_price_per_1k_tokens: 0.00025,
                output_price_per_1k_tokens: 0.00125,
            }),
            // Titan models
            "amazon.titan-text-lite-v1" => Some(Self {
                input_price_per_1k_tokens: 0.0003,
                output_price_per_1k_tokens: 0.0004,
            }),
            "amazon.titan-text-express-v1" => Some(Self {
                input_price_per_1k_tokens: 0.0008,
                output_price_per_1k_tokens: 0.0016,
            }),
            // Llama models
            "meta.llama3-8b-instruct-v1" => Some(Self {
                input_price_per_1k_tokens: 0.0003,
                output_price_per_1k_tokens: 0.0006,
            }),
            "meta.llama3-70b-instruct-v1" => Some(Self {
                input_price_per_1k_tokens: 0.00265,
                output_price_per_1k_tokens: 0.0035,
            }),
            _ => None,
        }
    }

    /// Calculate cost for token usage
    pub fn calculate_cost(&self, input_tokens: u32, output_tokens: u32) -> f64 {
        let input_cost = (input_tokens as f64 / 1000.0) * self.input_price_per_1k_tokens;
        let output_cost = (output_tokens as f64 / 1000.0) * self.output_price_per_1k_tokens;
        input_cost + output_cost
    }
}

/// Token usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
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
