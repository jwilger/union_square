# TDD Workflow for Union Square

## Overview

This document outlines the Test-Driven Development (TDD) workflow adopted for Union Square, following Kent Beck's red-green-refactor cycle. The goal is to ensure all features are developed with tests driving the design, creating a sustainable and reliable codebase.

## The Red-Green-Refactor Cycle

### 🔴 RED: Write a Failing Test First

**Before writing any production code**, always start with a failing test that describes the desired behavior.

#### Key Principles:
- Tests should fail for the **right reason** - they should fail because the functionality doesn't exist yet
- Keep tests small and focused on a single behavior
- Use descriptive test names that explain the expected behavior
- Write only enough test code to see it fail

#### Example: Request ID Middleware

```rust
#[tokio::test]
async fn test_request_id_generation() {
    // This test MUST fail first
    let request = Request::builder()
        .method("GET")
        .uri("/test")
        .body(Body::empty())
        .unwrap();

    let service = tower::ServiceBuilder::new()
        .layer(from_fn(request_id_middleware))  // This doesn't exist yet!
        .service(handler);

    let response = service.oneshot(request).await.unwrap();
    assert!(response.headers().contains_key("x-request-id"));

    let request_id = response.headers().get("x-request-id").unwrap();
    let uuid = Uuid::parse_str(request_id.to_str().unwrap()).unwrap();
    assert_eq!(uuid.get_version_num(), 7);
}
```

**Run the test**: `cargo test test_request_id_generation`
**Expected result**: Compilation failure because `request_id_middleware` doesn't exist.

### 🟢 GREEN: Write Minimal Code to Pass

Write **only enough** code to make the failing test pass. Resist the urge to add extra functionality.

#### Key Principles:
- Write the simplest implementation that could possibly work
- Don't add functionality not required by the current test
- The code can be ugly at this stage - that's what refactoring is for
- Focus on making the test pass, nothing more

#### Example: Minimal Request ID Implementation

```rust
pub async fn request_id_middleware(
    mut request: Request,
    next: Next,
) -> Result<Response, ProxyError> {
    // Minimal implementation - just add a UUID v7
    let request_id = Uuid::now_v7();
    let header_value = HeaderValue::from_str(&request_id.to_string())
        .expect("UUID v7 should always produce valid header value");

    request.headers_mut().insert("x-request-id", header_value.clone());

    let mut response = next.run(request).await;
    response.headers_mut().insert("x-request-id", header_value);

    Ok(response)
}
```

**Run the test**: `cargo test test_request_id_generation`
**Expected result**: Test passes ✅

### 🔵 REFACTOR: Improve the Design

Clean up the code while keeping all tests green. This is where we improve the design without changing behavior.

#### Key Principles:
- Remove duplication
- Improve naming for clarity
- Extract functions or modules when appropriate
- Ensure the design remains simple and expressive
- Run tests after each refactoring step

#### Example: Refactored Request ID Implementation

```rust
use crate::proxy::headers::X_REQUEST_ID;

pub async fn request_id_middleware(
    mut request: Request,
    next: Next,
) -> Result<Response, ProxyError> {
    // Check if request already has an ID
    let request_id = if let Some(existing_id) = request.headers().get(X_REQUEST_ID) {
        validate_and_use_existing_id(existing_id)
            .unwrap_or_else(generate_new_request_id)
    } else {
        generate_new_request_id()
    };

    let request_id_clone = request_id.clone();
    request.headers_mut().insert(X_REQUEST_ID, request_id);

    let mut response = next.run(request).await;
    response.headers_mut().insert(X_REQUEST_ID, request_id_clone);

    Ok(response)
}

fn generate_new_request_id() -> HeaderValue {
    let new_id = Uuid::now_v7();
    HeaderValue::from_str(&new_id.to_string())
        .expect("UUID v7 should always produce valid header value")
}

fn validate_and_use_existing_id(existing_id: &HeaderValue) -> Option<HeaderValue> {
    existing_id
        .to_str()
        .ok()
        .and_then(|s| Uuid::parse_str(s).ok())
        .and_then(|uuid| HeaderValue::from_str(&uuid.to_string()).ok())
}
```

**Run the tests**: `cargo test`
**Expected result**: All tests still pass ✅

## TDD Commit Patterns

Your commit history should clearly show the red-green-refactor cycle:

