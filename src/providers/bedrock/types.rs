//! Type definitions for AWS Bedrock provider

use currencies::{currency::USD, Amount};
use nutype::nutype;
use rust_decimal::Decimal;
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

/// Price per thousand tokens in USD using Decimal for precision
#[nutype(
    validate(predicate = |price| *price >= Decimal::ZERO),
    derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, AsRef)
)]
pub struct PricePerThousandTokens(Decimal);

impl PricePerThousandTokens {
    /// Create a new price from USD cents per 1K tokens
    pub fn from_usd_cents_per_1k(cents: i64) -> Self {
        let dollars = Decimal::from(cents) / Decimal::from(100);
        Self::try_new(dollars).unwrap()
    }

    /// Create a new price from USD dollars per 1K tokens (with precision handling)
    pub fn from_usd_dollars_per_1k(dollars: &str) -> Result<Self, rust_decimal::Error> {
        let decimal_value = dollars.parse::<Decimal>()?;
        Ok(Self::try_new(decimal_value).unwrap())
    }
}

/// Model pricing information using proper Money types
#[derive(Debug, Clone)]
pub struct ModelPricing {
    pub input_price_per_1k_tokens: PricePerThousandTokens,
    pub output_price_per_1k_tokens: PricePerThousandTokens,
}

impl ModelPricing {
    /// Get pricing for a specific model
    pub fn for_model(model_id: &str) -> Option<Self> {
        // Pricing as of 2025 - should be configurable in production
        match model_id {
            // Claude models - using precise decimal strings to avoid float precision issues
            "anthropic.claude-3-opus-20240229" => Some(Self {
                input_price_per_1k_tokens: PricePerThousandTokens::from_usd_dollars_per_1k("0.015")
                    .unwrap(),
                output_price_per_1k_tokens: PricePerThousandTokens::from_usd_dollars_per_1k(
                    "0.075",
                )
                .unwrap(),
            }),
            "anthropic.claude-3-sonnet-20240229" => Some(Self {
                input_price_per_1k_tokens: PricePerThousandTokens::from_usd_dollars_per_1k("0.003")
                    .unwrap(),
                output_price_per_1k_tokens: PricePerThousandTokens::from_usd_dollars_per_1k(
                    "0.015",
                )
                .unwrap(),
            }),
            "anthropic.claude-3-haiku-20240307" => Some(Self {
                input_price_per_1k_tokens: PricePerThousandTokens::from_usd_dollars_per_1k(
                    "0.00025",
                )
                .unwrap(),
                output_price_per_1k_tokens: PricePerThousandTokens::from_usd_dollars_per_1k(
                    "0.00125",
                )
                .unwrap(),
            }),
            // Titan models
            "amazon.titan-text-lite-v1" => Some(Self {
                input_price_per_1k_tokens: PricePerThousandTokens::from_usd_dollars_per_1k(
                    "0.0003",
                )
                .unwrap(),
                output_price_per_1k_tokens: PricePerThousandTokens::from_usd_dollars_per_1k(
                    "0.0004",
                )
                .unwrap(),
            }),
            "amazon.titan-text-express-v1" => Some(Self {
                input_price_per_1k_tokens: PricePerThousandTokens::from_usd_dollars_per_1k(
                    "0.0008",
                )
                .unwrap(),
                output_price_per_1k_tokens: PricePerThousandTokens::from_usd_dollars_per_1k(
                    "0.0016",
                )
                .unwrap(),
            }),
            // Llama models
            "meta.llama3-8b-instruct-v1" => Some(Self {
                input_price_per_1k_tokens: PricePerThousandTokens::from_usd_dollars_per_1k(
                    "0.0003",
                )
                .unwrap(),
                output_price_per_1k_tokens: PricePerThousandTokens::from_usd_dollars_per_1k(
                    "0.0006",
                )
                .unwrap(),
            }),
            "meta.llama3-70b-instruct-v1" => Some(Self {
                input_price_per_1k_tokens: PricePerThousandTokens::from_usd_dollars_per_1k(
                    "0.00265",
                )
                .unwrap(),
                output_price_per_1k_tokens: PricePerThousandTokens::from_usd_dollars_per_1k(
                    "0.0035",
                )
                .unwrap(),
            }),
            _ => None,
        }
    }

    /// Calculate cost for token usage returning proper Money type
    pub fn calculate_cost(
        &self,
        input_tokens: InputTokens,
        output_tokens: OutputTokens,
    ) -> Amount<USD> {
        // Calculate input cost: (tokens / 1000) * price_per_1k
        let input_ratio = Decimal::from(input_tokens.into_inner()) / Decimal::from(1000);
        let input_cost_decimal = self.input_price_per_1k_tokens.as_ref() * input_ratio;

        // Calculate output cost: (tokens / 1000) * price_per_1k
        let output_ratio = Decimal::from(output_tokens.into_inner()) / Decimal::from(1000);
        let output_cost_decimal = self.output_price_per_1k_tokens.as_ref() * output_ratio;

        // Total cost in dollars
        let total_cost_decimal = input_cost_decimal + output_cost_decimal;

        // Convert to cents (multiply by 100) and round UP to next cent (ceiling)
        let total_cents = (total_cost_decimal * Decimal::from(100)).ceil();

        // Convert to u64 for Amount (safe because we're dealing with reasonable cost values)
        // Use try_into() to convert Decimal to u64
        let cents_u64 = total_cents.try_into().unwrap_or(0);

        // Return as Money
        Amount::<USD>::from_raw(cents_u64)
    }
}

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
