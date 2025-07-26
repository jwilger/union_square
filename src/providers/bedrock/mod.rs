//! AWS Bedrock provider implementation
//!
//! This module implements the Bedrock-specific provider as the MVP provider
//! that demonstrates the proxy pattern for future providers.
//!
//! ## Features
//!
//! - SigV4 authentication pass-through
//! - Support for InvokeModel and InvokeModelWithResponseStream
//! - Model-specific request/response handling
//! - Cost calculation based on token usage
//! - Zero-copy streaming for minimal latency

pub mod auth;
pub mod models;
pub mod provider;
pub mod types;

pub use provider::BedrockProvider;
