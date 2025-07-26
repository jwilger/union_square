//! Proxy module for handling LLM API requests
//!
//! This module implements a high-performance proxy service with dual-path architecture
//! for LLM API calls, designed to minimize latency while capturing comprehensive audit data.
//!
//! ## Architecture Overview
//!
//! The proxy uses a dual-path architecture to achieve both low latency and comprehensive auditing:
//!
//! ```text
//! ┌─────────────┐     ┌──────────────┐     ┌─────────────┐
//! │   Client    │────▶│ Tower Stack  │────▶│  Hot Path   │
//! └─────────────┘     └──────────────┘     └──────┬──────┘
//!                            │                     │
//!                            ▼                     ▼
//!                     ┌──────────────┐     ┌──────────────┐
//!                     │  Middleware  │     │ Ring Buffer  │
//!                     └──────────────┘     └──────┬───────┘
//!                                                 │
//!                                                 ▼
//!                                          ┌──────────────┐
//!                                          │ Audit Path   │
//!                                          └──────────────┘
//! ```
//!
//! ### Hot Path (<5ms latency)
//! - Zero-copy streaming of requests/responses
//! - Minimal processing overhead
//! - Fire-and-forget writes to ring buffer
//!
//! ### Audit Path (async processing)
//! - Consumes events from ring buffer
//! - Persists to database
//! - Handles analytics and monitoring
//!
//! ## Module Organization
//!
//! - `types`: Core domain types with validation
//! - `http`: HTTP utilities and type-safe wrappers
//! - `middleware`: Tower middleware stack components
//! - `paths`: Hot path and audit path implementations
//! - `storage`: Ring buffer for event passing
//!
//! ## Example Usage
//!
//! ```rust,ignore
//! use union_square::proxy::{ProxyService, ProxyConfig, ProxyMiddlewareConfig};
//!
//! // Configure the proxy
//! let config = ProxyConfig::default();
//! let service = ProxyService::new(config);
//!
//! // Configure middleware
//! let middleware_config = ProxyMiddlewareConfig::default()
//!     .disable_metrics()  // Disable metrics endpoint
//!     .enable_detailed_errors();  // Enable detailed errors for debugging
//!
//! // Create router with middleware
//! let auth_config = middleware_config.auth.clone();
//! let router = service.into_router(auth_config);
//!
//! // Run the server
//! let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
//! axum::serve(listener, router).await?;
//! ```

// Core types and configuration
pub mod service;
pub mod types;

// HTTP utilities and types
pub mod http {
    pub use super::error_response::{ErrorResponse, ErrorResponseExt};
    pub use super::headers::*;
    pub use super::http_types::*;
    pub use super::url_resolver::{UrlResolver, UrlResolverConfig};
}

// Middleware components
pub use middleware::*;
pub use middleware_stack::{ProxyMiddlewareConfig, ProxyMiddlewareStack};

// Path implementations
pub mod paths {
    pub use super::audit_path::AuditPathProcessor;
    pub use super::audit_recorder::{
        extract_headers_vec, parse_http_method, parse_http_status, parse_request_uri,
        AuditRecorder, ChunkCapture, RingBufferAuditRecorder, CAPTURE_CHUNK_SIZE,
    };
    pub use super::hot_path::StreamingHotPathService;
}

// Storage and persistence
pub mod storage {
    pub use super::ring_buffer::{RingBuffer, RingBufferStats};
}

// Internal modules (not part of public API)
mod audit_path;
mod audit_recorder;
mod error_response;
mod headers;
mod hot_path;
mod http_types;
mod middleware;
mod middleware_stack;
mod provider_router;
mod ring_buffer;
mod url_resolver;

// Test modules
#[cfg(test)]
mod ring_buffer_performance_test;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod middleware_tests;

#[cfg(test)]
mod integration_tests;

#[cfg(test)]
mod provider_integration_tests;

#[cfg(test)]
pub mod test_utils;

#[cfg(test)]
pub mod middleware_test_helpers;

// Main public exports
pub use service::ProxyService;
pub use types::{ProxyConfig, ProxyError, ProxyResult};
