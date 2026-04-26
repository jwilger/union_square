# Stream-Centric EventCore Design

Union Square uses EventCore streams as command consistency boundaries. A command declares every stream it must read before deciding whether to emit new events. Streams are not aggregates and are not read models; projections and queries build read models from durable events.

## Canonical Stream Names

The source of truth for stream factories is `src/domain/streams.rs`.

| Pattern | Purpose |
| --- | --- |
| `session:{session_id}` | Durable facts for one LLM session |
| `analysis:{analysis_id}` | Analysis workflow decisions and outcomes |
| `user:{user_id}:settings` | Settings for one user |
| `extraction:{extraction_id}` | Test-case extraction workflow decisions and outcomes |

Stream factories return `Result<StreamId, StreamNameError>`. Callers must propagate stream-name failures to the imperative shell rather than panicking.

## Command Boundaries

Commands must use `#[derive(Command)]` and mark every consistency-boundary field with `#[stream]`.

Single-stream commands declare one stream:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Command)]
pub struct RecordModelFScore {
    #[stream]
    model_stream: StreamId,
    // command data omitted
}
```

Multi-stream commands declare every stream that participates in the decision:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Command)]
pub struct RecordVersionChange {
    #[stream]
    from_stream: StreamId,
    #[stream]
    to_stream: StreamId,
    // command data omitted
}
```

The command handler returns `NewEvents<DomainEvent>` and uses `require!` for business rules. It must not perform IO, call clocks, generate runtime IDs in `apply`, or query read models.

## Query Plans

When a read path needs related streams, build an explicit stream plan first and let the imperative shell read those streams.

`session_with_analyses_streams` returns the session stream followed by the requested analysis streams. The helper is pure and does not read from EventCore.

```rust
let plan = session_with_analyses_streams(&session_id, &analysis_ids)?;
for stream in plan.all_streams() {
    // Imperative shell reads stream and feeds events into projections.
}
```

This keeps CQRS boundaries clear: streams hold facts, projections hold read models, and queries read projections.

## Lifecycle Documentation

`STREAM_DOCUMENTATION` records each stream pattern, purpose, lifecycle, retention policy, and related stream patterns. Update it when adding a new canonical stream family.
