# Rule: Use thiserror for Errors

All error types in this project must be defined using `thiserror`.

## Why

- `thiserror` generates `Display`, `Error`, and `From` implementations automatically
- It keeps error definitions declarative and maintainable
- It integrates cleanly with the standard library's `Error` trait
- It avoids boilerplate while keeping full control

## Pattern

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DomainError {
    #[error("insufficient funds: required {required}, available {available}")]
    InsufficientFunds { required: Decimal, available: Decimal },

    #[error("invalid stream id: {0}")]
    InvalidStreamId(String),

    #[error(transparent)]
    DatabaseError(#[from] sqlx::Error),
}
```

## Guidelines

1. **One error enum per domain boundary** (e.g., `ProxyError`, `SessionError`)
2. **Use `#[from]` for automatic conversion** from lower-level errors
3. **Include context in display messages** — future debuggers will thank you
4. **Avoid generic `Other(String)` variants** — be specific

## When to use anyhow

`anyhow` is acceptable only at application boundaries (main.rs, HTTP handlers) where you want to collect diverse errors into a single return type for logging or HTTP responses. Domain code should use typed errors.

## Enforcement

- Code review by `type-theory-reviewer`
- `cargo clippy` linting
