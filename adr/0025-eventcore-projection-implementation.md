# 0025. EventCore Projection Implementation

Date: 2025-07-31

## Status

Accepted

## Context

Union Square's event-sourced system requires efficient read models to support CQRS patterns. The initial implementation used query-time projections that rebuilt state from events on every query, which:

- Was inefficient for large event streams
- Violated CQRS principles by not separating write and read models
- Did not leverage EventCore's built-in projection capabilities

EventCore provides a `Projection` trait designed for maintaining materialized views that update as events arrive, but it has specific requirements:

- All methods are async
- `apply_event` takes mutable state and Event (not StoredEvent)
- Complex lifetime requirements
- Uses ProjectionResult instead of standard Result types

## Decision

We will implement projections using EventCore's native `Projection` trait with the following approach:

1. **Imperative Shell Pattern**: Projections will use mutable state updates as part of the imperative shell, with pure domain logic in the core
2. **Session Summary Projection**: First concrete implementation tracking session state, metrics, and lifecycle
3. **Materialized Views**: Pre-computed projections stored separately from event streams
4. **Query Service**: Type-safe query interface reading from materialized projections

### Architecture Components

#### Core Infrastructure
- EventCore's native `Projection` trait implementation
- PostgreSQL storage for projection state
- Projection service for managing multiple projections
- Health checks and monitoring

#### Domain Projections
- `SessionSummaryProjection`: Aggregates session data across streams
- Query methods: by user, application, session status
- Metrics: request counts, response times, models used

### Implementation Constraints

1. **Mutable State**: Accept EventCore's imperative design rather than forcing functional approaches
2. **Async Methods**: All projection methods must be async
3. **Error Handling**: Use EventCore's ProjectionResult type system
4. **Type Safety**: Maintain strong typing throughout projection pipeline

## Alternatives Considered

### Functional Projection Layer
- **Considered**: Building a functional wrapper around EventCore's imperative Projection trait
- **Rejected**: Added complexity without significant benefit, fighting against EventCore's design

### Query-Time Projections
- **Considered**: Keeping the existing query-time approach with optimizations
- **Rejected**: Doesn't scale with large event streams, violates CQRS principles

### Custom Projection System
- **Considered**: Building our own projection infrastructure
- **Rejected**: Duplicates EventCore functionality, increases maintenance burden

## Consequences

### Positive
- Leverages EventCore's mature projection system
- Proper CQRS separation between writes and reads
- Efficient queries against materialized views
- Built-in checkpointing and error recovery
- Type-safe projection pipeline

### Negative
- Must work within EventCore's imperative design patterns
- All projection methods are async (complexity in simple operations)
- Requires PostgreSQL storage for projection state
- Initial implementation effort to migrate from query-time approach

### Migration Strategy
1. Implement EventCore projections alongside existing query system
2. Run projections to build initial materialized state
3. Switch queries to use materialized views
4. Remove legacy projection builder code

## Implementation Details

### SessionSummaryProjection Structure
```rust
pub struct SessionSummaryProjection;

#[async_trait]
impl Projection for SessionSummaryProjection {
    type State = SessionSummaryState;
    type Event = DomainEvent;

    async fn apply_event(&self, state: &mut Self::State, event: &Event) -> ProjectionResult<()> {
        // Mutable state updates as part of imperative shell
    }
}
```

### Query Interface
```rust
pub struct MaterializedQueryService {
    // Access to projection state
}

impl MaterializedQueryService {
    pub async fn active_sessions(&self) -> Result<Vec<SessionSummary>, QueryError> {
        // Type-safe queries against materialized views
    }
}
```

## Future Considerations

1. **Multiple Projections**: Additional projections for user activity, version analytics
2. **Projection Sharding**: Horizontal scaling for large datasets
3. **Event Replay**: Rebuild projections from event history
4. **Cross-Stream Queries**: Projections spanning multiple event streams

## Notes

This ADR supersedes the previous approach documented in `docs/projection-architecture.md`, which explored functional projection patterns that didn't align with EventCore's design philosophy.