```bash
# Red phase
git commit -m "test(middleware): add failing test for request ID generation"

# Green phase
git commit -m "feat(middleware): implement basic request ID generation"

# Refactor phase
git commit -m "refactor(middleware): extract ID validation and improve error handling"
```

### Commit Message Guidelines

- **test(scope)**: Adding failing tests (red phase)
- **feat(scope)**: Adding new functionality (green phase)
- **refactor(scope)**: Improving design without changing behavior (refactor phase)
- **fix(scope)**: Fixing bugs discovered by tests

## Test Organization

### Structure Tests by Behavior

Organize tests around behaviors, not implementation details:

```rust
#[cfg(test)]
mod middleware_tests {
    mod when_processing_requests_without_id {
        #[test]
        fn should_generate_uuid_v7() { }

        #[test]
        fn should_add_id_to_request_and_response() { }
    }

    mod when_processing_requests_with_existing_id {
        #[test]
        fn should_preserve_valid_uuid() { }

        #[test]
        fn should_replace_invalid_id() { }
    }
}
```

### Test Categories

1. **Unit Tests**: Test individual functions in isolation
2. **Integration Tests**: Test how components work together
3. **Property-Based Tests**: Test invariants across many inputs
4. **Characterization Tests**: Document existing behavior when adding tests to legacy code

## Development Workflow Commands

### Fast Feedback Loop

```bash
# Run tests on file changes (requires cargo-watch)
cargo watch -x 'test --lib'

# Run only changed tests (requires cargo-nextest)
cargo nextest run --changed

# Run a specific test
cargo test test_name -- --nocapture

# Run tests with coverage
cargo tarpaulin --out html
```

### TDD-Specific Commands

```bash
# Verify test fails before implementing
cargo test new_feature_test
# Expected: failure

# Implement minimal code, then verify test passes
cargo test new_feature_test
# Expected: success

# Run all tests after refactoring
cargo test
# Expected: all pass
```

## Common TDD Patterns

### Test Doubles

Use mocks, stubs, and fakes to isolate units under test:

```rust
// Mock external dependencies
#[cfg(test)]
struct MockEventStore {
    recorded_events: RefCell<Vec<DomainEvent>>,
}

impl EventStore for MockEventStore {
    async fn append_events(&self, events: Vec<DomainEvent>) -> Result<()> {
        self.recorded_events.borrow_mut().extend(events);
        Ok(())
    }
}
```

### Parameterized Tests

Test multiple scenarios with the same logic:

```rust
#[test]
fn test_invalid_api_keys() {
    let invalid_keys = vec![
        "",
        "too-short",
        "invalid-chars!@#",
        " whitespace-padded ",
    ];

    for key in invalid_keys {
        let result = ApiKey::try_new(key.to_string());
        assert!(result.is_err(), "Expected '{}' to be invalid", key);
    }
}
```

### Property-Based Testing

Test invariants across many generated inputs:

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn valid_request_ids_are_always_preserved(
        valid_uuid in "[a-f0-9]{8}-[a-f0-9]{4}-7[a-f0-9]{3}-[89ab][a-f0-9]{3}-[a-f0-9]{12}"
    ) {
        let request = Request::builder()
            .header("x-request-id", &valid_uuid)
            .body(Body::empty())
            .unwrap();

        let processed = process_request(request).await.unwrap();
        let response_id = processed.headers().get("x-request-id").unwrap().to_str().unwrap();

        prop_assert_eq!(response_id, valid_uuid);
    }
}
```

## TDD Anti-Patterns to Avoid

### ❌ Writing Tests After Implementation

This loses the design benefits of TDD:

```rust
// BAD: Implementation first
pub fn calculate_discount(customer: &Customer, order: &Order) -> Discount {
    // Complex implementation here...
}

// Then writing tests after
#[test]
fn test_calculate_discount() {
    // Test written to match existing implementation
}
```

### ❌ Testing Implementation Details

Tests should survive refactoring:

```rust
// BAD: Testing internal structure
#[test]
fn test_uses_hash_map_internally() {
    let cache = Cache::new();
    assert!(cache.internal_storage.is_empty()); // Will break on refactoring
}

