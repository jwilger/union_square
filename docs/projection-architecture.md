# Projection Architecture

## Overview

This document describes the projection architecture implemented for Union Square's event-sourced system.

## Problem Statement

The initial implementation used query-time projections that rebuilt state from events on every query. This approach:
- Was inefficient for large event streams
- Violated CQRS principles by not separating write and read models
- Did not leverage EventCore's built-in projection capabilities

## Solution Approach

We implemented a functional projection system that:
1. Maintains materialized views that update as events arrive
2. Uses immutable state updates following functional programming principles
3. Provides query services that read from pre-computed projections

## Architecture Components

### Core Infrastructure (`core.rs`)

- `FunctionalProjection` trait: Defines the contract for projections with immutable state updates
- `ProjectionAdapter`: Bridges our functional projections to EventCore's Projection trait
- `InMemoryProjectionStore`: Simple storage for projection state during development

### Projections

#### SessionSummaryProjection (`session_summary.rs`)
- Maintains summaries of all sessions with aggregated metrics
- Tracks request counts, response times, and models used
- Provides queries by user, application, and session status

#### UserActivityProjection (`user_activity.rs`)
- Aggregates user activity across all their sessions
- Tracks application usage and model preferences
- Supports top-user queries and usage analytics

### Projection Manager (`manager.rs`)

- Manages projection lifecycle (start/stop)
- Handles event subscriptions (simplified implementation)
- Provides registry for all projections

### Query Service (`queries_v2.rs`)

- `MaterializedQueryService`: Reads from pre-computed projections
- Provides type-safe query methods
- Returns domain-specific result types

## Implementation Challenges

### EventCore Projection Trait

EventCore's built-in `Projection` trait has specific requirements:
- All methods are async
- `apply_event` takes mutable state and Event (not StoredEvent)
- Complex lifetime requirements
- Different error types (ProjectionResult vs Result)

Our simplified implementation focused on the functional core while deferring full EventCore integration.

### Type System Constraints

- Nutype validation at boundaries required careful handling
- Private type re-exports needed proper imports
- EventCore types (EventId, StreamId) have specific constructors

## Design Decisions

1. **Functional Core**: All projections use immutable state updates
2. **Separate Storage**: Projection state stored separately from event store
3. **Simplified Subscriptions**: Initial implementation polls rather than true subscriptions
4. **In-Memory Storage**: Development implementation uses in-memory storage

## Future Improvements

1. **Full EventCore Integration**: Properly implement the Projection trait with all async methods
2. **Persistent Storage**: Store projection state in PostgreSQL
3. **True Subscriptions**: Implement proper event stream subscriptions
4. **Checkpointing**: Add checkpoint support for resumable projections
5. **Error Handling**: Implement retry logic and error recovery

## Testing Strategy

- Unit tests verify immutable state updates
- Functional tests verify projection logic
- Query tests ensure proper data retrieval
- Integration tests would verify end-to-end flows

## Migration Path

To migrate from query-time projections:
1. Deploy projection infrastructure
2. Run projections to build initial state
3. Switch queries to use materialized views
4. Remove old ProjectionBuilder code

## Conclusion

This architecture provides a foundation for efficient CQRS with event sourcing. While not fully integrated with EventCore's projection system, it demonstrates the principles and provides a clear migration path.
