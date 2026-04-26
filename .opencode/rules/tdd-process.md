# TDD Process Guardrail

## Mandatory Approach: Kent Beck Style Outside-In TDD

When changing behavior, you MUST follow strict Test-Driven Development:

### The Cycle (Red → Green → Refactor)

1. **Red**: Write exactly ONE failing test that describes the next smallest piece of desired behavior
2. **Green**: Write the minimum code to make that ONE test pass
3. **Refactor**: Clean up while keeping the test green
4. Repeat

### Anti-Patterns (NEVER DO THESE)

- NEVER write all tests up front before implementing
- NEVER write more than one failing test at a time
- NEVER implement code without a failing test first
- NEVER skip the refactor step
- NEVER write tests after implementation "just to verify"

### Outside-In BDD

Start from the outermost boundary:
- First acceptance test through the public API/HTTP boundary
- Then unit tests for the next layer inward as needed
- Domain command tests with InMemoryEventStore
- Pure function property tests

### Refactoring

Refactoring without behavior change:
- Characterization test first if behavior isn't already tested
- Make the change easy
- Make the easy change
- Green tests before and after

### Test Structure

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
    assert_eq!(events.len(), 3);
}
```

### Enforcement

- The `tdd-coach` bot runs automated checks and posts review comments on PRs that match configured source-code paths. It flags code changes that lack corresponding tests. It does not replace human approval.
- The CI workflow `ci.yml` runs `cargo nextest run --workspace` and `cargo tarpaulin` (or equivalent coverage job). PRs that reduce coverage below the threshold configured in `codecov.yml` (or the active coverage gate) will fail the coverage check.
- The pre-merge `test-presence` check (via lefthook or the CI `test` job) flags PRs missing tests for new behavior. Branch-protection rules on `main` require the `test` and `coverage` checks to pass before merging; maintainers may override with documented justification.
