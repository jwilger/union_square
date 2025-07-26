//! Provider abstraction and routing for LLM API providers
//!
//! This module implements the provider abstraction layer as defined in ADR-0011,
//! supporting multiple LLM providers with URL-based routing and preserving API compatibility.

pub mod bedrock;
pub mod response_processor;

use crate::proxy::types::ProxyError;
use async_trait::async_trait;
use axum::body::Body;
use hyper::{Request, Response};
use nutype::nutype;
use std::sync::Arc;

/// Provider identifier newtype for type safety
#[nutype(
    sanitize(trim, lowercase),
    validate(not_empty, regex = r"^[a-z][a-z0-9-]*$"),
    derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Display)
)]
pub struct ProviderId(String);

impl ProviderId {
    pub const BEDROCK: &'static str = "bedrock";
    pub const OPENAI: &'static str = "openai";
    pub const ANTHROPIC: &'static str = "anthropic";

    /// Create a new ProviderId for well-known providers without validation
    pub fn bedrock() -> Self {
        Self::try_new(Self::BEDROCK.to_string()).unwrap()
    }

    pub fn openai() -> Self {
        Self::try_new(Self::OPENAI.to_string()).unwrap()
    }

    pub fn anthropic() -> Self {
        Self::try_new(Self::ANTHROPIC.to_string()).unwrap()
    }
}

/// Request ID from provider for tracking
#[nutype(
    sanitize(trim),
    validate(not_empty, regex = r"^[a-zA-Z0-9-]+$"),
    derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Display)
)]
pub struct RequestId(String);

/// Registry of all available providers
#[derive(Default)]
pub struct ProviderRegistry {
    providers: Vec<Arc<dyn Provider>>,
}

impl ProviderRegistry {
    /// Create a new provider registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a provider
    pub fn register(&mut self, provider: Arc<dyn Provider>) {
        self.providers.push(provider);
    }

    /// Route a request to the appropriate provider based on path
    pub fn route(&self, path: &str) -> Option<Arc<dyn Provider>> {
        self.providers
            .iter()
            .find(|p| p.matches_path(path))
            .cloned()
    }
}

/// Core provider trait for LLM API providers
#[async_trait]
pub trait Provider: Send + Sync {
    /// Provider identifier (openai, anthropic, bedrock, etc.)
    fn id(&self) -> ProviderId;

    /// Check if this provider handles the given path
    fn matches_path(&self, path: &str) -> bool;

    /// Transform Union Square URL to provider URL
    fn transform_url(&self, url: &hyper::Uri) -> Result<hyper::Uri, ProviderError>;

    /// Validate and forward the request
    async fn forward_request(
        &self,
        request: Request<Body>,
        client: &hyper_util::client::legacy::Client<
            hyper_util::client::legacy::connect::HttpConnector,
            Body,
        >,
    ) -> Result<Response<Body>, ProviderError>;

    /// Extract metadata for audit logging
    fn extract_metadata(
        &self,
        request: &Request<Body>,
        response: &Response<Body>,
    ) -> ProviderMetadata;

    /// Provider-specific health check
    async fn health_check(
        &self,
        client: &hyper_util::client::legacy::Client<
            hyper_util::client::legacy::connect::HttpConnector,
            Body,
        >,
    ) -> HealthStatus;
}

/// Provider-specific error type
#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("Authentication error: {0}")]
    AuthenticationError(String),

    #[error("Provider unavailable: {0}")]
    Unavailable(String),

    #[error("Request failed: {0}")]
    RequestFailed(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<ProviderError> for ProxyError {
    fn from(err: ProviderError) -> Self {
        match err {
            ProviderError::InvalidPath(msg) | ProviderError::InvalidUrl(msg) => {
                ProxyError::InvalidTargetUrl(msg)
            }
            ProviderError::AuthenticationError(msg) => {
                ProxyError::Internal(format!("Authentication error: {msg}"))
            }
            ProviderError::Unavailable(msg) | ProviderError::RequestFailed(msg) => {
                ProxyError::Internal(format!("Provider error: {msg}"))
            }
            ProviderError::Internal(msg) => ProxyError::Internal(msg),
        }
    }
}

/// Provider metadata for audit logging using type-safe domain types
#[derive(Debug, Clone)]
pub struct ProviderMetadata {
    pub provider_id: ProviderId,
    pub model_id: Option<crate::providers::bedrock::types::ModelId>,
    pub request_tokens: Option<crate::providers::bedrock::types::InputTokens>,
    pub response_tokens: Option<crate::providers::bedrock::types::OutputTokens>,
    pub total_tokens: Option<crate::providers::bedrock::types::TotalTokens>,
    pub provider_request_id: Option<RequestId>,
}

impl Default for ProviderMetadata {
    fn default() -> Self {
        Self {
            provider_id: ProviderId::bedrock(), // Default to bedrock for now
            model_id: None,
            request_tokens: None,
            response_tokens: None,
            total_tokens: None,
            provider_request_id: None,
        }
    }
}

/// Health status for a provider
#[derive(Debug, Clone)]
pub enum HealthStatus {
    Healthy,
    Degraded(String),
    Unhealthy(String),
}
