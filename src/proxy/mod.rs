//! Proxy module for handling LLM API requests
//!
//! This module implements the dual-path architecture:
//! - Hot path: Minimal latency forwarding (<5ms overhead)
//! - Audit path: Async processing via ring buffer
//!
//! ## Module Organization
//!
//! - `core`: Core types and configuration
//! - `http`: HTTP-related types and utilities
//! - `middleware`: Request/response middleware components
//! - `paths`: Hot path and audit path implementations
//! - `storage`: Ring buffer and persistence

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
pub use middleware_stack::ProxyMiddlewareStack;

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
pub mod test_utils;

#[cfg(test)]
pub mod middleware_test_helpers;

// Main public exports
pub use service::ProxyService;
pub use types::{ProxyConfig, ProxyError, ProxyResult};
