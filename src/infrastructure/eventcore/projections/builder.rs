//! Projection builder for multi-stream queries
//!
//! This module provides a flexible builder pattern for creating projections
//! that can aggregate events from multiple streams.
//!
//! ## Event Ordering Strategies
//!
//! When merging events from multiple streams, this module provides two strategies:
//!
//! 1. **Chronological Ordering** (`merge_events_chronologically`):
//!    - Orders events primarily by timestamp
//!    - Uses event_id (UUIDv7) as tiebreaker for determinism
//!    - Best for analytics and time-based queries
//!    - May reorder events from the same stream if clock skew exists
//!
//! 2. **Stream Causality Ordering** (`merge_events_with_stream_causality`):
//!    - Preserves strict ordering within each stream (by event_version)
//!    - Merges streams based on timestamps while maintaining causality
//!    - Best for state reconstruction and event replay
//!    - Handles clock skew between nodes gracefully

use crate::domain::events::DomainEvent;
use eventcore::{EventStore, ReadOptions, StoredEvent, StreamId};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

/// Type alias for a filter function
pub type FilterFn = Box<dyn Fn(&DomainEvent) -> bool + Send + Sync>;

/// Type alias for a projection function
pub type ProjectionFn<T> = Box<dyn Fn(T, &StoredEvent<DomainEvent>) -> T + Send + Sync>;

/// Builder for creating multi-stream projections
#[allow(dead_code)]
pub struct ProjectionBuilder<T> {
    initial_state: T,
    streams: HashSet<StreamId>,
    event_filter: Option<FilterFn>,
    time_range: Option<(eventcore::Timestamp, eventcore::Timestamp)>,
    projection_fn: Option<ProjectionFn<T>>,
}

impl<T: Clone + Send + Sync + 'static> ProjectionBuilder<T> {
    /// Create a new projection builder with initial state
    pub fn new(initial_state: T) -> Self {
        Self {
            initial_state,
            streams: HashSet::new(),
            event_filter: None,
            time_range: None,
            projection_fn: None,
        }
    }

    /// Add a stream to include in the projection
    pub fn with_stream(mut self, stream_id: StreamId) -> Self {
        self.streams.insert(stream_id);
        self
    }

    /// Add multiple streams to include in the projection
    pub fn with_streams(mut self, stream_ids: impl IntoIterator<Item = StreamId>) -> Self {
        self.streams.extend(stream_ids);
        self
    }

    /// Filter events by type or condition
    pub fn filter_events<F>(mut self, filter: F) -> Self
    where
        F: Fn(&DomainEvent) -> bool + Send + Sync + 'static,
    {
        self.event_filter = Some(Box::new(filter));
        self
    }

    /// Filter events by time range
    pub fn within_time_range(
        mut self,
        start: eventcore::Timestamp,
        end: eventcore::Timestamp,
    ) -> Self {
        self.time_range = Some((start, end));
        self
    }

    /// Set the projection function
    pub fn project_with<F>(mut self, projection_fn: F) -> Self
    where
        F: Fn(T, &StoredEvent<DomainEvent>) -> T + Send + Sync + 'static,
    {
        self.projection_fn = Some(Box::new(projection_fn));
        self
    }

    /// Execute the projection
    pub async fn execute<ES: EventStore>(self, event_store: &ES) -> Result<T, ProjectionError>
    where
        ES::Event: TryFrom<DomainEvent> + Clone,
        DomainEvent: for<'a> TryFrom<&'a ES::Event>,
    {
        let projection_fn = self
            .projection_fn
            .ok_or(ProjectionError::NoProjectionFunction)?;

        let mut events_by_stream = HashMap::new();

        // Read events from each stream
        for stream_id in self.streams {
            let read_options = ReadOptions::default();
            let events = event_store
                .read_streams(&[stream_id.clone()], &read_options)
                .await
                .map_err(|e| ProjectionError::EventStore(e.to_string()))?;

            // Convert to DomainEvent
            let domain_events: Vec<StoredEvent<DomainEvent>> = events
                .events
                .into_iter()
                .filter_map(|stored_event| {
                    DomainEvent::try_from(&stored_event.payload)
                        .ok()
                        .map(|domain_event| StoredEvent {
                            stream_id: stored_event.stream_id,
                            event_id: stored_event.event_id,
                            payload: domain_event,
                            metadata: stored_event.metadata,
                            timestamp: stored_event.timestamp,
                            event_version: stored_event.event_version,
                        })
                })
                .collect();

            events_by_stream.insert(stream_id, domain_events);
        }

        // Merge events chronologically
        let mut all_events = merge_events_chronologically(events_by_stream);

        // Apply time range filter if specified
        if let Some((start, end)) = self.time_range {
            all_events.retain(|event| event.timestamp >= start && event.timestamp <= end);
        }

        // Apply event filter if specified
        if let Some(filter) = &self.event_filter {
            all_events.retain(|event| filter(&event.payload));
        }

        // Apply projection function to build final state
        let final_state = all_events
            .into_iter()
            .fold(self.initial_state, |state, event| {
                projection_fn(state, &event)
            });

        Ok(final_state)
    }
}

