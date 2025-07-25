//! Proxy module for handling LLM API requests
//!
//! This module implements the dual-path architecture:
//! - Hot path: Minimal latency forwarding (<5ms overhead)
//! - Audit path: Async processing via ring buffer

pub mod audit_path;
pub mod audit_recorder;
pub mod hot_path;
pub mod middleware;
pub mod middleware_stack;
pub mod ring_buffer;
pub mod ring_buffer_performance_test;
pub mod service;
pub mod types;
pub mod url_resolver;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod middleware_tests;

#[cfg(test)]
mod integration_tests;

pub use service::ProxyService;
pub use types::{ProxyConfig, ProxyError, ProxyResult};
