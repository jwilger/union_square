# Test-Driven Development Adoption Plan

## Current State Analysis

Based on our analysis of the codebase, we've identified significant gaps in TDD practices:

### Key Findings
1. **Middleware Tests**: 25 TODO placeholders in `src/proxy/middleware_tests.rs`
   - Request ID middleware tests (3 tests)
   - Authentication middleware tests (5 tests)
   - Error handling middleware tests (3 tests)
   - Combined middleware tests (2 tests)
   - Logging middleware tests (2 tests)
   - Rate limiting middleware tests (2 tests)

2. **Streaming Tests**: 3 TODO placeholders in `src/proxy/tests.rs`
   - Ring buffer capture during streaming
   - Large response chunking
   - Streaming timeout handling

3. **Integration Tests**: Various incomplete test scenarios
   - Load tests marked as `#[ignore]`
   - Missing characterization tests for existing behavior

4. **Commit History**: No evidence of red-green-refactor cycle in commits

## TDD Adoption Strategy

### 1. Immediate Actions

#### A. Establish TDD Workflow
Every new feature or bug fix MUST follow this workflow:

1. **RED**: Write a failing test first
   ```rust
   #[test]
   fn test_feature_behavior() {
       // Arrange
       let input = create_test_input();

       // Act
       let result = feature_under_test(input);

       // Assert
       assert_eq!(result, expected_output);
       // This MUST fail initially
   }
   ```

2. **GREEN**: Write minimal code to pass
   - Only implement what's needed to make the test pass
   - Resist over-engineering

3. **REFACTOR**: Improve the design
   - Clean up duplication
   - Improve naming
   - Extract functions/modules
   - Run tests after each change

#### B. Commit Message Patterns
Adopt commit patterns that show TDD cycle:

```
test(component): add failing test for [feature]
feat(component): implement [feature] to pass test
refactor(component): clean up [feature] implementation
```

### 2. Implementation Priority

Based on criticality and dependencies:

#### Phase 1: Core Middleware (Week 1)
1. **Request ID Middleware** (Critical for tracing)
   - Test ID generation
   - Test ID preservation
   - Test propagation through stack

2. **Authentication Middleware** (Security critical)
   - Test valid API key acceptance
   - Test missing key rejection
   - Test invalid key rejection
   - Test malformed header handling
   - Test bypass paths

3. **Error Handling Middleware** (User experience)
   - Test error formatting
   - Test panic recovery
   - Test error correlation with request ID

#### Phase 2: Supporting Middleware (Week 2)
4. **Logging Middleware**
   - Test request logging format
   - Test response logging with timing

5. **Rate Limiting Middleware**
   - Test per-key rate limiting
   - Test rate limit headers

6. **Combined Middleware Stack**
   - Test middleware ordering
   - Test error propagation

#### Phase 3: Advanced Features (Week 3)
7. **Streaming Tests**
   - Test ring buffer capture
   - Test large response chunking
   - Test timeout handling

### 3. TDD Best Practices for This Project

#### Test Structure
```rust
#[cfg(test)]
mod tests {
    use super::*;

    mod feature_name {
        use super::*;

        // Group related tests together
        #[test]
        fn test_happy_path() { }

        #[test]
        fn test_edge_case() { }

        #[test]
        fn test_error_scenario() { }
    }
}
```

#### Test Naming Convention
- Use descriptive names: `test_request_id_generation_creates_uuid_v7`
- Include the scenario: `test_auth_rejects_missing_api_key`
- Be specific about expectations: `test_error_response_includes_correlation_id`

#### Test Data Builders
Create builders for complex test data:
```rust
struct TestRequestBuilder {
    method: String,
    uri: String,
    headers: Vec<(String, String)>,
}

impl TestRequestBuilder {
    fn new() -> Self { /* defaults */ }
    fn with_auth(mut self, key: &str) -> Self { /* ... */ }
    fn build(self) -> Request<Body> { /* ... */ }
}
```

### 4. Measuring TDD Adoption

#### Metrics to Track
1. **Test-First Commits**: Ratio of test commits before implementation
2. **Test Coverage**: Maintain > 80% for new code
3. **Test Execution Time**: Keep under 5 seconds for unit tests
4. **Defect Rate**: Track bugs found in TDD vs non-TDD code

#### Git Hooks
Set up pre-commit hooks to enforce:
- Tests must pass
- Coverage thresholds
- Commit message format

### 5. Team Guidelines

#### When Writing Tests
1. **Start with the assertion**: What should be true?
2. **Work backwards**: What setup is needed?
3. **Keep tests focused**: One behavior per test
4. **Use descriptive failures**: Custom error messages

#### Code Review Checklist
- [ ] Does the PR include tests written before implementation?
- [ ] Do commit messages show red-green-refactor cycle?
- [ ] Are tests testing behavior, not implementation?
- [ ] Do tests cover edge cases?
- [ ] Are tests independent and deterministic?

### 6. Migration Strategy for Existing Code

For code with TODO tests:
1. **Characterization Tests First**: Document current behavior
2. **Fill TODO Tests**: One at a time, following TDD
3. **Refactor if Needed**: Only after tests are green

### 7. Common TDD Patterns for This Project

#### Testing Async Middleware
```rust
#[tokio::test]
async fn test_middleware_behavior() {
    // Arrange
    let middleware = create_middleware();
    let request = TestRequestBuilder::new()
        .with_path("/test")
        .build();

    // Act
    let response = middleware.call(request).await.unwrap();

    // Assert
    assert_eq!(response.status(), StatusCode::OK);
}
```

#### Testing Error Scenarios
```rust
#[test]
fn test_error_handling() {
    // Arrange
    let service = create_failing_service();

    // Act
    let result = service.process(invalid_input);

    // Assert
    assert!(matches!(result, Err(ProxyError::InvalidRequest(_))));
}
```

## Implementation Timeline

**Week 1**:
- Set up git hooks and tooling
- Complete Request ID and Auth middleware tests
- Establish team practices

**Week 2**:
- Complete remaining middleware tests
- Begin streaming tests
- First TDD feature from scratch

**Week 3**:
- Complete all TODO tests
- Measure adoption metrics
- Refine process based on learnings

## Success Criteria

1. **100% of TODO tests implemented** using TDD
2. **All new features** follow red-green-refactor
3. **Commit history** clearly shows TDD cycle
4. **Test suite runs** in < 10 seconds
5. **Team confidence** in changing code due to tests

## Resources

- [Growing Object-Oriented Software, Guided by Tests](http://www.growing-object-oriented-software.com/)
- [Test Driven Development: By Example](https://www.amazon.com/Test-Driven-Development-Kent-Beck/dp/0321146530)
- [Working Effectively with Legacy Code](https://www.amazon.com/Working-Effectively-Legacy-Michael-Feathers/dp/0131177052)