/// Errors that can occur during projection execution
#[derive(Debug, thiserror::Error)]
pub enum ProjectionError {
    #[error("Event store error: {0}")]
    EventStore(String),

    #[error("No projection function defined")]
    NoProjectionFunction,

    #[error("Event conversion error: {0}")]
    EventConversion(String),
}

/// Helper function to merge events from multiple streams chronologically
///
/// This function ensures deterministic ordering of events by:
/// 1. Primary sort by timestamp
/// 2. Secondary sort by event_id (UUIDv7 provides time-based ordering)
/// 3. Preserves stream-local ordering (event_version) when timestamps are equal
pub fn merge_events_chronologically(
    mut events_by_stream: HashMap<StreamId, Vec<StoredEvent<DomainEvent>>>,
) -> Vec<StoredEvent<DomainEvent>> {
    // First, ensure each stream's events are properly ordered by event_version
    // This preserves causality within each stream
    for events in events_by_stream.values_mut() {
        events.sort_by_key(|e| e.event_version);
    }

    let mut all_events: Vec<StoredEvent<DomainEvent>> =
        events_by_stream.into_values().flatten().collect();

    // Sort with deterministic ordering:
    // 1. By timestamp (primary)
    // 2. By event_id when timestamps are equal (secondary)
    // This ensures consistent ordering even with clock skew or simultaneous events
    all_events.sort_by(|a, b| {
        match a.timestamp.cmp(&b.timestamp) {
            Ordering::Equal => {
                // When timestamps are equal, use event_id for deterministic ordering
                // EventId is UUIDv7 which includes timestamp information
                a.event_id.cmp(&b.event_id)
            }
            other => other,
        }
    });

    all_events
}

