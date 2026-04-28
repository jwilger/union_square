# Rule: Event Model Readiness

Before implementing any event-sourced feature, the event model must be reviewed and approved.

During the architecture alignment initiative, current persisted events, event schemas, and deployed behavior are not compatibility contracts. Existing event variants and fields MAY be replaced, removed, renamed, or retyped when that is the cleanest route to the target event model. Alignment-era changes MUST NOT require migration code, compatibility variants, or backward-compatible deserializers unless a specific schema has an acceptance record in `docs/accepted-replay/<schema-id>.yaml` marking it as accepted for ongoing historical replay.

An acceptance record MUST identify the schema, the accepted semantic version or commit hash, the acceptance timestamp, the approving human, the approver signature, and the PR link. The canonical format is documented in `docs/accepted-replay/README.md`; reviewers and CI MUST use that record to determine when compatibility obligations apply.

After an event schema is accepted as part of the aligned architecture, emitted events are historical facts. Future evolution MUST preserve replay by adding optional fields with defaults or by adding new event variants. Accepted historical fields and variants MUST NOT be removed, renamed, or retyped.

## Readiness Checklist

- [ ] Events are named in **past tense** (`SessionRecorded`, not `RecordSession`)
- [ ] Each event contains **all data needed** for future projections
- [ ] Events are **immutable after alignment acceptance** — once an aligned schema is accepted for historical use, never modify its existing fields or variants in place
- [ ] **Incremental fields after alignment acceptance** — new events can add fields after schema acceptance, but accepted historical fields are never removed or retyped
- [ ] **Alignment scope is explicit** — compatibility variants and migrations are only required after a schema is accepted for ongoing historical replay
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

Schema evolution rules apply after alignment acceptance.

Before alignment acceptance, prefer the clean target event model over transitional compatibility. Do not add V1/V2 variants solely to preserve current alignment-era persisted data unless an acceptance record requires preserving that schema.

After alignment acceptance, when you need to change event structure:

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
