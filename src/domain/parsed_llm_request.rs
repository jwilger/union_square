//! Parsed LLM request types
//!
//! These domain types represent the result of parsing an LLM provider request.
//! The actual parsing logic lives in the adapter layer.

use crate::domain::{
    llm::ModelVersion,
    types::{LlmParameters, Prompt},
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
pub type ParseResult<T> = Result<T, ParseError>;

/// Parsed LLM request data
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedLlmRequest {
    pub model_version: ModelVersion,
    pub prompt: Prompt,
    pub parameters: LlmParameters,
}
