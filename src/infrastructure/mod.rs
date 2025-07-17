//! Infrastructure layer for Union Square
//!
//! This module contains the implementations for external concerns.
//! Currently includes:
//! - Database access via SQLx

pub mod database;

pub use database::*;
