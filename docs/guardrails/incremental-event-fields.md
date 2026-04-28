# Rule: Incremental Event Fields

Event schema compatibility has two phases.

During the architecture alignment initiative, current persisted events, event schemas, and deployed behavior are not compatibility contracts. Existing schemas MAY be replaced when that is the cleanest path. No migration code, compatibility variants, or backward-compatible deserializers are required for current persisted events unless a specific schema has an acceptance record in `docs/accepted-replay/<schema-id>.yaml` marking it as accepted for ongoing historical replay.

An acceptance record MUST identify the schema, the accepted semantic version or commit hash, the acceptance timestamp, the approving human, the approver signature, and the PR link. The canonical format is documented in `docs/accepted-replay/README.md`; reviewers and CI MUST use that record to determine when compatibility obligations and backward compatibility tests apply.

After the aligned event model is accepted, event schemas MUST evolve only by adding optional fields with defaults or by adding new event variants. Accepted historical event fields MUST NOT be removed, renamed, or retyped.

## Why

Events are immutable facts. Once a schema is accepted for ongoing historical replay, changing an existing field's type or name breaks deserialization of historical events.

## Allowed Changes After Alignment

1. **Add new optional fields** to accepted event variants (with `#[serde(default)]`)
2. **Add new event variants** to the event enum
3. **Add new event types** entirely

## Forbidden Changes After Alignment

An accepted event variant is one explicitly accepted for ongoing historical replay after alignment. Acceptance does not require prior production emission, but once accepted and emitted, it must remain replayable.

1. Removing a field from an event variant accepted after alignment
2. Renaming a field in an event variant accepted after alignment
3. Changing the type of a field in an event variant accepted after alignment
4. Removing an event variant accepted after alignment once it has been emitted

## Post-Alignment Migration Strategy

These steps apply only after a schema has been accepted for ongoing historical replay.

When you need data that an accepted historical event does not have:

1. Create a new event variant with the additional data
2. Update command handlers to emit the new variant
3. Keep the old variant in the enum for historical replay
4. In projections, handle both old and new variants

Do not use this pattern solely for alignment-era schemas that have not been accepted as durable historical contracts.

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
- Backward compatibility tests in CI for schemas accepted as part of the aligned architecture
