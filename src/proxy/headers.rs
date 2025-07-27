//! HTTP header constants and utilities for the proxy service
//!
//! This module centralizes all HTTP header names and header-related
//! constants used throughout the proxy service to ensure consistency
//! and make maintenance easier.

use ::http::header;

/// Custom header name for the target URL that the proxy should forward requests to
pub const X_TARGET_URL: &str = "x-target-url";

/// Header name for request ID used for tracing and correlation
pub const X_REQUEST_ID: &str = "x-request-id";

/// Header name for session ID used for grouping related requests
pub const X_SESSION_ID: &str = "x-session-id";

/// Header name for API key authentication
pub const X_API_KEY: &str = "x-api-key";

/// Authorization header prefix for bearer tokens
pub const BEARER_PREFIX: &str = "Bearer ";

/// Standard header re-exports for convenience
pub use header::{AUTHORIZATION, CONTENT_LENGTH, CONTENT_TYPE, HOST, USER_AGENT};

/// Well-known paths
pub mod paths {
    /// Default path when none is specified
    pub const DEFAULT: &str = "/";

    /// Health check endpoint path
    pub const HEALTH: &str = "/health";

    /// Metrics endpoint path
    pub const METRICS: &str = "/metrics";
}

/// Common content types (re-exported from centralized constants)
pub mod content_types {
    pub use crate::providers::constants::http::content_types::*;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_constants() {
        // Ensure header names follow conventions
        assert!(X_TARGET_URL.starts_with("x-"));
        assert!(X_REQUEST_ID.starts_with("x-"));
        assert!(X_SESSION_ID.starts_with("x-"));

        // Ensure paths are valid
        assert!(paths::DEFAULT.starts_with('/'));
        assert!(paths::HEALTH.starts_with('/'));
        assert!(paths::METRICS.starts_with('/'));

        // Ensure bearer prefix has proper format
        assert!(BEARER_PREFIX.ends_with(' '));
    }
}
