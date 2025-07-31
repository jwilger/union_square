//! Example demonstrating multi-stream query patterns in Union Square
//!
//! TODO: This example is temporarily disabled while we migrate from our custom
//! projection infrastructure to EventCore's built-in projection system.
//!
//! The query patterns shown here demonstrate valuable domain logic for
//! aggregating data across multiple event streams, but need to be
//! reimplemented using EventCore's native APIs.

fn main() {
    println!("This example is temporarily disabled.");
    println!("We are migrating from custom projection infrastructure to EventCore's built-in projection system.");
    println!("Please check back after the migration is complete.");
}

/* Original example code preserved for reference during migration:

use union_square::domain::{
    events::DomainEvent,
    llm::{LlmProvider, ModelVersion},
    metrics::Timestamp,
    session::{ApplicationId, SessionId},
    streams::session_stream,
    types::ModelId,
    user::UserId,
};
use union_square::infrastructure::eventcore::projections::{
    builder::ProjectionBuilder,
    queries::{get_session_events, get_sessions_by_version, ApplicationMetrics, VersionUsageStats},
    read_models::SessionSummary,
};

use eventcore::EventStore;
use eventcore_memory::InMemoryEventStore;
use std::collections::{HashMap, HashSet};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Union Square: Multi-Stream Query Examples\n");
    println!("This example demonstrates the projection and query capabilities");
    println!("without populating test data, focusing on the query patterns.\n");

    // Set up in-memory event store for demonstration
    let event_store = InMemoryEventStore::<DomainEvent>::new();

    // Create some test data references
    let user_id = UserId::generate();
    let session_id = SessionId::generate();
    let app_id = ApplicationId::try_new("example-app".to_string())?;

    println!("=== Example 1: Query Session Events ===");
    example_session_query(&event_store, &session_id).await?;

    println!("\n=== Example 2: Build Session Summary Projection ===");
    example_session_projection(&event_store, &session_id, &user_id, &app_id).await?;

    println!("\n=== Example 3: Multi-Stream Aggregation ===");
    example_multi_stream_aggregation(&event_store).await?;

    println!("\n=== Example 4: Version Usage Analysis ===");
    example_version_analysis(&event_store).await?;

    Ok(())
}

// ... rest of example functions ...
*/
