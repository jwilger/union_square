//! Version capture functionality for extracting version information from provider responses
//!
//! This module provides the capability to extract and parse version information
//! from different LLM provider responses, handling provider-specific formats.

use crate::domain::{
    ApiVersion, ExtendedModelVersion, LlmProvider, ModelName, ModelVersionString,
    ProviderVersionInfo,
};
use serde_json::Value;
use std::collections::HashMap;

/// Trait for capturing version information from provider responses
pub trait VersionCapture: Send + Sync {
    /// Extract version information from response headers and body
    fn capture_version(
        &self,
        provider: &LlmProvider,
        headers: &HashMap<String, String>,
        response_body: &Value,
    ) -> Result<ExtendedModelVersion, VersionCaptureError>;
}

/// Errors that can occur during version capture
#[derive(Debug, thiserror::Error)]
pub enum VersionCaptureError {
    #[error("Missing required field: {field}")]
    MissingField { field: String },

    #[error("Invalid field value: {field} = {value}. {message}")]
    InvalidFieldValue {
        field: String,
        value: String,
        message: String,
    },

    #[error("Unsupported provider: {0:?}")]
    UnsupportedProvider(LlmProvider),

    #[error("Version parsing failed: {0}")]
    ParsingError(String),
}

/// Default implementation of version capture
pub struct DefaultVersionCapture;

impl VersionCapture for DefaultVersionCapture {
    fn capture_version(
        &self,
        provider: &LlmProvider,
        headers: &HashMap<String, String>,
        response_body: &Value,
    ) -> Result<ExtendedModelVersion, VersionCaptureError> {
        match provider {
            LlmProvider::OpenAI => capture_openai_version(headers, response_body),
            LlmProvider::Anthropic => capture_anthropic_version(headers, response_body),
            LlmProvider::Google => capture_vertex_ai_version(headers, response_body),
            LlmProvider::Azure => capture_azure_version(headers, response_body),
            LlmProvider::Other(name) => capture_generic_version(name, headers, response_body),
        }
    }
}

/// Capture OpenAI version information
fn capture_openai_version(
    headers: &HashMap<String, String>,
    response_body: &Value,
) -> Result<ExtendedModelVersion, VersionCaptureError> {
    // Extract model name from response
    let model_name = response_body
        .get("model")
        .and_then(|v| v.as_str())
        .ok_or_else(|| VersionCaptureError::MissingField {
            field: "model".to_string(),
        })?;

    let model_name = ModelName::try_new(model_name.to_string()).map_err(|e| {
        VersionCaptureError::InvalidFieldValue {
            field: "model".to_string(),
            value: model_name.to_string(),
            message: format!("Original error: {e}"),
        }
    })?;

    // Extract system fingerprint if available
    let system_fingerprint = response_body
        .get("system_fingerprint")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // Extract API version from headers
    let api_version = headers
        .get("openai-version")
        .cloned()
        .unwrap_or_else(|| "2023-12-01".to_string());

    let api_version = ApiVersion::try_new(api_version.clone()).map_err(|e| {
        VersionCaptureError::InvalidFieldValue {
            field: "api_version".to_string(),
            value: api_version,
            message: format!("Original error: {e}"),
        }
    })?;

    // Store raw model name without parsing - version info comes from explicit API fields
    let model_version = None;

    Ok(ExtendedModelVersion::new(
        LlmProvider::OpenAI,
        model_name,
        ProviderVersionInfo::OpenAI {
            model_version,
            api_version,
            system_fingerprint,
        },
    ))
}

// Note: We no longer parse version information from model names to avoid
// fragility when providers change their naming schemes. Version information
// should come from explicit API fields (headers, response metadata) when available.

