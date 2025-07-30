//! Example demonstrating the projection concept
//!
//! This example shows how projections work conceptually without
//! the full EventCore integration complexity.

use std::collections::HashMap;

// Simplified event types
#[derive(Debug, Clone)]
enum Event {
    SessionStarted {
        session_id: String,
        user_id: String,
    },
    RequestReceived {
        session_id: String,
        model: String,
    },
    RequestCompleted {
        session_id: String,
        duration_ms: u64,
    },
}

// Projection state
#[derive(Debug, Clone)]
struct SessionSummaryState {
    summaries: HashMap<String, SessionSummary>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct SessionSummary {
    session_id: String,
    user_id: String,
    request_count: usize,
    models_used: Vec<String>,
    total_duration_ms: u64,
}

// Functional projection
struct SessionSummaryProjection;

impl SessionSummaryProjection {
    fn initial_state() -> SessionSummaryState {
        SessionSummaryState {
            summaries: HashMap::new(),
        }
    }

    fn apply_event(state: &SessionSummaryState, event: &Event) -> SessionSummaryState {
        let mut new_state = state.clone();

        match event {
            Event::SessionStarted {
                session_id,
                user_id,
            } => {
                new_state.summaries.insert(
                    session_id.clone(),
                    SessionSummary {
                        session_id: session_id.clone(),
                        user_id: user_id.clone(),
                        request_count: 0,
                        models_used: Vec::new(),
                        total_duration_ms: 0,
                    },
                );
            }
            Event::RequestReceived { session_id, model } => {
                if let Some(summary) = new_state.summaries.get_mut(session_id) {
                    summary.request_count += 1;
                    if !summary.models_used.contains(model) {
                        summary.models_used.push(model.clone());
                    }
                }
            }
            Event::RequestCompleted {
                session_id,
                duration_ms,
            } => {
                if let Some(summary) = new_state.summaries.get_mut(session_id) {
                    summary.total_duration_ms += duration_ms;
                }
            }
        }

        new_state
    }
}

fn main() {
    println!("Projection Concept Example\n");

    // Create events
    let events = vec![
        Event::SessionStarted {
            session_id: "session-1".to_string(),
            user_id: "user-1".to_string(),
        },
        Event::RequestReceived {
            session_id: "session-1".to_string(),
            model: "gpt-4".to_string(),
        },
        Event::RequestCompleted {
            session_id: "session-1".to_string(),
            duration_ms: 150,
        },
        Event::RequestReceived {
            session_id: "session-1".to_string(),
            model: "claude-3".to_string(),
        },
        Event::RequestCompleted {
            session_id: "session-1".to_string(),
            duration_ms: 200,
        },
    ];

    // Apply events to build projection
    let mut state = SessionSummaryProjection::initial_state();

    for event in &events {
        println!("Applying event: {event:?}");
        state = SessionSummaryProjection::apply_event(&state, event);
    }

    // Query the projection
    println!("\nFinal projection state:");
    for (id, summary) in &state.summaries {
        println!(
            "Session {}: {} requests, {} models used, {}ms total",
            id,
            summary.request_count,
            summary.models_used.len(),
            summary.total_duration_ms
        );
        println!("  Models: {:?}", summary.models_used);
    }

    // Demonstrate query efficiency
    println!("\nQuery efficiency:");
    println!("- Traditional: Read all events, rebuild state, then query");
    println!("- Projection: State already computed, just lookup!");

    // Example query
    if let Some(summary) = state.summaries.get("session-1") {
        println!(
            "\nSession-1 average response time: {}ms",
            summary.total_duration_ms / summary.request_count as u64
        );
    }
}
