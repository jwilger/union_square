# EventCore Command Patterns

This document demonstrates key EventCore patterns for implementing event-sourced commands in Union Square.

## Table of Contents

1. [Basic Command Structure](#basic-command-structure)
2. [Single-Stream Commands](#single-stream-commands)
3. [Multi-Stream Commands](#multi-stream-commands)
4. [State Management](#state-management)
5. [Business Rule Validation](#business-rule-validation)
6. [Error Handling](#error-handling)
7. [Testing Commands](#testing-commands)

## Basic Command Structure

Every EventCore command follows this pattern:

```rust
#[derive(Command, Clone, Debug, Serialize, Deserialize)]
struct MyCommand {
    // Stream fields must be annotated with #[stream]
    #[stream]
    primary_stream: MyStreamId,

    // Regular command data fields
    some_data: String,
    other_data: u64,
}
```

The `#[derive(Command)]` macro automatically generates:
- A phantom type for stream access control
- Implementation of the `CommandStreams` trait
- Type associations required by EventCore

## Single-Stream Commands

The simplest pattern involves a single stream:

```rust
#[derive(Command)]
struct StartSession {
    #[stream]
    session_stream: SessionStreamId,

    user_id: UserId,
    application_id: ApplicationId,
}

#[async_trait]
impl CommandLogic for StartSession {
    type State = SessionState;
    type Event = DomainEvent;

    fn apply(&self, state: &mut Self::State, event: &StoredEvent<Self::Event>) {
        // Update state based on events
        match &event.payload {
            DomainEvent::SessionStarted { .. } => {
                state.started = true;
            }
            _ => {}
        }
    }

    async fn handle(
        &self,
        read_streams: ReadStreams<Self::StreamSet>,
        state: Self::State,
        _stream_resolver: &mut StreamResolver,
    ) -> CommandResult<Vec<StreamWrite<Self::StreamSet, Self::Event>>> {
        let mut events = Vec::new();

        // Business rule validation
        require!(!state.started, "Session already started");

        // Emit events
        emit!(
            events,
            &read_streams,
            self.session_stream.clone(),
            DomainEvent::SessionStarted {
                session_id: SessionId::from(self.session_stream.clone()),
                user_id: self.user_id.clone(),
                application_id: self.application_id.clone(),
                started_at: Timestamp::now(),
            }
        );

        Ok(events)
    }
}
```

## Multi-Stream Commands

The power of EventCore shines with multi-stream atomic operations:

```rust
#[derive(Command)]
struct StartSessionAnalysis {
    #[stream]
    session_stream: SessionStreamId,
    #[stream]
    analysis_stream: AnalysisStreamId,

    reason: AnalysisReason,
}

#[async_trait]
impl CommandLogic for StartSessionAnalysis {
    type State = CombinedState;
    type Event = AnalysisDomainEvent;

    fn apply(&self, state: &mut Self::State, event: &StoredEvent<Self::Event>) {
        // Handle events from different streams
        match &event.stream_id {
            stream_id if stream_id == self.session_stream.as_ref() => {
                // Update session-related state
                state.session_exists = true;
            }
            stream_id if stream_id == self.analysis_stream.as_ref() => {
                // Update analysis-related state
                match &event.payload {
                    AnalysisDomainEvent::Analysis(AnalysisEvent::AnalysisStarted { .. }) => {
                        state.analysis_active = true;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    async fn handle(
        &self,
        read_streams: ReadStreams<Self::StreamSet>,
        state: Self::State,
        _stream_resolver: &mut StreamResolver,
    ) -> CommandResult<Vec<StreamWrite<Self::StreamSet, Self::Event>>> {
        let mut events = Vec::new();

        // Validate business rules across streams
        require!(state.session_exists, "Session not found");
        require!(!state.analysis_active, "Analysis already active");

        // Emit events to multiple streams atomically
        emit!(
            events,
            &read_streams,
            self.analysis_stream.clone(),
            AnalysisDomainEvent::Analysis(AnalysisEvent::AnalysisStarted {
                analysis_id: AnalysisId::from(self.analysis_stream.clone()),
                session_id: SessionId::from(self.session_stream.clone()),
                reason: self.reason.clone(),
                started_at: Timestamp::now(),
            })
        );

        Ok(events)
    }
}
```

## State Management

State represents the current state of your aggregates, rebuilt from events:

```rust
#[derive(Default, Debug)]
struct SessionState {
    started: bool,
    ended: bool,
    request_count: usize,
}

// For multi-stream commands, combine state from multiple streams
#[derive(Default, Debug)]
struct CombinedState {
    // From session stream
    session_exists: bool,
    session_id: Option<SessionId>,

    // From analysis stream
    analysis_active: bool,
    analysis_id: Option<AnalysisId>,
}
```

State must implement:
- `Default`: Initial state before any events
- `Send + Sync`: For async execution

## Business Rule Validation

Use the `require!` macro for declarative validation:

```rust
// Simple validation
require!(state.balance >= amount, "Insufficient funds");

// Complex validation
require!(
    state.session_exists && !state.session_ended,
    "Cannot perform operation on invalid session"
);

// Validation with error types
let validated_amount = amount
    .validate()
    .map_err(|e| CommandError::ValidationFailed(e.to_string()))?;
```

## Error Handling

Commands return `CommandResult<Vec<StreamWrite<...>>>`:

```rust
// Using require! for business rules
require!(condition, "Error message");

// Using ? for fallible operations
let resolved = stream_resolver.resolve("dynamic-id").await?;

// Custom error handling
match validate_complex_rule(&state) {
    Ok(_) => {},
    Err(e) => return Err(CommandError::BusinessRuleViolation(e.to_string())),
}
```

## Testing Commands

Test commands using `InMemoryEventStore`:

```rust
#[tokio::test]
async fn test_command_success() {
    let event_store = Arc::new(InMemoryEventStore::new());

    let command = StartSession {
        session_stream: SessionStreamId::from(SessionId::generate()),
        user_id: UserId::generate(),
        application_id: ApplicationId::try_new("test-app").unwrap(),
    };

    // Execute command
    let result = event_store.handle_command(command).await;
    assert!(result.is_ok());

    // Verify events
    let events = event_store
        .read_stream(&session_stream.into(), None)
        .await
        .unwrap();

    assert_eq!(events.len(), 1);
    // Assert on event contents
}

#[tokio::test]
async fn test_business_rule_violation() {
    let event_store = Arc::new(InMemoryEventStore::new());

    // Setup initial state
    let command = /* ... */;
    event_store.handle_command(command).await.unwrap();

    // Try invalid operation
    let invalid_command = /* ... */;
    let result = event_store.handle_command(invalid_command).await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Expected error"));
}
```

## Best Practices

1. **Stream Design**: Each stream should represent a consistency boundary
2. **Event Granularity**: Events should capture business intent, not CRUD operations
3. **State Minimalism**: Only track state needed for business rules
4. **Idempotency**: Design commands to be safely retryable
5. **Error Messages**: Provide clear, actionable error messages

## Common Patterns

### Dynamic Stream Resolution

```rust
async fn handle(
    &self,
    read_streams: ReadStreams<Self::StreamSet>,
    state: Self::State,
    stream_resolver: &mut StreamResolver,
) -> CommandResult<Vec<StreamWrite<Self::StreamSet, Self::Event>>> {
    // Resolve streams dynamically
    let user_prefs_stream = stream_resolver
        .resolve(&format!("user-prefs-{}", self.user_id))
        .await?;

    // Use resolved stream...
}
```

### Conditional Event Emission

```rust
let mut events = Vec::new();

if state.should_notify {
    emit!(
        events,
        &read_streams,
        self.notification_stream.clone(),
        NotificationEvent::Created { /* ... */ }
    );
}

emit!(
    events,
    &read_streams,
    self.primary_stream.clone(),
    PrimaryEvent::Updated { /* ... */ }
);
```

### Saga Coordination

```rust
// Emit compensating events on failure
match external_service.call().await {
    Ok(result) => {
        emit!(events, &read_streams, stream, Event::Success { result });
    }
    Err(e) => {
        emit!(events, &read_streams, stream, Event::Failed { error: e });
        emit!(events, &read_streams, saga_stream, Event::CompensationRequired { /* ... */ });
    }
}
```

## Integration with Union Square

These patterns are used throughout Union Square for:
- Session management (single stream)
- Analysis initiation (multi-stream coordination)
- Test case extraction (three-stream atomic operations)
- Metrics calculation (aggregation across streams)

See `src/domain/commands_test.rs` for complete working examples.