// GOOD: Testing behavior
#[test]
fn test_caches_expensive_computations() {
    let cache = Cache::new();
    let result1 = cache.get_or_compute("key", expensive_function);
    let result2 = cache.get_or_compute("key", expensive_function);

    assert_eq!(result1, result2);
    assert_eq!(expensive_function.call_count(), 1); // Only called once
}
```

### ❌ Overly Complex Tests

If a test is hard to write, the design is probably wrong:

```rust
// BAD: Complex test setup indicates poor design
#[test]
fn test_complex_workflow() {
    let mut mock1 = MockService1::new();
    let mut mock2 = MockService2::new();
    let mut mock3 = MockService3::new();
    // 50 lines of setup...

    let result = complex_workflow(&mock1, &mock2, &mock3, /* many args */);
    // Complex assertions...
}
```

## Integration with Type-Driven Development

Union Square combines TDD with type-driven development for maximum safety:

### Types Reduce Test Burden

Well-designed types eliminate entire categories of tests:

```rust
// The type system prevents invalid states
pub struct AuthenticatedRequest<T> {
    inner: T,
    api_key: ApiKey, // Guaranteed to be valid
}

// So we don't need to test invalid API key scenarios in business logic
#[test]
fn test_process_authenticated_request() {
    let request = AuthenticatedRequest::new(/* valid request */, valid_api_key);
    // No need to test invalid API key - type system prevents it
    let result = process_request(request);
    assert!(result.is_ok());
}
```

### Test Behavior, Not Types

Don't test that the compiler works:

```rust
// BAD: Testing type system
#[test]
fn test_api_key_is_string() {
    let key = ApiKey::try_new("test".to_string()).unwrap();
    assert!(key.as_str().is_ascii()); // Compiler already guarantees this
}

// GOOD: Testing business rules
#[test]
fn test_api_key_grants_access_to_correct_resources() {
    let key = ApiKey::try_new("user-key".to_string()).unwrap();
    let resources = key.allowed_resources();
    assert!(!resources.contains(&AdminResource::UserManagement));
}
```

## Measuring TDD Success

### Metrics to Track

1. **Test Coverage**: Should be > 80% for core domain logic
2. **Test Speed**: Unit tests should run in < 1ms each
3. **Commit Patterns**: Evidence of red-green-refactor in git history
4. **Defect Rate**: Lower bug rates in production
5. **Development Velocity**: Faster feature delivery after initial investment

### Git History Analysis

Good TDD should show in your git history:

```bash
# Look for TDD patterns in commits
git log --oneline --grep="test.*add\|failing"
git log --oneline --grep="feat.*implement\|pass"
git log --oneline --grep="refactor"

# Measure test-to-code ratio
git log --stat | grep -E "\+.*test.*rs|\+.*\.rs" | wc -l
```

## Continuous Integration

### Pre-commit Hooks

Ensure TDD discipline with automated checks:

```toml
# .pre-commit-config.yaml
repos:
  - repo: local
    hooks:
      - id: check-test-coverage
        name: Ensure adequate test coverage
        entry: cargo tarpaulin --fail-under 80
        language: system
        pass_filenames: false

      - id: verify-test-first
        name: Check for test-first development
        entry: ./scripts/verify-test-first.sh
        language: script
        files: '\.rs$'
```

### CI Pipeline

```yaml
# .github/workflows/tdd-checks.yml
name: TDD Quality Checks

on: [push, pull_request]

jobs:
  test-quality:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Run tests
        run: cargo test

      - name: Check test coverage
        run: cargo tarpaulin --fail-under 80

      - name: Verify commit patterns show TDD
        run: ./scripts/check-tdd-commits.sh
```

## Summary

TDD in Union Square follows these core principles:

1. **Red**: Write failing tests first that describe desired behavior
2. **Green**: Implement minimal code to make tests pass
3. **Refactor**: Improve design while keeping tests green
4. **Commit**: Show the cycle clearly in git history
5. **Fast Feedback**: Keep the test suite fast (< 1s for unit tests)
6. **Type Safety**: Use types to reduce test burden and focus on behavior
7. **Continuous**: Make TDD part of the CI/CD pipeline

The goal is not just to have tests, but to use tests to drive better design, provide living documentation, and enable confident refactoring.

## References

- [Test Driven Development: By Example](https://www.amazon.com/Test-Driven-Development-Kent-Beck/dp/0321146530) - Kent Beck
- [Growing Object-Oriented Software, Guided by Tests](https://www.amazon.com/Growing-Object-Oriented-Software-Guided-Tests/dp/0321503627) - Freeman & Pryce
- [Working Effectively with Legacy Code](https://www.amazon.com/Working-Effectively-Legacy-Michael-Feathers/dp/0131177052) - Michael Feathers
- [Union Square ADRs](../adr/) - Architecture Decision Records documenting design choices
