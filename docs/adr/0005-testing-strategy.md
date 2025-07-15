# Testing Strategy

- Status: accepted
- Deciders: John Wilger, Technical Architecture Team
- Date: 2024-07-15

## Context and Problem Statement

Union Square requires a comprehensive testing strategy that ensures reliability while maintaining fast feedback loops. The system must test complex async behavior, validate proxy performance requirements (< 5ms latency), ensure provider compatibility, and support the test case extraction feature where LLM interactions can become regression tests.

## Decision Drivers

- **Fast Feedback**: Tests must run quickly for developer productivity
- **Comprehensive Coverage**: Test business logic, integration points, and performance
- **Type Safety**: Leverage type system to reduce test burden
- **Async Testing**: Handle async/streaming behavior correctly
- **Test Extraction**: Support converting real sessions into test cases
- **CI/CD Integration**: Tests must work in automated pipelines

## Considered Options

- **Option 1**: Traditional unit/integration/e2e pyramid
- **Option 2**: Property-based testing first
- **Option 3**: Behavior-driven development (BDD)
- **Option 4**: Hybrid approach with property and example tests

## Decision Outcome

Chosen option: **"Hybrid approach with property and example tests"** because it leverages our strong type system while providing concrete examples for complex scenarios. Property-based tests verify invariants, while example-based tests cover specific behaviors and edge cases.

### Testing Layers

1. **Type-Driven Tests** (Compile Time)
   - Type system ensures many properties
   - No runtime tests needed for type safety

2. **Property-Based Tests** (Unit)
   - Test invariants and laws
   - Generate random valid inputs
   - Focus on pure functions

3. **Example-Based Tests** (Unit/Integration)
   - Specific scenarios and edge cases
   - Provider-specific behavior
   - Error conditions

4. **Performance Tests** (Integration)
   - Validate < 5ms latency requirement
   - Load testing for concurrent sessions
   - Memory usage under load

5. **Contract Tests** (Integration)
   - Verify provider API compatibility
   - Use recorded responses for stability

### Implementation Examples

```rust
// Property-based test using proptest
#[cfg(test)]
mod property_tests {
    use proptest::prelude::*;
    
    proptest! {
        #[test]
        fn session_id_roundtrip(s in "[a-zA-Z0-9-]{1,50}") {
            let session_id = SessionId::new(&s).unwrap();
            let serialized = serde_json::to_string(&session_id).unwrap();
            let deserialized: SessionId = serde_json::from_str(&serialized).unwrap();
            prop_assert_eq!(session_id, deserialized);
        }
        
        #[test]
        fn recording_event_ordering(
            events in prop::collection::vec(arb_session_event(), 1..100)
        ) {
            let mut pipeline = RecordingPipeline::new();
            for event in &events {
                pipeline.process(event.clone()).await.unwrap();
            }
            let stored = pipeline.get_events().await;
            prop_assert_eq!(events.len(), stored.len());
            // Verify timestamp ordering preserved
            prop_assert!(stored.windows(2).all(|w| w[0].timestamp <= w[1].timestamp));
        }
    }
}

// Performance test
#[tokio::test]
async fn proxy_latency_under_5ms() {
    let proxy = test_helpers::start_proxy().await;
    let client = TestClient::new(&proxy);
    
    // Warm up
    for _ in 0..10 {
        client.proxy_request(test_request()).await.unwrap();
    }
    
    // Measure
    let mut latencies = Vec::new();
    for _ in 0..100 {
        let start = Instant::now();
        client.proxy_request(test_request()).await.unwrap();
        latencies.push(start.elapsed());
    }
    
    let p99 = percentile(&latencies, 99.0);
    assert!(p99 < Duration::from_millis(5), "P99 latency: {:?}", p99);
}

// Contract test with recorded response
#[test]
fn openai_chat_completion_parsing() {
    let recorded_response = include_str!("fixtures/openai_chat_response.json");
    let parsed: OpenAiChatResponse = serde_json::from_str(recorded_response).unwrap();
    
    assert_eq!(parsed.model, "gpt-4");
    assert!(parsed.usage.total_tokens > 0);
}
```

### Test Helpers and Fixtures

```rust
// Test builder for complex types
pub struct SessionBuilder {
    id: Option<SessionId>,
    metadata: HashMap<String, serde_json::Value>,
}

impl SessionBuilder {
    pub fn new() -> Self { 
        Self { 
            id: None, 
            metadata: HashMap::new() 
        } 
    }
    
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(SessionId::new(id.into()).unwrap());
        self
    }
    
    pub fn build(self) -> Session {
        Session {
            id: self.id.unwrap_or_else(|| SessionId::new("test-session").unwrap()),
            metadata: SessionMetadata::from(self.metadata),
            // ...
        }
    }
}

// Async test helpers
pub async fn with_test_db<F, Fut>(f: F) 
where 
    F: FnOnce(Database) -> Fut,
    Fut: Future<Output = ()>,
{
    let db = Database::connect("postgres://localhost/union_square_test").await.unwrap();
    db.begin_test_transaction().await;
    f(db.clone()).await;
    db.rollback_test_transaction().await;
}
```

### CI/CD Integration

```yaml
# GitHub Actions test job
test:
  runs-on: ubuntu-latest
  services:
    postgres:
      image: postgres:15
      env:
        POSTGRES_PASSWORD: postgres
      options: >-
        --health-cmd pg_isready
        --health-interval 10s
        --health-timeout 5s
        --health-retries 5
  
  steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable
    
    - name: Run tests
      run: |
        cargo test --all-features
        cargo test --doc
    
    - name: Run property tests (extended)
      run: |
        PROPTEST_CASES=10000 cargo test --test property_tests
    
    - name: Performance tests
      run: |
        cargo test --test performance --release
```

### Positive Consequences

- **High Confidence**: Property tests find edge cases automatically
- **Fast Execution**: Most tests run in milliseconds
- **Clear Documentation**: Tests serve as usage examples
- **Regression Prevention**: Extracted test cases prevent regressions
- **Type Leverage**: Many properties verified at compile time

### Negative Consequences

- **Learning Curve**: Property-based testing requires different thinking
- **Test Complexity**: Some async tests are complex to write
- **Flaky Tests**: Network/timing issues in integration tests

## Pros and Cons of the Options

### Option 1: Traditional unit/integration/e2e pyramid

Standard testing pyramid with mostly unit tests.

- Good, because well understood by developers
- Good, because clear test boundaries
- Bad, because misses edge cases
- Bad, because lots of mocking required
- Bad, because doesn't leverage type system

### Option 2: Property-based testing first

Focus primarily on property tests with generators.

- Good, because finds edge cases automatically
- Good, because tests invariants comprehensively
- Bad, because harder to debug failures
- Bad, because not all properties are easy to express
- Bad, because slower than example tests

### Option 3: Behavior-driven development

Cucumber-style tests with Given/When/Then.

- Good, because readable by non-developers
- Good, because focuses on behavior
- Bad, because adds abstraction layer
- Bad, because slower to write and run
- Bad, because doesn't leverage types

### Option 4: Hybrid approach

Combine property and example tests based on context.

- Good, because uses right tool for each job
- Good, because leverages type system
- Good, because balances coverage and speed
- Bad, because requires multiple testing skills
- Bad, because more test infrastructure

## Links

- Influenced by [ADR-0004](0004-type-system.md) - Types enable property testing
- Influences future ADR on CI/CD pipeline
- Related to [ADR-0001](0001-overall-architecture-pattern.md) - Tests focus on pure functions