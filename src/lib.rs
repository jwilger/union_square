//! Union Square - A proxy/wire-tap service for LLM calls
//!
//! This service intercepts and records all LLM interactions for later analysis
//! and test-case extraction, following type-driven development principles.

pub mod application;
pub mod config;
pub mod domain;
pub mod error;
pub mod infrastructure;

pub use application::Application;
pub use error::{Error, Result};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_basic_functionality() {
        // Basic smoke test to ensure the library compiles and basic types work
        let result: Result<()> = Ok(());
        assert!(result.is_ok());
    }
}
