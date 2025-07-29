# TDD Workflow Guide

This guide provides step-by-step instructions for practicing Test-Driven Development in the Union Square project.

## The TDD Cycle

```
┌─────────┐
│   RED   │ Write a failing test
└────┬────┘
     │
     ▼
┌─────────┐
│  GREEN  │ Write minimal code to pass
└────┬────┘
     │
     ▼
┌─────────┐
│REFACTOR │ Improve design, keep tests green
└────┬────┘
     │
     └──── Repeat for next requirement
```

## Step-by-Step Workflow

### 1. RED - Write a Failing Test First

**BEFORE writing any production code:**

```rust
#[test]
fn test_new_feature() {
    // This test MUST fail initially
    let result = function_that_does_not_exist_yet();
    assert_eq!(result, expected_value);
}
```

**Run the test to see it fail:**
```bash
cargo test test_new_feature
```

**Verify it fails for the RIGHT reason:**
- Compilation error: Function doesn't exist ✓
- Wrong assertion: Implementation exists but wrong ✗

### 2. GREEN - Make the Test Pass

**Write ONLY enough code to pass:**

```rust
fn function_that_does_not_exist_yet() -> i32 {
    // Simplest implementation that passes
    expected_value
}
```

**Run the test again:**
```bash
cargo test test_new_feature
```

**Rules:**
- No extra features
- No premature optimization
- Just make it green

### 3. REFACTOR - Improve the Design

**With tests green, now improve:**

```rust
fn function_that_does_not_exist_yet() -> i32 {
    // Better implementation
    calculate_value_properly()
}
```

**Run tests after EACH change:**
```bash
cargo test
```

**Safe refactoring includes:**
- Extracting functions
- Improving names
- Removing duplication
- Simplifying logic

## Practical Example: Middleware Test

### Step 1: Write Failing Test

```rust
#[tokio::test]
async fn test_request_id_middleware_generates_uuid_v7() {
    // Arrange
    let request = Request::builder()
        .uri("/test")
        .body(Body::empty())
        .unwrap();

    // Act - This will fail because middleware doesn't exist
    let response = request_id_middleware(request, next).await.unwrap();

    // Assert
    assert!(response.headers().contains_key("x-request-id"));
    let id = response.headers().get("x-request-id").unwrap();
    let uuid = Uuid::parse_str(id.to_str().unwrap()).unwrap();
    assert_eq!(uuid.get_version_num(), 7);
}
```

### Step 2: Minimal Implementation

```rust
pub async fn request_id_middleware(
    request: Request,
    next: Next,
) -> Result<Response, ProxyError> {
    let id = Uuid::now_v7();
    let mut response = next.run(request).await;
    response.headers_mut().insert(
        "x-request-id",
        HeaderValue::from_str(&id.to_string()).unwrap()
    );
    Ok(response)
}
```

### Step 3: Refactor

```rust
pub async fn request_id_middleware(
    mut request: Request,
    next: Next,
) -> Result<Response, ProxyError> {
    // Better: Add to request too
    let request_id = generate_or_preserve_request_id(&request);
    request.headers_mut().insert(X_REQUEST_ID, request_id.clone());

    let mut response = next.run(request).await;
    response.headers_mut().insert(X_REQUEST_ID, request_id);

    Ok(response)
}

fn generate_or_preserve_request_id(request: &Request) -> HeaderValue {
    request.headers()
        .get(X_REQUEST_ID)
        .cloned()
        .unwrap_or_else(|| {
            HeaderValue::from_str(&Uuid::now_v7().to_string())
                .expect("UUID should be valid header")
        })
}
```

## Git Workflow for TDD

### Commit Pattern

Use these commit message patterns:

1. **Red Phase**:
   ```
   test(middleware): add failing test for request ID generation

   - Test expects UUID v7 in x-request-id header
   - Test currently fails with: function not found
   ```

2. **Green Phase**:
   ```
   feat(middleware): implement request ID generation

   - Generates UUID v7 for new requests
   - Adds header to response
   - Makes test_request_id_generation pass
   ```

3. **Refactor Phase**:
   ```
   refactor(middleware): extract request ID logic

   - Extract generate_or_preserve_request_id function
   - Add request ID to both request and response
   - All tests still green
   ```

### Example Git Flow

```bash
# 1. Create feature branch
git checkout -b feature/request-id-middleware

# 2. Write failing test
# ... edit test file ...
git add src/proxy/middleware_tests.rs
git commit -m "test(middleware): add failing test for request ID generation"

# 3. Run test to verify failure
cargo test test_request_id_generation
# Verify it fails as expected

# 4. Implement minimal solution
# ... edit implementation ...
git add src/proxy/middleware.rs
git commit -m "feat(middleware): implement request ID generation"

# 5. Refactor if needed
# ... improve code ...
git add src/proxy/middleware.rs
git commit -m "refactor(middleware): extract request ID helper function"

# 6. Continue with next test...
```

## Testing Patterns

### Pattern 1: Arrange-Act-Assert

