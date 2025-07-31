//! EventCore projections
//!
//! This module contains projection implementations for building read models
//! from the event stream.

pub mod builder;
pub mod core;
pub mod id_extraction;
pub mod monitoring;
pub mod postgres;
pub mod queries;
pub mod query_service;
pub mod read_models;
pub mod runner;
pub mod service;
pub mod session_summary;