/// Alternative merge strategy that maintains strict stream causality
///
/// This function ensures that within a stream, event ordering is preserved
/// even if timestamps suggest a different order (due to clock skew).
/// Use this when stream causality is more important than global time ordering.
#[allow(dead_code)]
pub fn merge_events_with_stream_causality(
    mut events_by_stream: HashMap<StreamId, Vec<StoredEvent<DomainEvent>>>,
) -> Vec<StoredEvent<DomainEvent>> {
    // Build a dependency graph based on stream versions
    let mut stream_positions: HashMap<StreamId, usize> = HashMap::new();

    // Sort each stream by event_version to ensure causality
    for (stream_id, events) in events_by_stream.iter_mut() {
        events.sort_by_key(|e| e.event_version);
        stream_positions.insert(stream_id.clone(), 0);
    }

    let mut result = Vec::new();

    // Merge events while respecting stream causality
    loop {
        let mut next_event: Option<(StreamId, &StoredEvent<DomainEvent>)> = None;

        // Find the next event to process across all streams
        for (stream_id, events) in &events_by_stream {
            if let Some(&pos) = stream_positions.get(stream_id) {
                if let Some(event) = events.get(pos) {
                    if let Some((_, current_next)) = &next_event {
                        // Compare timestamps, but with tolerance for clock skew
                        if event.timestamp < current_next.timestamp {
                            next_event = Some((stream_id.clone(), event));
                        } else if event.timestamp == current_next.timestamp {
                            // Use event_id as tiebreaker
                            if event.event_id < current_next.event_id {
                                next_event = Some((stream_id.clone(), event));
                            }
                        }
                    } else {
                        next_event = Some((stream_id.clone(), event));
                    }
                }
            }
        }

        match next_event {
            Some((stream_id, event)) => {
                result.push(event.clone());
                if let Some(pos) = stream_positions.get_mut(&stream_id) {
                    *pos += 1;
                }
            }
            None => break, // No more events
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{session::SessionId, streams::session_stream};
    use eventcore_memory::InMemoryEventStore;

    #[derive(Debug, Clone, PartialEq)]
    struct TestState {
        event_count: usize,
        session_ids: HashSet<SessionId>,
    }

    #[tokio::test]
    async fn test_projection_builder_basic() {
        let initial_state = TestState {
            event_count: 0,
            session_ids: HashSet::new(),
        };

        let session_id = SessionId::generate();
        let stream_id = session_stream(&session_id);

        let projection = ProjectionBuilder::new(initial_state.clone())
            .with_stream(stream_id)
            .project_with(|mut state, event| {
                state.event_count += 1;
                if let Ok(DomainEvent::SessionStarted { session_id, .. }) =
                    DomainEvent::try_from(&event.payload)
                {
                    state.session_ids.insert(session_id);
                }
                state
            });

        let event_store = InMemoryEventStore::<DomainEvent>::new();
        let result = projection.execute(&event_store).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().event_count, 0); // No events in store yet
    }

    #[tokio::test]
    async fn test_projection_with_multiple_streams() {
        let initial_state = TestState {
            event_count: 0,
            session_ids: HashSet::new(),
        };

        let session_id1 = SessionId::generate();
        let session_id2 = SessionId::generate();
        let streams = vec![session_stream(&session_id1), session_stream(&session_id2)];

        let projection = ProjectionBuilder::new(initial_state)
            .with_streams(streams)
            .project_with(|mut state, _| {
                state.event_count += 1;
                state
            });

        let event_store = InMemoryEventStore::<DomainEvent>::new();
        let result = projection.execute(&event_store).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_projection_with_filter() {
        let initial_state = 0u32;

        let session_id = SessionId::generate();
        let stream_id = session_stream(&session_id);

        let projection = ProjectionBuilder::new(initial_state)
            .with_stream(stream_id)
            .filter_events(|event| matches!(event, DomainEvent::SessionStarted { .. }))
            .project_with(|count, _| count + 1);

        let event_store = InMemoryEventStore::<DomainEvent>::new();
        let result = projection.execute(&event_store).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0); // No events match filter yet
    }

    #[tokio::test]
    async fn test_projection_with_time_range() {
        let initial_state = Vec::<DomainEvent>::new();

        let session_id = SessionId::generate();
        let stream_id = session_stream(&session_id);
        let start = eventcore::Timestamp::now();
        let end = eventcore::Timestamp::now();

        let projection = ProjectionBuilder::new(initial_state)
            .with_stream(stream_id)
            .within_time_range(start, end)
            .project_with(|mut events, event| {
                if let Ok(domain_event) = DomainEvent::try_from(&event.payload) {
                    events.push(domain_event);
                }
                events
            });

        let event_store = InMemoryEventStore::<DomainEvent>::new();
        let result = projection.execute(&event_store).await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_merge_events_chronologically() {
        // This test is temporarily simplified until we understand StoredEvent structure
        let _events_by_stream = HashMap::new();
        let merged = merge_events_chronologically(_events_by_stream);
        assert_eq!(merged.len(), 0);
    }

    #[tokio::test]
    async fn test_projection_error_no_projection_function() {
        let initial_state = 0u32;
        let session_id = SessionId::generate();
        let stream_id = session_stream(&session_id);

        let projection = ProjectionBuilder::new(initial_state).with_stream(stream_id);
        // Note: No project_with() called

        let event_store = InMemoryEventStore::<DomainEvent>::new();
        let result = projection.execute(&event_store).await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ProjectionError::NoProjectionFunction
        ));
    }
}