```rust
#[test]
fn test_behavior() {
    // Arrange - Set up test data
    let input = TestData::new();
    let expected = ExpectedResult::new();

    // Act - Execute the behavior
    let actual = system_under_test(input);

    // Assert - Verify the outcome
    assert_eq!(actual, expected);
}
```

### Pattern 2: Given-When-Then

```rust
#[test]
fn test_business_rule() {
    // Given - Initial context
    let mut account = Account::new(100.0);

    // When - Action occurs
    let result = account.withdraw(50.0);

    // Then - Expected outcome
    assert!(result.is_ok());
    assert_eq!(account.balance(), 50.0);
}
```

### Pattern 3: Test Data Builders

```rust
struct RequestBuilder {
    method: Method,
    path: String,
    headers: HeaderMap,
}

impl RequestBuilder {
    fn new() -> Self {
        Self {
            method: Method::GET,
            path: "/".to_string(),
            headers: HeaderMap::new(),
        }
    }

    fn post(mut self) -> Self {
        self.method = Method::POST;
        self
    }

    fn path(mut self, path: &str) -> Self {
        self.path = path.to_string();
        self
    }

    fn header(mut self, key: &str, value: &str) -> Self {
        self.headers.insert(key, value.parse().unwrap());
        self
    }

    fn build(self) -> Request<Body> {
        Request::builder()
            .method(self.method)
            .uri(self.path)
            .body(Body::empty())
            .unwrap()
    }
}

// Usage
let request = RequestBuilder::new()
    .post()
    .path("/api/v1/completions")
    .header("Authorization", "Bearer token")
    .build();
```

## Common Pitfalls to Avoid

### 1. Writing Tests After Code
❌ **Wrong:**
```rust
// Implement first
fn calculate_tax(amount: f64) -> f64 {
    amount * 0.08
}

// Then write test
#[test]
fn test_calculate_tax() {
    assert_eq!(calculate_tax(100.0), 8.0);
}
```

✅ **Right:**
```rust
// Write test first
#[test]
fn test_calculate_tax_at_8_percent() {
    assert_eq!(calculate_tax(100.0), 8.0); // Fails
}

// Then implement
fn calculate_tax(amount: f64) -> f64 {
    amount * 0.08
}
```

### 2. Testing Implementation Details
❌ **Wrong:**
```rust
#[test]
fn test_uses_hashmap_internally() {
    let cache = Cache::new();
    // Don't test internal structure
    assert!(cache.internal_map.is_empty());
}
```

✅ **Right:**
```rust
#[test]
fn test_cache_returns_none_when_empty() {
    let cache = Cache::new();
    // Test behavior, not implementation
    assert_eq!(cache.get("key"), None);
}
```

### 3. Writing Multiple Assertions
❌ **Wrong:**
```rust
#[test]
fn test_everything() {
    let user = User::new("Alice", 30);
    assert_eq!(user.name(), "Alice");
    assert_eq!(user.age(), 30);
    assert!(user.is_adult());
    assert!(!user.is_senior());
}
```

✅ **Right:**
```rust
#[test]
fn test_user_has_correct_name() {
    let user = User::new("Alice", 30);
    assert_eq!(user.name(), "Alice");
}

#[test]
fn test_user_is_adult_at_30() {
    let user = User::new("Alice", 30);
    assert!(user.is_adult());
}
```

## IDE Setup for TDD

### VS Code
1. Install "Rust Analyzer" extension
2. Configure test lens:
   ```json
   {
     "rust-analyzer.lens.enable": true,
     "rust-analyzer.lens.run": true
   }
   ```
3. Use Ctrl+Shift+P → "Rust: Run Test" for current test

### IntelliJ IDEA
1. Install Rust plugin
2. Use Ctrl+Shift+F10 to run test under cursor
3. Use Ctrl+Shift+F9 to run previous test

## Continuous Practice

### Daily TDD Kata
Practice with simple exercises:
1. FizzBuzz
2. String Calculator
3. Roman Numerals
4. Bowling Score

### Code Review Checklist
- [ ] Test written before implementation?
- [ ] Test fails for the right reason?
- [ ] Minimal code to pass?
- [ ] Refactoring done with tests green?
- [ ] Good test names?
- [ ] Tests document behavior?

## Resources

### Videos
- [TDD in Rust - A Practical Introduction](https://www.youtube.com/watch?v=_M1ykLJB5u0)
- [Kent Beck TDD Demo](https://www.youtube.com/watch?v=58jGpV2Cg50)

### Books
- "Test Driven Development: By Example" - Kent Beck
- "Growing Object-Oriented Software, Guided by Tests" - Freeman & Pryce

### Practice Sites
- [Exercism Rust Track](https://exercism.org/tracks/rust)
- [Coding Dojo Kata Catalog](https://codingdojo.org/kata/)

## Getting Help

- **Stuck on a test?** Ask: "What's the simplest test that could possibly fail?"
- **Implementation complex?** Ask: "What's the next small step?"
- **Not sure what to test?** Ask: "What behavior am I trying to enable?"

Remember: TDD is a discipline that takes practice. Start small, be patient, and the benefits will come!
