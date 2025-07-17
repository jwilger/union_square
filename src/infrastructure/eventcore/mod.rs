//! EventCore infrastructure implementation
//!
//! This module provides the EventCore event store integration
//! and adapters for Union Square.
//!
//! The full EventCore command pattern implementation is planned for
//! future iterations. Currently, we use an adapter pattern to emit
//! events that are ready for EventCore integration.

pub mod version_events;

pub use version_events::*;
