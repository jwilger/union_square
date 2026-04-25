# Rule: CQRS Model Separation

Maintain clear separation between command (write) and query (read) models.

## Guidelines

1. **Commands change state** — They validate, emit events, and write to the event store
2. **Queries read state** — They read from projections or read models, never from the event store directly
3. **Never mix commands and queries in the same handler** — A function should either write or read, not both
4. **Projections are separate from aggregates** — Event handlers that build read models are not part of the command logic

## Structure

```
src/
  commands/     # Command handlers (write side)
  queries/      # Query handlers (read side)
  projections/  # Event handlers that build read models
  domain/       # Aggregates, events, value objects
```

## Example

```rust
// Command — writes
pub async fn handle_place_order(cmd: PlaceOrder, store: &EventStore) -> Result<(), Error> {
    cmd.execute(store).await
}

// Query — reads
pub async fn get_order_summary(order_id: &StreamId, db: &PgPool) -> Result<OrderSummary, Error> {
    sqlx::query_as!("SELECT * FROM order_summaries WHERE order_id = $1", order_id.as_str())
        .fetch_one(db)
        .await
}
```

## Enforcement

- Code review by `event-sourcing-architect`
- `ast-grep` rules detecting mixed read/write in handlers
