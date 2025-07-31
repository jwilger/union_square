//! Domain types and business logic for Union Square
//!
//! This module contains the core domain types that represent the business
//! concepts of Union Square, following type-driven development principles.

pub mod commands;
pub mod config_types;
pub mod events;
pub mod identifiers;
pub mod llm;
pub mod metrics;
pub mod network_types;
pub mod session;
pub mod streams;
pub mod test_case;
pub mod test_data;
pub mod types;
pub mod user;
pub mod validation_constants;
pub mod version;

// Example modules demonstrating EventCore patterns
#[cfg(test)]
pub mod analysis_events;
#[cfg(test)]
pub mod commands_test;

pub use events::*;
pub use identifiers::{AnalysisId, ExtractionId};
pub use llm::*;
pub use metrics::*;
pub use session::*;
pub use test_case::*;
pub use user::*;
pub use version::*;
