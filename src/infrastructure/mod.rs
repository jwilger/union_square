//! Infrastructure layer for Union Square
//!
//! This module contains the implementations for external concerns.
//! Currently includes:
//! - Database access via SQLx
//! - EventCore integration for event sourcing

pub mod database;
pub mod eventcore;
pub mod log_messages;

pub use database::*;
