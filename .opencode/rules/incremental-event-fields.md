# Rule: Incremental Event Fields

Event schemas must only evolve by adding new fields or new event variants. Never remove, rename, or change the type of existing fields.

## Why

Events are immutable facts. Once an event is persisted, it exists forever in the event store. Changing an existing field's type or name breaks deserialization of historical events.

## Allowed Changes

1. **Add new optional fields** to existing event variants (with `#[serde(default)]`)
2. **Add new event variants** to the event enum
3. **Add new event types** entirely

## Forbidden Changes

1. Removing a field from an existing event variant
2. Renaming a field in an existing event variant
3. Changing the type of an existing field
4. Removing an event variant that has been emitted in production

## Migration Strategy

When you need data that an old event doesn't have:

1. Create a new event variant with the additional data
2. Update command handlers to emit the new variant
3. Keep the old variant in the enum for historical replay
4. In projections, handle both old and new variants

## Example

```rust
// Original
enum DomainEvent {
    SessionStarted { session_id: Uuid },
}

// After adding metadata requirement
enum DomainEvent {
    SessionStarted { session_id: Uuid }, // Keep original
    SessionStartedWithMetadata {
        session_id: Uuid,
        metadata: SessionMetadata,
    },
}
```

## Enforcement

- ADR required for any event schema change
- Code review by `event-sourcing-architect`
- Backward compatibility tests in CI
