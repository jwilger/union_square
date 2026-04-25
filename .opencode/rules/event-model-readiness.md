# Rule: Event Model Readiness

Before implementing any event-sourced feature, the event model must be reviewed and approved.

During the architecture alignment initiative, current persisted event schemas may be replaced when that is the cleanest path. After a schema is accepted as part of the aligned architecture, future event evolution must be additive or use new event variants so historical replay remains safe.

## Readiness Checklist

- [ ] Events are named in **past tense** (`SessionRecorded`, not `RecordSession`)
- [ ] Each event contains **all data needed** for future projections
- [ ] Events are **immutable** — never modify an event's schema after it's in use
- [ ] **Incremental fields after alignment** — new events can add fields after schema acceptance, but accepted historical fields are never removed or retyped
- [ ] Events have **clear stream boundaries** — every event belongs to a logical aggregate stream
- [ ] **No event references external mutable state** — events are self-contained facts

## Event Naming Convention

```rust
// Good
enum DomainEvent {
    SessionStarted { session_id: Uuid, provider: String, started_at: DateTime<Utc> },
    RequestForwarded { request_id: Uuid, model: String, tokens_prompt: u32 },
    ResponseReceived { request_id: Uuid, tokens_completion: u32, duration_ms: u64 },
    SessionCompleted { session_id: Uuid, total_requests: u32, ended_at: DateTime<Utc> },
}

// Bad
enum DomainEvent {
    StartSession,                    // Imperative, not past tense
    ForwardRequest { id: Uuid },     // Insufficient context
    GetResponse,                     // Imperative
    EndSession,                      // Imperative
}
```

## Schema Evolution

After alignment, when you need to change event structure:

1. **Add new fields**: Add a new event variant with the additional data
2. **Deprecate old variants**: Stop emitting the old variant, but keep it in the enum for deserialization
3. **Document in ADR**: Create an ADR explaining the migration strategy

```rust
enum DomainEvent {
    // Old variant — kept for backward compatibility
    #[serde(rename = "RequestForwarded")]
    RequestForwardedV1 { request_id: Uuid },

    // New variant — current
    #[serde(rename = "RequestForwardedV2")]
    RequestForwardedV2 {
        request_id: Uuid,
        model: String,
        tokens_prompt: u32,
    },
}
```

## Enforcement

- Code review by `event-sourcing-architect` and `event-modeling-expert`
- ADR required for any event schema change
