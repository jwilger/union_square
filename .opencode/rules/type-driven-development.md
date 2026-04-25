# Rule: Type-Driven Development

This project follows strict type-driven development principles.

## Core Principles

1. **Types come first**: Model the domain, make illegal states unrepresentable, then implement
2. **Parse, don't validate**: Transform unstructured data into structured data at system boundaries ONLY
   - Validation should be encoded in the type system to the maximum extent possible
   - Use smart constructors with validation only at the system's input boundaries
   - Once data is parsed into domain types, those types guarantee validity throughout the system
3. **No primitive obsession**: Use newtypes for all domain concepts
4. **Functional Core, Imperative Shell**: Pure functions at the heart, side effects at the edges
5. **Total functions**: Every function should handle all cases explicitly

## Newtype Pattern

Use `nutype` for validated newtypes:

```rust
use nutype::nutype;

#[nutype(
    validate(len_char_min = 1, len_char_max = 256),
    derive(Debug, Clone, Serialize, Deserialize, Display)
)]
pub struct UserName(String);

#[nutype(
    validate(greater_or_equal = 0),
    derive(Debug, Clone, Copy, Serialize, Deserialize)
)]
pub struct MoneyAmount(f64);
```

## Smart Constructors

For types that need custom validation not covered by `nutype`:

```rust
#[derive(Debug, Clone)]
pub struct StreamId(String);

impl StreamId {
    pub fn new(id: impl Into<String>) -> Result<Self, DomainError> {
        let id = id.into();
        if id.is_empty() {
            return Err(DomainError::InvalidStreamId(id));
        }
        Ok(Self(id))
    }
}
```

## Algebraic Data Types

Use enums to model state machines and mutually exclusive states:

```rust
#[derive(Debug, Clone)]
pub enum SessionStatus {
    Active { started_at: DateTime<Utc> },
    Paused { paused_at: DateTime<Utc>, reason: String },
    Completed { completed_at: DateTime<Utc>, summary: String },
}
```

## Enforcement

- Code review by `type-theory-reviewer` and `type-driven-development-expert`
- `cargo clippy` linting