/// Capture Anthropic version information
fn capture_anthropic_version(
    headers: &HashMap<String, String>,
    response_body: &Value,
) -> Result<ExtendedModelVersion, VersionCaptureError> {
    // Extract model name from response
    let model_name = response_body
        .get("model")
        .and_then(|v| v.as_str())
        .ok_or_else(|| VersionCaptureError::MissingField {
            field: "model".to_string(),
        })?;

    let model_name = ModelName::try_new(model_name.to_string()).map_err(|e| {
        VersionCaptureError::InvalidFieldValue {
            field: "model".to_string(),
            value: model_name.to_string(),
            message: format!("Original error: {e}"),
        }
    })?;

    // Extract API version from headers
    let api_version = headers
        .get("anthropic-version")
        .cloned()
        .unwrap_or_else(|| "2023-06-01".to_string());

    let api_version = ApiVersion::try_new(api_version.clone()).map_err(|e| {
        VersionCaptureError::InvalidFieldValue {
            field: "api_version".to_string(),
            value: api_version,
            message: format!("Original error: {e}"),
        }
    })?;

    // Store raw model name without parsing - use explicit version fields when available
    let model_version = ModelVersionString::try_new("unknown".to_string()).map_err(|e| {
        VersionCaptureError::InvalidFieldValue {
            field: "model_version".to_string(),
            value: "unknown".to_string(),
            message: format!("Original error: {e}"),
        }
    })?;

    Ok(ExtendedModelVersion::new(
        LlmProvider::Anthropic,
        model_name,
        ProviderVersionInfo::Anthropic {
            model_version,
            api_version,
            capabilities_version: None,
        },
    ))
}

// Removed complex model name parsing functions to avoid fragility.
// Model names are stored as-is, version info comes from explicit API fields.

/// Capture Vertex AI version information
fn capture_vertex_ai_version(
    headers: &HashMap<String, String>,
    response_body: &Value,
) -> Result<ExtendedModelVersion, VersionCaptureError> {
    // For Vertex AI, model info might be in metadata
    let metadata =
        response_body
            .get("metadata")
            .ok_or_else(|| VersionCaptureError::MissingField {
                field: "metadata".to_string(),
            })?;

    let model_name = metadata
        .get("model")
        .and_then(|v| v.as_str())
        .ok_or_else(|| VersionCaptureError::MissingField {
            field: "metadata.model".to_string(),
        })?;

    let model_name = ModelName::try_new(model_name.to_string()).map_err(|e| {
        VersionCaptureError::InvalidFieldValue {
            field: "model".to_string(),
            value: model_name.to_string(),
            message: format!("Original error: {e}"),
        }
    })?;

    let version = metadata
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("latest");

    let version = ModelVersionString::try_new(version.to_string()).map_err(|e| {
        VersionCaptureError::InvalidFieldValue {
            field: "version".to_string(),
            value: version.to_string(),
            message: format!("Original error: {e}"),
        }
    })?;

    let location = headers
        .get("x-goog-api-location")
        .cloned()
        .unwrap_or_else(|| "us-central1".to_string());

    Ok(ExtendedModelVersion::new(
        LlmProvider::Google,
        model_name.clone(),
        ProviderVersionInfo::VertexAI {
            model: model_name,
            version,
            location,
        },
    ))
}

/// Capture Azure OpenAI version information
fn capture_azure_version(
    headers: &HashMap<String, String>,
    response_body: &Value,
) -> Result<ExtendedModelVersion, VersionCaptureError> {
    // Azure uses similar format to OpenAI but with deployment names
    let openai_version = capture_openai_version(headers, response_body)?;

    // Create a new version with Azure as the provider
    Ok(ExtendedModelVersion::new(
        LlmProvider::Azure,
        openai_version.model_name,
        openai_version.version_info,
    ))
}

