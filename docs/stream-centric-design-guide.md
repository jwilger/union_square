# Stream-Centric Design Guide for Union Square

This guide provides practical guidance for developers working with Union Square's stream-centric event-sourced architecture using EventCore.

## Table of Contents

1. [Introduction](#introduction)
2. [Core Concepts](#core-concepts)
3. [Stream Design Principles](#stream-design-principles)
4. [Implementation Patterns](#implementation-patterns)
5. [Query Patterns](#query-patterns)
6. [Testing Strategies](#testing-strategies)
7. [Best Practices](#best-practices)
8. [Common Pitfalls](#common-pitfalls)

## Introduction

Union Square uses a stream-centric approach to event sourcing, where streams represent consistency boundaries that can span multiple traditional aggregates. This guide will help you understand how to design, implement, and work with streams effectively.

### Why Stream-Centric?

Traditional aggregate-centric event sourcing often leads to:
- Artificial boundaries that don't match business workflows
- Complex coordination between aggregates
- Difficulty expressing cross-aggregate constraints

Stream-centric design allows:
- Natural modeling of business workflows
- Atomic operations across multiple entities
- Flexible consistency boundaries

## Core Concepts

### Streams as Consistency Boundaries

A stream is a sequence of events that form a consistency boundary. Unlike traditional aggregates, streams can:

1. **Span multiple entities**: A session stream contains events for the session itself, requests, responses, and metrics
2. **Have dynamic boundaries**: Commands determine which streams to include based on runtime data
3. **Support atomic multi-stream operations**: EventCore ensures all-or-nothing writes across streams

### Stream Types in Union Square

```rust
// Primary streams
session:{session_id}         // Session lifecycle and requests
analysis:{analysis_id}       // Analysis processes
user:{user_id}:settings     // User preferences
extraction:{extraction_id}   // Test case extraction

// Derived streams
session:{session_id}:metrics // Aggregated metrics
user:{user_id}:activity     // Activity summaries
```

### Command-Driven Stream Selection

Commands determine which streams participate in an operation:

```rust
#[derive(Command)]
struct ProcessPayment {
    #[stream]
    order_stream: OrderStreamId,      // Primary stream
    #[stream]
    payment_stream: PaymentStreamId,  // Secondary stream
    #[stream]
    inventory_stream: InventoryStreamId, // Optional stream

    amount: Money,
}
```

## Stream Design Principles

### 1. Design Around Business Workflows

Streams should represent natural business boundaries:

**Good**: Session stream contains all session-related events
```rust
session:{id} contains:
  - SessionStarted
  - LlmRequestReceived
  - LlmResponseReceived
  - FScoreCalculated
  - SessionEnded
```

**Bad**: Separate streams for each request type
```rust
session-starts:{id}
session-requests:{id}
session-responses:{id}
session-metrics:{id}
```

### 2. Use Semantic Stream Names

Stream names should be:
- **Descriptive**: Clear purpose from the name
- **Hierarchical**: Show relationships (e.g., `user:{id}:settings`)
- **Consistent**: Follow established patterns
- **Discoverable**: Easy to find related streams

### 3. Keep Streams Cohesive

Events in a stream should:
- Relate to the same business concept
- Have similar lifecycles
- Be read together frequently

### 4. Plan for Stream Evolution

Consider future needs:
- Event schema evolution
- Stream migration patterns
- Archival strategies

## Implementation Patterns

### Creating Stream Identifier Types

Use newtypes for type-safe stream identifiers:

```rust
use nutype::nutype;
use eventcore::StreamId;

#[nutype(
    derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize),
    validate(regex = r"^session:[0-9a-f-]+$")
)]
pub struct SessionStreamId(String);

impl AsRef<StreamId> for SessionStreamId {
    fn as_ref(&self) -> &StreamId {
        unsafe { std::mem::transmute(self.as_ref()) }
    }
}

// Factory function for creating stream IDs
pub fn session_stream(session_id: &SessionId) -> SessionStreamId {
    SessionStreamId::new(format!("session:{}", session_id))
        .expect("Session ID format is always valid")
}
```

### Implementing Commands

Follow the command pattern for stream operations:

```rust
#[derive(Command, Clone, Debug, Serialize, Deserialize)]
struct StartSession {
    #[stream]
    session_stream: SessionStreamId,

    user_id: UserId,
    application_id: ApplicationId,
}

#[derive(Default)]
struct SessionState {
    started: bool,
    user_id: Option<UserId>,
    request_count: usize,
}

#[async_trait]
impl CommandLogic for StartSession {
    type State = SessionState;
    type Event = DomainEvent;

    fn apply(&self, state: &mut Self::State, event: &StoredEvent<Self::Event>) {
        match &event.payload {
            DomainEvent::SessionStarted { user_id, .. } => {
                state.started = true;
                state.user_id = Some(user_id.clone());
            }
            DomainEvent::LlmRequestReceived { .. } => {
                state.request_count += 1;
            }
            _ => {}
        }
    }

    async fn handle(
        &self,
        read_streams: ReadStreams<Self::StreamSet>,
        state: Self::State,
        _: &mut StreamResolver,
    ) -> CommandResult<Vec<StreamWrite<Self::StreamSet, Self::Event>>> {
        let mut events = Vec::new();

        // Business rule validation
        require!(!state.started, "Session already started");

        // Emit event
        emit!(
            events,
            &read_streams,
            self.session_stream.clone(),
            DomainEvent::SessionStarted {
                session_id: extract_session_id(&self.session_stream),
                user_id: self.user_id.clone(),
                application_id: self.application_id.clone(),
                started_at: Timestamp::now(),
            }
        );

        Ok(events)
    }
}
```

### Multi-Stream Commands

For operations spanning multiple streams:

```rust
#[derive(Command)]
struct TransferFunds {
    #[stream]
    source_account: AccountStreamId,
    #[stream]
    target_account: AccountStreamId,
    #[stream]
    transaction_log: TransactionStreamId,

    amount: Money,
    reference: TransferReference,
}

#[derive(Default)]
struct TransferState {
    source_balance: Money,
    target_exists: bool,
    duplicate_reference: bool,
}

impl CommandLogic for TransferFunds {
    // ... apply implementation ...

    async fn handle(
        &self,
        read_streams: ReadStreams<Self::StreamSet>,
        state: Self::State,
        _: &mut StreamResolver,
    ) -> CommandResult<Vec<StreamWrite<Self::StreamSet, Self::Event>>> {
        let mut events = Vec::new();

        // Business rules across streams
        require!(state.source_balance >= self.amount, "Insufficient funds");
        require!(state.target_exists, "Target account not found");
        require!(!state.duplicate_reference, "Duplicate transfer reference");

        // Atomic writes to all streams
        emit!(events, &read_streams, self.source_account.clone(),
            AccountEvent::Debited { amount: self.amount, reference: self.reference.clone() }
        );

        emit!(events, &read_streams, self.target_account.clone(),
            AccountEvent::Credited { amount: self.amount, reference: self.reference.clone() }
        );

        emit!(events, &read_streams, self.transaction_log.clone(),
            TransactionEvent::Recorded {
                from: self.source_account.clone(),
                to: self.target_account.clone(),
                amount: self.amount,
                reference: self.reference.clone(),
                timestamp: Timestamp::now(),
            }
        );

        Ok(events)
    }
}
```

## Query Patterns

### Using the Projection Builder

The projection builder provides a flexible way to query across streams:

```rust
use union_square::infrastructure::eventcore::projections::builder::ProjectionBuilder;

// Single stream projection
let session_summary = ProjectionBuilder::new(SessionSummary::default())
    .with_stream(session_stream(&session_id))
    .project_with(|mut summary, event| {
        match &event.payload {
            DomainEvent::SessionStarted { user_id, .. } => {
                summary.user_id = Some(user_id.clone());
            }
            DomainEvent::LlmRequestReceived { .. } => {
                summary.request_count += 1;
            }
            _ => {}
        }
        summary
    })
    .execute(&event_store)
    .await?;
```

### Multi-Stream Projections

Aggregate data from multiple streams:

```rust
// Get all sessions for a user
let user_sessions = get_user_sessions(&event_store, &user_id).await?;
let session_streams: Vec<_> = user_sessions
    .iter()
    .map(session_stream)
    .collect();

// Build cross-session metrics
let metrics = ProjectionBuilder::new(UserMetrics::default())
    .with_streams(session_streams)
    .filter_events(|event| {
        matches!(event,
            DomainEvent::LlmRequestReceived { .. } |
            DomainEvent::FScoreCalculated { .. }
        )
    })
    .project_with(|mut metrics, event| {
        match &event.payload {
            DomainEvent::LlmRequestReceived { model_version, .. } => {
                metrics.increment_model_usage(model_version);
            }
            DomainEvent::FScoreCalculated { f_score, .. } => {
                metrics.record_performance(*f_score);
            }
            _ => {}
        }
        metrics
    })
    .execute(&event_store)
    .await?;
```

### Time-Based Queries

Filter events by time range:

```rust
let last_hour = Timestamp::now() - Duration::hours(1);
let now = Timestamp::now();

let recent_activity = ProjectionBuilder::new(ActivitySummary::default())
    .with_stream(session_stream(&session_id))
    .within_time_range(last_hour, now)
    .project_with(|mut summary, event| {
        summary.event_count += 1;
        summary.last_event_time = Some(event.timestamp);
        summary
    })
    .execute(&event_store)
    .await?;
```

## Testing Strategies

### Unit Testing Commands

Test commands with in-memory event store:

```rust
#[tokio::test]
async fn test_start_session_command() {
    let event_store = InMemoryEventStore::new();
    let executor = CommandExecutor::new(event_store.clone());

    let session_id = SessionId::generate();
    let command = StartSession {
        session_stream: session_stream(&session_id),
        user_id: UserId::generate(),
        application_id: ApplicationId::try_new("test-app").unwrap(),
    };

    // Execute command
    let result = executor.execute(Box::new(command)).await;
    assert!(result.is_ok());

    // Verify events
    let events = get_session_events(&event_store, &session_id).await.unwrap();
    assert_eq!(events.len(), 1);
    assert!(matches!(events[0], DomainEvent::SessionStarted { .. }));
}
```

### Testing Multi-Stream Operations

```rust
#[tokio::test]
async fn test_multi_stream_analysis() {
    let event_store = InMemoryEventStore::new();
    let executor = CommandExecutor::new(event_store.clone());

    // Set up initial state
    let session_id = SessionId::generate();
    let start_session = StartSession { /* ... */ };
    executor.execute(Box::new(start_session)).await.unwrap();

    // Add some requests
    for _ in 0..5 {
        let receive_request = ReceiveLlmRequest { /* ... */ };
        executor.execute(Box::new(receive_request)).await.unwrap();
    }

    // Start analysis
    let analysis_id = AnalysisId::generate();
    let start_analysis = StartSessionAnalysis {
        session_stream: session_stream(&session_id),
        analysis_stream: analysis_stream(&analysis_id),
        reason: AnalysisReason::PerformanceReview,
    };

    let result = executor.execute(Box::new(start_analysis)).await;
    assert!(result.is_ok());

    // Verify state across streams
    let session_events = get_session_events(&event_store, &session_id).await.unwrap();
    let analysis_events = get_analysis_events(&event_store, &analysis_id).await.unwrap();

    assert!(session_events.iter().any(|e| matches!(e, DomainEvent::AnalysisRequested { .. })));
    assert!(analysis_events.iter().any(|e| matches!(e, DomainEvent::AnalysisStarted { .. })));
}
```

### Property-Based Testing

Use property-based testing for invariants:

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn session_state_invariants(
        requests in prop::collection::vec(any::<RequestData>(), 0..100)
    ) {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            let event_store = InMemoryEventStore::new();
            let executor = CommandExecutor::new(event_store.clone());

            // Start session
            let session_id = SessionId::generate();
            let start = StartSession { /* ... */ };
            executor.execute(Box::new(start)).await.unwrap();

            // Process requests
            for request_data in requests {
                let command = ReceiveLlmRequest {
                    session_stream: session_stream(&session_id),
                    request_data,
                };
                let _ = executor.execute(Box::new(command)).await;
            }

            // Verify invariants
            let state = build_session_state(&event_store, &session_id).await;
            prop_assert!(state.request_count >= state.successful_responses);
            prop_assert!(state.started_at <= state.last_activity);
        });
    }
}
```

## Best Practices

### 1. Use Type-Safe Stream IDs

Always use newtype wrappers for stream IDs:

```rust
// Good
pub fn session_stream(id: &SessionId) -> SessionStreamId { /* ... */ }
pub fn analysis_stream(id: &AnalysisId) -> AnalysisStreamId { /* ... */ }

// Bad
pub fn session_stream(id: &str) -> StreamId { /* ... */ }
```

### 2. Keep Commands Focused

Each command should:
- Have a single responsibility
- Validate one set of business rules
- Emit related events atomically

### 3. Design for Idempotency

Make commands idempotent where possible:

```rust
async fn handle(...) -> CommandResult<...> {
    // Check if operation already completed
    if state.operation_completed {
        return Ok(vec![]); // No-op
    }

    // Proceed with operation
    // ...
}
```

### 4. Use Meaningful Event Names

Events should be:
- Past tense (something that happened)
- Business-focused (not technical)
- Self-documenting

```rust
// Good
SessionStarted
PaymentProcessed
AnalysisCompleted

// Bad
SessionCreate
ProcessPayment
FinishAnalysis
```

### 5. Plan for Event Evolution

Design events for future compatibility:

```rust
#[derive(Serialize, Deserialize)]
#[serde(tag = "version")]
enum VersionedEvent {
    #[serde(rename = "1")]
    V1(DomainEventV1),

    #[serde(rename = "2")]
    V2(DomainEventV2),
}
```

## Common Pitfalls

### 1. Over-Granular Streams

**Problem**: Creating too many fine-grained streams
```rust
// Too granular
request:{request_id}
response:{request_id}
metrics:{request_id}
```

**Solution**: Group related events
```rust
// Better
session:{session_id} // Contains requests, responses, and metrics
```

### 2. Ignoring Read Patterns

**Problem**: Designing streams without considering queries
**Solution**: Design streams around both write and read patterns

### 3. Mixing Technical and Business Events

**Problem**: Including infrastructure events in domain streams
```rust
// Bad
DomainEvent::DatabaseConnectionLost
DomainEvent::CacheInvalidated
```

**Solution**: Keep technical events separate from business events

### 4. Forgetting About Event Ordering

**Problem**: Assuming events are always processed in order
**Solution**: Design for eventual consistency and handle out-of-order events

### 5. Not Planning for Archive

**Problem**: Unbounded stream growth
**Solution**: Design archival strategies from the start

## Conclusion

Stream-centric design in Union Square provides a powerful way to model complex business workflows while maintaining consistency and performance. By following these patterns and practices, you can build robust, scalable event-sourced systems that accurately reflect your domain.

Key takeaways:
1. Streams represent consistency boundaries, not just aggregates
2. Commands determine stream participation dynamically
3. Design streams around business workflows
4. Use projections for flexible querying
5. Test thoroughly with both unit and integration tests
6. Plan for evolution and archival from the start

For more examples, see:
- `/examples/multi_stream_queries.rs` - Query pattern examples
- `/src/domain/commands_test.rs` - Command implementation examples
- `/docs/eventcore-patterns.md` - EventCore pattern reference
