# 0022. Type-Safe Request Lifecycle State Machine

Date: 2025-07-28

## Status

Accepted

## Context

The initial implementation used boolean flags to track request state:
- `request_received: bool`
- `request_forwarded: bool`
- `response_received: bool`

This approach had several problems:
- Invalid state combinations were possible (e.g., response received before request forwarded)
- State transitions were not explicit
- Business rules were enforced through runtime checks
- The true state space was not clear from the type signature

The type-driven-development-expert agent advocated for making illegal states unrepresentable at compile time.

## Decision

We will implement a type-safe state machine using Rust's algebraic data types:

```rust
enum RequestLifecycle {
    NotStarted,
    Received {
        received_at: DateTime<Utc>,
    },
    Forwarded {
        received_at: DateTime<Utc>,
        forwarded_at: DateTime<Utc>,
    },
    ResponseReceived {
        received_at: DateTime<Utc>,
        forwarded_at: DateTime<Utc>,
        response_at: DateTime<Utc>,
    },
    Completed {
        received_at: DateTime<Utc>,
        forwarded_at: DateTime<Utc>,
        response_at: DateTime<Utc>,
        completed_at: DateTime<Utc>,
    },
    Failed {
        state: Box<RequestLifecycle>,
        error: String,
        failed_at: DateTime<Utc>,
    },
}
```

Each variant carries only the data relevant to that state, and transitions are validated at compile time.

## Consequences

### Positive

- **Compile-time guarantees**: Invalid states are impossible to represent
- **Self-documenting**: The type clearly shows all possible states and transitions
- **Explicit transitions**: State changes must go through validated transition methods
- **Type-safe data access**: Each state provides only its relevant data
- **Better error messages**: Invalid transitions are caught at compile time

### Negative

- **More verbose**: Requires pattern matching for state access
- **Memory overhead**: Each variant stores all previous timestamps
- **Learning curve**: Developers need to understand algebraic data types

### Trade-offs Accepted

The verbosity and slight memory overhead are acceptable costs for the significant improvement in type safety and code clarity. The compile-time guarantees prevent entire classes of bugs.

## References

- "Making Illegal States Unrepresentable" - Yaron Minsky
- Type-driven development principles
- Rust Book chapter on Enums
