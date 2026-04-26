# Rule: Acceptance Test Reachability

Every feature must have at least one acceptance test that exercises it end-to-end.

## Definition

An acceptance test:
- Starts the full application (or realistic test double for external services)
- Exercises the feature through its public interface (HTTP API)
- Verifies the outcome from the user's perspective

## Rules

1. **One acceptance test per feature** — Every user-visible behavior needs one
2. **Acceptance tests are not unit tests** — They test integration, not implementation details
3. **Acceptance tests must be deterministic** — No random data, no timing dependencies
4. **Acceptance tests must be fast** — Target < 5 seconds per test

## Example

```rust
#[tokio::test]
async fn proxy_records_complete_llm_session() {
    let app = create_test_app().await;
    let mock_bedrock = app.mock_bedrock().expect_request().respond_ok();

    let response = app
        .oneshot(build_proxy_request("hello"))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let session = app.get_session(session_id).await;
    assert_eq!(session.requests.len(), 1);
    assert_eq!(session.responses.len(), 1);
}
```

## Enforcement

- Code review by `tdd-coach` and `event-sourcing-test-architect`
- PRs without acceptance tests for new features are rejected
