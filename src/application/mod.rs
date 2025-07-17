//! Application services and business logic orchestration
//!
//! This module contains application services that coordinate
//! domain logic and infrastructure components.

pub mod app;
pub mod version_tracking;

pub use app::Application;
pub use version_tracking::{VersionStats, VersionTestService, VersionTrackingService};
