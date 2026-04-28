# Rule: Outside-In TDD Execution

All production code must be written using outside-in test-driven development.

## The Cycle

1. **Red**: Write a failing test that describes the desired behavior
2. **Green**: Write the minimum code to make the test pass
3. **Refactor**: Clean up the code while keeping tests green

## Execution Order

1. **Start with an acceptance test** — Define the feature from the user's perspective
2. **Drive inward with unit tests** — Test components in isolation as you discover them
3. **Test at the appropriate boundary**:
   - HTTP handlers: Integration tests with the full Axum app
   - Domain commands: Unit tests with `InMemoryEventStore`
   - Pure functions: Property-based tests with `proptest` or `quickcheck`

## Rules

- **No production code without a failing test first** — The test must fail for the right reason
- **Tests are executable specifications** — They should read like documentation
- **One concept per test** — Don't test multiple things in one test function
- **Descriptive test names** — `order_rejected_when_insufficient_funds`, not `test_order_1`

## Test Structure

```rust
#[tokio::test]
async fn session_records_all_llm_interactions() {
    // Arrange
    let app = create_test_app().await;
    let request = build_proxy_request();

    // Act
    let response = app.oneshot(request).await.unwrap();

    // Assert
    assert_eq!(response.status(), StatusCode::OK);
    let events = app.event_store.events_for_stream("session-1").await;
    assert_eq!(events.len(), 3); // request, response, summary
}
```

## Enforcement

- Code review by `tdd-coach` and `event-sourcing-test-architect`
- Coverage gates in CI
- PRs without tests are rejected
