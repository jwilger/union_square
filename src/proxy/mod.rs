//! Proxy module for handling LLM API requests
//!
//! This module implements the dual-path architecture:
//! - Hot path: Minimal latency forwarding (<5ms overhead)
//! - Audit path: Async processing via ring buffer

pub mod hot_path;
pub mod ring_buffer;
pub mod service;
pub mod types;

#[cfg(test)]
mod tests;

pub use service::ProxyService;
pub use types::{ProxyConfig, ProxyError, ProxyResult};
