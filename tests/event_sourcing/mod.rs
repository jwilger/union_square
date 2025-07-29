//! Comprehensive test infrastructure for event-sourced systems
//!
//! This module provides testing utilities and infrastructure for testing
//! event-sourced systems including fixtures, temporal testing, projections,
//! and concurrent operations.

pub mod concurrency;
pub mod fixtures;
pub mod integration;
pub mod projections;
pub mod temporal;