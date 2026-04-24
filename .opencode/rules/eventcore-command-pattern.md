# Rule: EventCore Command Pattern

All event-sourced state changes must use the EventCore command pattern.

## Command Definition

```rust
#[derive(Command, Clone, Debug, Serialize, Deserialize)]
pub struct PlaceOrder {
    #[stream]
    order_stream: StreamId,
    #[stream]
    customer_stream: StreamId,
    items: Vec<OrderItem>,
}
```

## CommandLogic Implementation

```rust
#[async_trait]
impl CommandLogic for PlaceOrder {
    type State = OrderState;
    type Event = DomainEvent;

    fn apply(&self, state: &mut Self::State, event: &StoredEvent<Self::Event>) {
        match &event.payload {
            DomainEvent::OrderPlaced { items, total } => {
                state.items = items.clone();
                state.total = *total;
            }
            _ => {}
        }
    }

    async fn handle(
        &self,
        read_streams: ReadStreams<Self::StreamSet>,
        state: Self::State,
        stream_resolver: &mut StreamResolver,
    ) -> CommandResult<Vec<StreamWrite<Self::StreamSet, Self::Event>>> {
        let mut events = Vec::new();

        require!(state.status == OrderStatus::Draft, "Order must be in draft status");

        let total = calculate_total(&self.items);
        emit!(
            events,
            &read_streams,
            self.order_stream.clone(),
            DomainEvent::OrderPlaced {
                items: self.items.clone(),
                total,
            }
        );

        Ok(events)
    }
}
```

## Key Rules

1. **Always derive `Command`** — Never implement stream types manually
2. **Use `#[stream]` attribute** — Mark all stream fields explicitly
3. **Use `require!` for business rules** — Not `if` + `return Err`
4. **Use `emit!` for events** — Never push events manually to the vec
5. **Event names are past tense** — `OrderPlaced`, not `PlaceOrder`
6. **State defaults to `Default`** — Ensure `State: Default + Send + Sync`

## Testing

Use `InMemoryEventStore` for unit tests:

```rust
#[tokio::test]
async fn placing_order_emits_event() {
    let store = InMemoryEventStore::new();
    let command = PlaceOrder {
        order_stream: StreamId::new("order-1").unwrap(),
        customer_stream: StreamId::new("customer-1").unwrap(),
        items: vec![OrderItem { name: "Widget".into(), price: 10.00 }],
    };

    let result = command.execute(&store).await.unwrap();

    assert_eq!(result.events.len(), 1);
    assert_matches!(
        result.events[0].payload,
        DomainEvent::OrderPlaced { .. }
    );
}
```

## Enforcement

- Code review by `event-sourcing-architect`
- Integration tests verifying event sequences