/// Capture generic version information for unknown providers
fn capture_generic_version(
    provider_name: &str,
    headers: &HashMap<String, String>,
    response_body: &Value,
) -> Result<ExtendedModelVersion, VersionCaptureError> {
    // Try to extract model name from common fields
    let model_name = response_body
        .get("model")
        .or_else(|| response_body.get("model_name"))
        .or_else(|| response_body.get("model_id"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| VersionCaptureError::MissingField {
            field: "model".to_string(),
        })?;

    let model_name = ModelName::try_new(model_name.to_string()).map_err(|e| {
        VersionCaptureError::InvalidFieldValue {
            field: "model".to_string(),
            value: model_name.to_string(),
            message: format!("Original error: {e}"),
        }
    })?;

    // Collect all version-related data
    let mut version_data = serde_json::Map::new();

    // Add relevant headers
    for (key, value) in headers {
        if key.contains("version") || key.contains("model") {
            version_data.insert(key.clone(), Value::String(value.clone()));
        }
    }

    // Add relevant fields from response
    if let Some(obj) = response_body.as_object() {
        for (key, value) in obj {
            if key.contains("version") || key.contains("model") {
                version_data.insert(key.clone(), value.clone());
            }
        }
    }

    Ok(ExtendedModelVersion::new(
        LlmProvider::Other(provider_name.to_string()),
        model_name,
        ProviderVersionInfo::Other {
            provider_name: provider_name.to_string(),
            version_data: Value::Object(version_data),
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_capture_openai_version_with_system_fingerprint() {
        let capture = DefaultVersionCapture;
        let mut headers = HashMap::new();
        headers.insert("openai-version".to_string(), "2023-12-01".to_string());

        let response_body = json!({
            "model": "gpt-4-1106-preview",
            "system_fingerprint": "fp_123456789"
        });

        let result = capture.capture_version(&LlmProvider::OpenAI, &headers, &response_body);
        assert!(result.is_ok());

        let version = result.unwrap();
        assert_eq!(version.provider, LlmProvider::OpenAI);
        assert_eq!(version.model_name.as_ref(), "gpt-4-1106-preview");

        match version.version_info {
            ProviderVersionInfo::OpenAI {
                model_version,
                api_version,
                system_fingerprint,
            } => {
                // We no longer parse version from model name
                assert!(model_version.is_none());
                assert_eq!(api_version.as_ref(), "2023-12-01");
                assert_eq!(system_fingerprint, Some("fp_123456789".to_string()));
            }
            _ => panic!("Expected OpenAI version info"),
        }
    }

    #[test]
    fn test_capture_anthropic_version() {
        let capture = DefaultVersionCapture;
        let mut headers = HashMap::new();
        headers.insert("anthropic-version".to_string(), "2023-06-01".to_string());

        let response_body = json!({
            "model": "claude-3-opus-20240229"
        });

        let result = capture.capture_version(&LlmProvider::Anthropic, &headers, &response_body);
        assert!(result.is_ok());

        let version = result.unwrap();
        assert_eq!(version.provider, LlmProvider::Anthropic);

        match version.version_info {
            ProviderVersionInfo::Anthropic {
                model_version,
                api_version,
                ..
            } => {
                // We use "unknown" as a placeholder since we don't parse model names
                assert_eq!(model_version.as_ref(), "unknown");
                assert_eq!(api_version.as_ref(), "2023-06-01");
            }
            _ => panic!("Expected Anthropic version info"),
        }
    }

    #[test]
    fn test_capture_version_missing_model() {
        let capture = DefaultVersionCapture;
        let headers = HashMap::new();
        let response_body = json!({});

        let result = capture.capture_version(&LlmProvider::OpenAI, &headers, &response_body);
        assert!(result.is_err());

        match result.unwrap_err() {
            VersionCaptureError::MissingField { field } => {
                assert_eq!(field, "model");
            }
            _ => panic!("Expected MissingField error"),
        }
    }

    #[test]
    fn test_openai_version_capture_without_parsing() {
        let capture = DefaultVersionCapture;
        let mut headers = HashMap::new();
        headers.insert("openai-version".to_string(), "2023-12-01".to_string());

        let response_body = json!({"model": "gpt-4-1106-preview"});

        let result = capture.capture_version(&LlmProvider::OpenAI, &headers, &response_body);
        assert!(result.is_ok());

        let version = result.unwrap();
        // We no longer parse version from model name, so model_version should be None
        match version.version_info {
            ProviderVersionInfo::OpenAI { model_version, .. } => {
                assert!(model_version.is_none());
            }
            _ => panic!("Expected OpenAI version info"),
        }
    }

    #[test]
    fn test_anthropic_version_capture_without_parsing() {
        let capture = DefaultVersionCapture;
        let mut headers = HashMap::new();
        headers.insert("anthropic-version".to_string(), "2023-06-01".to_string());

        let response_body = json!({"model": "claude-3-opus-20240229"});

        let result = capture.capture_version(&LlmProvider::Anthropic, &headers, &response_body);
        assert!(result.is_ok());

        let version = result.unwrap();
        // We store "unknown" as a placeholder since we don't parse model names anymore
        match version.version_info {
            ProviderVersionInfo::Anthropic { model_version, .. } => {
                assert_eq!(model_version.as_ref(), "unknown");
            }
            _ => panic!("Expected Anthropic version info"),
        }
    }
}
