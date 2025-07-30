# Stream Lifecycle Patterns in Event-Sourced Systems

This document describes stream lifecycle patterns, relationships, and best practices for managing event streams in Union Square's event-sourced architecture using EventCore.

## Table of Contents

1. [Stream Lifecycle Overview](#stream-lifecycle-overview)
2. [Stream Naming Conventions](#stream-naming-conventions)
3. [Stream Creation Patterns](#stream-creation-patterns)
4. [Stream Evolution](#stream-evolution)
5. [Stream Relationships](#stream-relationships)
6. [Stream Closure and Archival](#stream-closure-and-archival)
7. [Retention Policies](#retention-policies)
8. [Best Practices](#best-practices)
9. [Common Patterns](#common-patterns)

## Stream Lifecycle Overview

In EventCore's architecture, streams represent consistency boundaries that can span multiple traditional aggregates. Each stream has a distinct lifecycle:

```
Creation → Active Evolution → Completion → Archival → Deletion
```

### Key Principles

1. **Streams are append-only**: Once created, events can only be added, never modified or deleted
2. **Streams have semantic meaning**: Each stream represents a business concept or process
3. **Streams can be correlated**: Multiple streams often work together to represent complex workflows
4. **Streams have defined lifecycles**: From creation to archival, each phase has specific patterns

## Stream Naming Conventions

Union Square follows these naming patterns to ensure consistency and discoverability:

```rust
// Session lifecycle streams
session:{session_id}         // All events for a session
session:{session_id}:metrics // Derived metrics for a session

// Analysis process streams
analysis:{analysis_id}       // Analysis workflow events
analysis:{analysis_id}:results // Analysis results

// User-scoped streams
user:{user_id}:settings     // User preferences and configuration
user:{user_id}:activity     // User activity log

// Request tracking streams
request:{request_id}        // Individual request lifecycle

// Extraction process streams
extraction:{extraction_id}  // Test case extraction events
```

### Naming Rules

1. Use lowercase with colons as separators
2. Start with the aggregate type
3. Include the unique identifier
4. Add qualifiers for sub-streams (e.g., `:settings`, `:metrics`)
5. Keep names descriptive but concise

## Stream Creation Patterns

### 1. Direct Creation

Streams are created implicitly when the first event is written:

```rust
// Stream created when session starts
emit!(events, &read_streams, session_stream(&session_id),
    DomainEvent::SessionStarted {
        session_id,
        user_id,
        application_id,
        started_at: Timestamp::now(),
    }
);
```

### 2. Derived Stream Creation

Some streams are created as a result of events in other streams:

```rust
// When analysis is requested, create analysis stream
match event {
    DomainEvent::AnalysisRequested { session_id, .. } => {
        let analysis_id = AnalysisId::generate();
        let analysis_stream = analysis_stream(&analysis_id);

        emit!(events, &read_streams, analysis_stream,
            DomainEvent::AnalysisCreated {
                analysis_id,
                session_id,
                created_at: Timestamp::now(),
            }
        );
    }
}
```

### 3. Correlation Stream Creation

Create streams that correlate multiple entities:

```rust
// Create a correlation stream for multi-session analysis
let correlation_id = CorrelationId::generate();
let correlation_stream = StreamId::try_new(
    format!("correlation:{}:sessions", correlation_id)
)?;

for session_id in session_ids {
    emit!(events, &read_streams, correlation_stream.clone(),
        DomainEvent::SessionCorrelated {
            correlation_id,
            session_id,
            correlated_at: Timestamp::now(),
        }
    );
}
```

## Stream Evolution

Streams evolve through their lifecycle based on business events:

### 1. State Transitions

```rust
// Request stream lifecycle
enum RequestLifecycle {
    NotStarted,
    Received { request_id, received_at },
    Forwarded { request_id, received_at, forwarded_at },
    ResponseReceived { request_id, received_at, forwarded_at, response_at },
    Completed { request_id, received_at, forwarded_at, response_at, completed_at },
    Failed { request_id, failed_at, reason },
}
```

### 2. Event Patterns by Lifecycle Phase

#### Initialization Phase
```rust
// First events establish stream context
SessionStarted → establishes session stream
UserCreated → establishes user stream
AnalysisCreated → establishes analysis stream
```

#### Active Phase
```rust
// Business operations
LlmRequestReceived → RequestStarted → ResponseReceived
SessionTagged → adds metadata
MetricsCalculated → updates analytics
```

#### Completion Phase
```rust
// Terminal events
SessionEnded → marks session complete
AnalysisCompleted → analysis finished
ExtractionFinished → extraction done
```

### 3. Stream Forking

Sometimes streams spawn child streams:

```rust
// Parent session stream spawns analysis streams
impl CommandLogic for RequestAnalysis {
    async fn handle(&self, read_streams: ReadStreams<Self::StreamSet>, state: Self::State, _: &mut StreamResolver) -> CommandResult<Vec<StreamWrite<Self::StreamSet, Self::Event>>> {
        let mut events = Vec::new();

        // Write to parent session stream
        emit!(events, &read_streams, self.session_stream.clone(),
            DomainEvent::AnalysisRequested {
                session_id: self.session_id,
                requested_at: Timestamp::now(),
            }
        );

        // Create child analysis stream
        let analysis_id = AnalysisId::generate();
        emit!(events, &read_streams, analysis_stream(&analysis_id),
            DomainEvent::AnalysisStarted {
                analysis_id,
                session_id: self.session_id,
                started_at: Timestamp::now(),
            }
        );

        Ok(events)
    }
}
```

## Stream Relationships

### 1. Parent-Child Relationships

```
session:{id}
    ├── request:{request_id_1}
    ├── request:{request_id_2}
    └── analysis:{analysis_id}
            └── extraction:{extraction_id}
```

### 2. Cross-Stream References

Events can reference other streams:

```rust
DomainEvent::AnalysisCompleted {
    analysis_id: AnalysisId,
    session_id: SessionId,  // References parent session
    results: AnalysisResults,
    completed_at: Timestamp,
}
```

### 3. Correlation Patterns

#### Saga Pattern
Multiple streams participate in a distributed workflow:

```rust
// Order processing saga
order:{order_id} → OrderPlaced
inventory:{item_id} → ItemReserved
payment:{payment_id} → PaymentProcessed
order:{order_id} → OrderConfirmed
```

#### Aggregation Pattern
Multiple streams contribute to a summary:

```rust
// F-score calculation across sessions
session:{id1} → MetricsRecorded
session:{id2} → MetricsRecorded
application:{app_id}:metrics → AggregatedMetricsCalculated
```

### 4. Stream Dependencies

Some streams depend on others:

```rust
// Analysis stream depends on session stream
impl CommandLogic for StartAnalysis {
    async fn handle(&self, read_streams: ReadStreams<Self::StreamSet>, state: Self::State, _: &mut StreamResolver) -> CommandResult<Vec<StreamWrite<Self::StreamSet, Self::Event>>> {
        // Read session stream to verify session exists and is active
        let session_state = read_session_state(&read_streams, &self.session_id).await?;

        require!(session_state.is_active(), "Cannot analyze inactive session");

        // Continue with analysis...
    }
}
```

## Stream Closure and Archival

### 1. Explicit Closure

Some streams have explicit termination events:

```rust
DomainEvent::SessionEnded {
    session_id: SessionId,
    ended_at: Timestamp,
    final_status: SessionStatus,
}
```

### 2. Implicit Closure

Streams may be considered closed after inactivity:

```rust
// Configuration for implicit closure
const SESSION_INACTIVITY_THRESHOLD: Duration = Duration::hours(24);
const ANALYSIS_TIMEOUT: Duration = Duration::hours(1);
```

### 3. Closure Validation

Ensure streams are properly closed:

```rust
impl SessionState {
    pub fn can_close(&self) -> bool {
        // All requests must be completed
        self.active_requests.is_empty() &&
        // No pending analysis
        self.pending_analyses.is_empty() &&
        // No active extractions
        self.active_extractions.is_empty()
    }
}
```

### 4. Post-Closure Operations

After closure, streams may still receive certain events:

```rust
// Audit events can be added after closure
DomainEvent::SessionAudited {
    session_id: SessionId,
    audit_type: AuditType,
    audited_at: Timestamp,
}

// Compliance events
DomainEvent::SessionRedacted {
    session_id: SessionId,
    reason: RedactionReason,
    redacted_at: Timestamp,
}
```

## Retention Policies

### 1. Time-Based Retention

```rust
pub struct RetentionPolicy {
    // Active streams - full retention
    active_retention: Duration,

    // Archived streams - compressed/summarized
    archive_retention: Duration,

    // Compliance hold - extended retention
    compliance_retention: Option<Duration>,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            active_retention: Duration::days(90),
            archive_retention: Duration::days(365 * 2), // 2 years
            compliance_retention: Some(Duration::days(365 * 7)), // 7 years
        }
    }
}
```

### 2. Event-Based Retention

Different events may have different retention:

```rust
impl DomainEvent {
    pub fn retention_category(&self) -> RetentionCategory {
        match self {
            // PII events - shorter retention
            DomainEvent::UserCreated { .. } => RetentionCategory::PersonalData,

            // Audit events - extended retention
            DomainEvent::SessionAudited { .. } => RetentionCategory::Audit,

            // Metrics - standard retention
            DomainEvent::FScoreCalculated { .. } => RetentionCategory::Analytics,

            // Technical events - short retention
            DomainEvent::LlmRequestStarted { .. } => RetentionCategory::Operational,

            _ => RetentionCategory::Standard,
        }
    }
}
```

### 3. Archival Strategies

#### Snapshot + Events Pattern
```rust
// Create snapshot at closure
DomainEvent::SessionSnapshot {
    session_id: SessionId,
    state: SessionSummary,
    event_count: usize,
    created_at: Timestamp,
}

// Archive detailed events separately
pub async fn archive_session(session_id: &SessionId) -> Result<(), ArchivalError> {
    // 1. Create snapshot
    let snapshot = create_session_snapshot(session_id).await?;

    // 2. Move events to cold storage
    let events = read_session_events(session_id).await?;
    cold_storage.store_compressed(session_id, events).await?;

    // 3. Keep only snapshot in hot storage
    hot_storage.replace_with_snapshot(session_id, snapshot).await?;

    Ok(())
}
```

#### Hierarchical Archival
```rust
// Archive child streams when parent is archived
pub async fn archive_session_hierarchy(session_id: &SessionId) -> Result<(), ArchivalError> {
    // Archive all request streams
    for request_id in get_session_requests(session_id).await? {
        archive_request_stream(&request_id).await?;
    }

    // Archive analysis streams
    for analysis_id in get_session_analyses(session_id).await? {
        archive_analysis_stream(&analysis_id).await?;
    }

    // Finally archive the session stream
    archive_session_stream(session_id).await?;

    Ok(())
}
```

## Best Practices

### 1. Stream Design Principles

#### Single Writer Principle
Each stream should have a clear owner:

```rust
// Good: Session service owns session streams
pub struct SessionService {
    event_store: Arc<dyn EventStore>,
}

impl SessionService {
    pub async fn start_session(&self, user_id: UserId, app_id: ApplicationId) -> Result<SessionId, Error> {
        let session_id = SessionId::generate();
        let stream_id = session_stream(&session_id);

        // Only SessionService writes SessionStarted events
        self.event_store.append(stream_id, vec![
            DomainEvent::SessionStarted { session_id, user_id, app_id, started_at: Timestamp::now() }
        ]).await?;

        Ok(session_id)
    }
}
```

#### Bounded Context Alignment
Streams should align with domain boundaries:

```rust
// Each bounded context has its own stream patterns
mod session_context {
    pub fn session_stream(id: &SessionId) -> StreamId { /* ... */ }
    pub fn request_stream(id: &RequestId) -> StreamId { /* ... */ }
}

mod analytics_context {
    pub fn analysis_stream(id: &AnalysisId) -> StreamId { /* ... */ }
    pub fn metrics_stream(id: &MetricsId) -> StreamId { /* ... */ }
}

mod user_context {
    pub fn user_stream(id: &UserId) -> StreamId { /* ... */ }
    pub fn settings_stream(id: &UserId) -> StreamId { /* ... */ }
}
```

### 2. Stream Granularity

#### Fine-Grained Streams
Better for:
- High-concurrency scenarios
- Independent lifecycles
- Selective reading

```rust
// Fine-grained: Separate stream per request
request:{request_id_1}
request:{request_id_2}
request:{request_id_3}
```

#### Coarse-Grained Streams
Better for:
- Related events that are always read together
- Maintaining order across related events
- Simpler consistency boundaries

```rust
// Coarse-grained: All requests in session stream
session:{session_id} contains all request events
```

### 3. Stream Metadata

Track stream metadata for lifecycle management:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamMetadata {
    pub stream_id: StreamId,
    pub stream_type: StreamType,
    pub created_at: Timestamp,
    pub last_event_at: Timestamp,
    pub event_count: usize,
    pub status: StreamStatus,
    pub parent_stream: Option<StreamId>,
    pub retention_policy: RetentionPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StreamStatus {
    Active,
    Closing,
    Closed,
    Archived,
    Deleted,
}
```

### 4. Stream Discovery

Implement patterns for finding related streams:

```rust
// Stream index for discovery
pub trait StreamIndex {
    async fn find_by_session(&self, session_id: &SessionId) -> Result<Vec<StreamId>, Error>;
    async fn find_by_user(&self, user_id: &UserId) -> Result<Vec<StreamId>, Error>;
    async fn find_by_time_range(&self, start: Timestamp, end: Timestamp) -> Result<Vec<StreamId>, Error>;
    async fn find_children(&self, parent_stream: &StreamId) -> Result<Vec<StreamId>, Error>;
}
```

## Common Patterns

### 1. Stream Versioning

Handle schema evolution:

```rust
// Version in stream name
let stream_v1 = StreamId::try_new(format!("session:v1:{}", session_id))?;
let stream_v2 = StreamId::try_new(format!("session:v2:{}", session_id))?;

// Migration event
DomainEvent::StreamMigrated {
    from_stream: stream_v1,
    to_stream: stream_v2,
    migrated_at: Timestamp::now(),
}
```

### 2. Stream Compaction

Reduce stream size while preserving history:

```rust
pub async fn compact_stream(stream_id: &StreamId) -> Result<(), CompactionError> {
    let events = read_all_events(stream_id).await?;

    // Create checkpoint
    let checkpoint = create_checkpoint(&events)?;

    // Keep only significant events + checkpoint
    let compacted_events = events.into_iter()
        .filter(|e| e.is_significant())
        .collect::<Vec<_>>();

    // Write to new compacted stream
    let compacted_stream = StreamId::try_new(format!("{}.compacted", stream_id.as_ref()))?;
    write_events(compacted_stream, vec![checkpoint]).await?;
    write_events(compacted_stream, compacted_events).await?;

    Ok(())
}
```

### 3. Stream Replay

Support replaying streams for different purposes:

```rust
pub async fn replay_stream<F>(
    stream_id: &StreamId,
    from: Option<Timestamp>,
    until: Option<Timestamp>,
    mut handler: F,
) -> Result<(), ReplayError>
where
    F: FnMut(&DomainEvent) -> Result<(), ReplayError>,
{
    let events = read_events_range(stream_id, from, until).await?;

    for event in events {
        handler(&event.payload)?;
    }

    Ok(())
}

// Usage: Rebuild read model
replay_stream(&session_stream, None, None, |event| {
    match event {
        DomainEvent::SessionStarted { .. } => update_session_count(),
        DomainEvent::LlmRequestReceived { .. } => update_request_metrics(),
        _ => Ok(()),
    }
}).await?;
```

### 4. Stream Monitoring

Track stream health and patterns:

```rust
#[derive(Debug, Clone)]
pub struct StreamHealth {
    pub stream_id: StreamId,
    pub event_rate: f64,  // events per second
    pub last_event_age: Duration,
    pub error_count: usize,
    pub size_bytes: usize,
    pub projected_closure: Option<Timestamp>,
}

pub async fn monitor_stream_health(stream_id: &StreamId) -> Result<StreamHealth, Error> {
    let metadata = get_stream_metadata(stream_id).await?;
    let recent_events = get_recent_event_count(stream_id, Duration::minutes(5)).await?;

    Ok(StreamHealth {
        stream_id: stream_id.clone(),
        event_rate: recent_events as f64 / 300.0, // 5 minutes in seconds
        last_event_age: Timestamp::now().duration_since(metadata.last_event_at),
        error_count: count_error_events(stream_id).await?,
        size_bytes: get_stream_size(stream_id).await?,
        projected_closure: estimate_closure_time(stream_id).await?,
    })
}
```

## Conclusion

Effective stream lifecycle management is crucial for maintaining a healthy event-sourced system. By following these patterns and best practices, you can ensure that your streams remain manageable, discoverable, and aligned with your business requirements throughout their lifecycle.

Key takeaways:
1. Design streams around business concepts and consistency boundaries
2. Use consistent naming conventions for discoverability
3. Plan for the entire lifecycle from creation to archival
4. Implement appropriate retention policies for compliance and performance
5. Monitor stream health and growth patterns
6. Design for stream evolution and schema changes

Remember that streams are the primary organizing principle in an event-sourced system—invest time in getting their design right, as changes become more difficult as the system grows.
