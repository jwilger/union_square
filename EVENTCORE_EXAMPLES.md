# EventCore Command Examples for Union Square

This directory contains example implementations of EventCore commands demonstrating key patterns for the Union Square event-sourced architecture.

## Overview

Based on the event discovery from issue #157, we've implemented example commands that show:

1. **Single-stream commands** - Basic session management
2. **Multi-stream commands** - The `StartSessionAnalysis` pattern from the issue
3. **Three-stream atomic operations** - Complex test case extraction
4. **State management** - Combining state from multiple streams
5. **Business rule validation** - Using `require!` macro
6. **Error handling** - Proper command error patterns

## Key Files

- `src/domain/commands_test.rs` - Complete working examples of EventCore commands
- `src/domain/analysis_events.rs` - Additional events for the analysis domain
- `docs/eventcore-patterns.md` - Comprehensive guide to EventCore patterns

## The StartSessionAnalysis Pattern

The primary example from issue #157 demonstrates multi-stream coordination:

```rust
#[derive(Command)]
struct StartSessionAnalysis {
    #[stream]
    session_stream: SessionStreamId,
    #[stream]
    analysis_stream: AnalysisStreamId,

    reason: AnalysisReason,
}
```

This command:
- Reads from the session stream to validate session state
- Reads from the analysis stream to check for existing analyses
- Writes atomically to the analysis stream to start the analysis
- Demonstrates EventCore's multi-stream consistency guarantees

## Key EventCore Patterns Demonstrated

### 1. Using #[derive(Command)] Macro

```rust
#[derive(Command, Clone, Debug, Serialize, Deserialize)]
struct MyCommand {
    #[stream]
    stream_one: StreamTypeOne,
    #[stream]
    stream_two: StreamTypeTwo,

    // Regular fields
    data: String,
}
```

The macro automatically generates all required trait implementations.

### 2. State Management Across Streams

```rust
fn apply(&self, state: &mut Self::State, event: &StoredEvent<Self::Event>) {
    match &event.stream_id {
        stream_id if stream_id == self.session_stream.as_ref() => {
            // Update session-related state
        }
        stream_id if stream_id == self.analysis_stream.as_ref() => {
            // Update analysis-related state
        }
        _ => {}
    }
}
```

### 3. Business Rule Validation

```rust
async fn handle(...) -> CommandResult<...> {
    // Validate across multiple streams
    require!(state.session_exists, "Session not found");
    require!(state.request_count > 0, "No requests to analyze");
    require!(!state.analysis_started, "Analysis already started");

    // Proceed with event emission...
}
```

### 4. Atomic Multi-Stream Writes

```rust
let mut events = Vec::new();

// Events are collected and written atomically
emit!(events, &read_streams, self.analysis_stream.clone(),
    AnalysisEvent::Started { ... });

emit!(events, &read_streams, self.notification_stream.clone(),
    NotificationEvent::Created { ... });

Ok(events) // All events written atomically or none
```

## Running the Examples

```bash
# Run the example tests
cargo test commands_test

# Check the implementation
cargo check --all-targets
```

## Integration Points

These patterns integrate with Union Square's existing domain:
- Uses existing `SessionId`, `AnalysisId`, etc. from `domain::identifiers`
- Extends `DomainEvent` enum with analysis-specific events
- Follows the stream naming patterns from `domain::streams`
- Maintains type safety with nutype-based value objects

## Next Steps

To implement these patterns in production:

1. Move `analysis_events.rs` content into main `events.rs`
2. Implement production commands in `domain::commands/`
3. Add PostgreSQL event store configuration
4. Create projections for read models
5. Add integration tests with real event store

## References

- [EventCore Documentation](https://docs.rs/eventcore/latest/eventcore/)
- [Issue #157](https://github.com/jwilger/union_square/issues/157) - Original implementation request
- `docs/eventcore-patterns.md` - Detailed pattern guide
