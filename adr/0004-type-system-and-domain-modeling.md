# Type System and Domain Modeling

- Status: accepted
- Deciders: John Wilger, Technical Architecture Team
- Date: 2025-07-14

## Context and Problem Statement

Union Square requires a robust type system that makes illegal states unrepresentable, validates data at system boundaries, and provides compile-time guarantees about correctness. The domain model must accurately represent LLM interactions, sessions, test cases, and analytics while maintaining flexibility for different provider APIs and evolving requirements.

## Decision Drivers

- **Type Safety**: Leverage Rust's type system to prevent runtime errors
- **Domain Clarity**: Types should clearly express business concepts
- **Validation**: Parse, don't validate - ensure data validity through types
- **Performance**: Zero-cost abstractions where possible
- **Extensibility**: Support adding new providers and features
- **Developer Experience**: Types should guide correct usage

## Considered Options

- **Option 1**: Simple type aliases with runtime validation
- **Option 2**: Newtype pattern with smart constructors
- **Option 3**: Full Domain-Driven Design with aggregates
- **Option 4**: Type-state pattern for workflow modeling

## Decision Outcome

Chosen option: **"Newtype pattern with smart constructors"** combined with **"Type-state pattern for workflows"** where appropriate. This provides maximum type safety while keeping the implementation pragmatic and performant.

### Core Domain Types

```rust
// Using nutype for compile-time validation
#[nutype(
    validate(not_empty, regex = "^[a-zA-Z0-9-]+$"),
    derive(Debug, Clone, PartialEq, Eq, Hash, AsRef, Deref, Serialize, Deserialize)
)]
pub struct SessionId(String);

#[nutype(
    validate(not_empty, max_len = 100),
    derive(Debug, Clone, PartialEq, Eq, Hash, AsRef, Deref, Serialize, Deserialize)
)]
pub struct ApplicationName(String);

#[nutype(
    validate(greater = 0),
    derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)
)]
pub struct TokenCount(u32);

// Domain events using enums
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionEvent {
    Started {
        session_id: SessionId,
        application_id: ApplicationId,
        metadata: SessionMetadata,
        occurred_at: DateTime<Utc>,
    },
    RequestRecorded {
        session_id: SessionId,
        provider: Provider,
        model: ModelIdentifier,
        request: LlmRequest,
        occurred_at: DateTime<Utc>,
    },
    ResponseRecorded {
        session_id: SessionId,
        response: LlmResponse,
        latency_ms: LatencyMs,
        tokens_used: TokenUsage,
        occurred_at: DateTime<Utc>,
    },
    ErrorOccurred {
        session_id: SessionId,
        error: ProxyError,
        occurred_at: DateTime<Utc>,
    },
}

// Type-state pattern for test execution
pub struct TestCase<State> {
    id: TestCaseId,
    name: TestCaseName,
    expected_behavior: ExpectedBehavior,
    _state: PhantomData<State>,
}

// Test states
pub struct Draft;
pub struct Ready;
pub struct Running;
pub struct Completed;

impl TestCase<Draft> {
    pub fn finalize(self) -> Result<TestCase<Ready>, ValidationError> {
        // Validation logic
        Ok(TestCase {
            id: self.id,
            name: self.name,
            expected_behavior: self.expected_behavior,
            _state: PhantomData,
        })
    }
}

impl TestCase<Ready> {
    pub fn execute(self) -> TestCase<Running> {
        TestCase {
            id: self.id,
            name: self.name,
            expected_behavior: self.expected_behavior,
            _state: PhantomData,
        }
    }
}
```

### Provider Abstraction

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Provider {
    OpenAI,
    Anthropic,
    Bedrock,
    VertexAI,
}

// Sealed trait pattern for provider-specific behavior
pub trait ProviderApi: private::Sealed {
    type Request: Serialize + DeserializeOwned;
    type Response: Serialize + DeserializeOwned;
    type StreamChunk: Serialize + DeserializeOwned;

    fn endpoint_pattern(&self) -> &'static str;
    fn parse_model(&self, request: &Self::Request) -> ModelIdentifier;
}

mod private {
    pub trait Sealed {}
}
```

### Error Modeling

```rust
#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    #[error("Invalid session ID: {0}")]
    InvalidSessionId(String),

    #[error("Rate limit exceeded for {provider:?}")]
    RateLimitExceeded { provider: Provider },

    #[error("Test assertion failed: {message}")]
    TestAssertionFailed { message: String },
}

// Result type alias for convenience
pub type DomainResult<T> = Result<T, DomainError>;
```

### Positive Consequences

- **Compile-time Safety**: Invalid states cannot be constructed
- **Self-documenting**: Types clearly express intent
- **Refactoring Safety**: Type changes catch all usage sites
- **Performance**: Newtypes have zero runtime cost
- **Serialization**: All types can be serialized/deserialized

### Negative Consequences

- **Boilerplate**: More type definitions upfront
- **Learning Curve**: Developers need to understand the patterns
- **Conversion Overhead**: Need to convert at boundaries

## Pros and Cons of the Options

### Option 1: Simple type aliases with runtime validation

Using type aliases like `type SessionId = String`.

- Good, because minimal boilerplate
- Good, because familiar to most developers
- Bad, because no compile-time guarantees
- Bad, because validation scattered throughout code
- Bad, because easy to mix up different string types

### Option 2: Newtype pattern with smart constructors

Wrapping primitives in structs with validation.

- Good, because compile-time type safety
- Good, because validation at construction
- Good, because zero runtime cost
- Good, because works well with serde
- Bad, because more initial setup
- Bad, because need conversion at boundaries

### Option 3: Full Domain-Driven Design

Complex aggregates, entities, and value objects.

- Good, because rich domain modeling
- Good, because encapsulates business rules
- Bad, because significant complexity
- Bad, because may be overkill for proxy service
- Bad, because performance overhead

### Option 4: Type-state pattern for workflows

Using phantom types to encode state transitions.

- Good, because illegal state transitions impossible
- Good, because self-documenting workflows
- Bad, because complex type signatures
- Bad, because unfamiliar to many developers

## Implementation Guidelines

1. **Use nutype** for all domain primitives requiring validation
2. **Smart constructors** return `Result<T, ValidationError>`
3. **Parse at boundaries** - validate once, use everywhere
4. **Rich error types** using thiserror
5. **Serde integration** for all public types
6. **Builder pattern** for complex type construction

## Links

- Influenced by [ADR-0001](0001-overall-architecture-pattern.md) - Types support functional core
- Influences [ADR-0005](0005-testing-strategy.md) - Types enable property testing
- Related to [ADR-0003](0003-proxy-implementation.md) - Provider traits
