//! Example demonstrating multi-stream query patterns in Union Square
//!
//! This example shows how to use the projection builder and query functions
//! to aggregate data from multiple event streams.

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

    println!("\n=== Example 5: Custom Time-Based Projection ===");
    example_time_based_projection(&event_store).await?;

    demonstrate_application_metrics();

    Ok(())
}

/// Example 1: Query all events for a session
async fn example_session_query<ES: EventStore>(
    event_store: &ES,
    session_id: &SessionId,
) -> Result<(), Box<dyn std::error::Error>>
where
    ES::Event: TryFrom<DomainEvent> + Clone,
    DomainEvent: for<'a> TryFrom<&'a ES::Event>,
{
    println!("Querying events for session: {session_id}");

    // Use the query function to get all session events
    let events = get_session_events(event_store, session_id).await?;

    println!("Found {} events (empty in this demo)", events.len());
    println!("In a real system, this would include:");
    println!("  - SessionStarted");
    println!("  - LlmRequestReceived");
    println!("  - LlmResponseReceived");
    println!("  - FScoreCalculated");
    println!("  - SessionEnded");

    Ok(())
}

/// Example 2: Build a session summary using projection
async fn example_session_projection<ES: EventStore>(
    event_store: &ES,
    session_id: &SessionId,
    user_id: &UserId,
    app_id: &ApplicationId,
) -> Result<(), Box<dyn std::error::Error>>
where
    ES::Event: TryFrom<DomainEvent> + Clone,
    DomainEvent: for<'a> TryFrom<&'a ES::Event>,
{
    println!("Building session summary projection...");

    let summary = ProjectionBuilder::new(SessionSummary::new(
        session_id.clone(),
        user_id.clone(),
        app_id.clone(),
        Timestamp::now(),
    ))
    .with_stream(session_stream(session_id))
    .project_with(|mut summary, event| {
        match &event.payload {
            DomainEvent::LlmRequestStarted { .. } => {
                summary.total_requests += 1;
            }
            DomainEvent::LlmResponseReceived { .. } => {
                summary.successful_requests += 1;
            }
            DomainEvent::SessionEnded {
                ended_at,
                final_status,
                ..
            } => {
                summary.end_session(*ended_at, final_status.clone());
            }
            _ => {}
        }
        summary
    })
    .execute(event_store)
    .await?;

    println!("Session Summary Created:");
    println!("  Session ID: {}", summary.session_id);
    println!("  Status: {:?}", summary.status);
    println!("  Total Requests: {}", summary.total_requests);
    println!("  Successful: {}", summary.successful_requests);

    Ok(())
}

/// Example 3: Aggregate data from multiple streams
async fn example_multi_stream_aggregation<ES: EventStore>(
    event_store: &ES,
) -> Result<(), Box<dyn std::error::Error>>
where
    ES::Event: TryFrom<DomainEvent> + Clone,
    DomainEvent: for<'a> TryFrom<&'a ES::Event>,
{
    println!("Demonstrating multi-stream aggregation pattern...");

    // Create multiple session IDs for the example
    let session_ids: Vec<SessionId> = (0..3).map(|_| SessionId::generate()).collect();

    #[derive(Debug, Clone)]
    struct CrossSessionMetrics {
        total_events: usize,
        sessions_with_metrics: HashSet<SessionId>,
        model_versions_used: HashSet<ModelVersion>,
    }

    let initial_state = CrossSessionMetrics {
        total_events: 0,
        sessions_with_metrics: HashSet::new(),
        model_versions_used: HashSet::new(),
    };

    // Build projection across multiple session streams
    let streams: Vec<_> = session_ids.iter().map(session_stream).collect();

    let metrics = ProjectionBuilder::new(initial_state)
        .with_streams(streams)
        .project_with(|mut state, event| {
            state.total_events += 1;

            if let DomainEvent::FScoreCalculated {
                session_id,
                model_version,
                ..
            } = &event.payload
            {
                state.sessions_with_metrics.insert(session_id.clone());
                state.model_versions_used.insert(model_version.clone());
            }

            state
        })
        .execute(event_store)
        .await?;

    println!("Cross-Session Metrics:");
    println!("  Total Events Processed: {}", metrics.total_events);
    println!(
        "  Sessions with Metrics: {}",
        metrics.sessions_with_metrics.len()
    );
    println!(
        "  Model Versions Used: {}",
        metrics.model_versions_used.len()
    );

    Ok(())
}

/// Example 4: Analyze version usage patterns
async fn example_version_analysis<ES: EventStore>(
    event_store: &ES,
) -> Result<(), Box<dyn std::error::Error>>
where
    ES::Event: TryFrom<DomainEvent> + Clone,
    DomainEvent: for<'a> TryFrom<&'a ES::Event>,
{
    println!("Analyzing model version usage...");

    let model_version = ModelVersion {
        provider: LlmProvider::Anthropic,
        model_id: ModelId::try_new("claude-3-opus".to_string())?,
    };

    // Find all sessions that used this model version
    let sessions = get_sessions_by_version(event_store, &model_version).await?;

    println!("Model Version: {}", model_version.model_id);
    println!("Used in {} sessions", sessions.len());

    // In a real system, we'd aggregate metrics like:
    let usage_stats = VersionUsageStats {
        model_version: model_version.clone(),
        total_requests: 0,
        unique_sessions: sessions.len(),
        unique_users: 0,
        average_requests_per_session: 0.0,
    };

    println!("\nUsage Statistics:");
    println!("  Total Requests: {}", usage_stats.total_requests);
    println!("  Unique Sessions: {}", usage_stats.unique_sessions);
    println!(
        "  Average Requests/Session: {:.2}",
        usage_stats.average_requests_per_session
    );

    Ok(())
}

/// Example 5: Time-based projection with filtering
async fn example_time_based_projection<ES: EventStore>(
    event_store: &ES,
) -> Result<(), Box<dyn std::error::Error>>
where
    ES::Event: TryFrom<DomainEvent> + Clone,
    DomainEvent: for<'a> TryFrom<&'a ES::Event>,
{
    println!("Creating time-based projection with filtering...");

    let session_id = SessionId::generate();
    let end_time = eventcore::Timestamp::now();
    let start_time = end_time; // In real usage, this would be calculated

    #[derive(Debug, Clone)]
    struct TimeWindowMetrics {
        events_in_window: usize,
        event_types: HashMap<String, usize>,
    }

    let projection = ProjectionBuilder::new(TimeWindowMetrics {
        events_in_window: 0,
        event_types: HashMap::new(),
    })
    .with_stream(session_stream(&session_id))
    .within_time_range(start_time, end_time)
    .project_with(|mut state, event| {
        state.events_in_window += 1;

        let event_type = match &event.payload {
            DomainEvent::SessionStarted { .. } => "SessionStarted",
            DomainEvent::LlmRequestReceived { .. } => "LlmRequestReceived",
            DomainEvent::LlmResponseReceived { .. } => "LlmResponseReceived",
            DomainEvent::FScoreCalculated { .. } => "FScoreCalculated",
            _ => "Other",
        };

        *state.event_types.entry(event_type.to_string()).or_insert(0) += 1;
        state
    })
    .execute(event_store)
    .await?;

    println!("Time Window Analysis:");
    println!("  Events in Window: {}", projection.events_in_window);
    println!("  Event Type Distribution: {:?}", projection.event_types);

    Ok(())
}

/// Example showing how application metrics would be calculated
fn demonstrate_application_metrics() {
    println!("\n=== Application Metrics Pattern ===");

    let app_id = ApplicationId::try_new("demo-app".to_string()).unwrap();

    // This shows the structure of application metrics
    let metrics = ApplicationMetrics {
        application_id: app_id.clone(),
        total_sessions: 100,
        total_requests: 1500,
        unique_users: 25,
        model_versions_used: HashSet::from([
            ModelVersion {
                provider: LlmProvider::Anthropic,
                model_id: ModelId::try_new("claude-3-opus".to_string()).unwrap(),
            },
            ModelVersion {
                provider: LlmProvider::OpenAI,
                model_id: ModelId::try_new("gpt-4".to_string()).unwrap(),
            },
        ]),
        average_session_length: Duration::from_secs(300),
    };

    println!("Application: {}", metrics.application_id);
    println!("  Total Sessions: {}", metrics.total_sessions);
    println!("  Total Requests: {}", metrics.total_requests);
    println!("  Unique Users: {}", metrics.unique_users);
    println!("  Model Versions: {}", metrics.model_versions_used.len());
    println!("  Avg Session Length: {:?}", metrics.average_session_length);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_projection_examples_compile() {
        // Simple test to ensure the examples compile and run
        let event_store = InMemoryEventStore::<DomainEvent>::new();
        let session_id = SessionId::generate();
        let user_id = UserId::generate();
        let app_id = ApplicationId::try_new("test".to_string()).unwrap();

        // Test that query functions work
        let result = example_session_query(&event_store, &session_id).await;
        assert!(result.is_ok());

        // Test that projection works
        let result = example_session_projection(&event_store, &session_id, &user_id, &app_id).await;
        assert!(result.is_ok());
    }
}
