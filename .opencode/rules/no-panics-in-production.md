# Rule: No Panics in Production Code

Production code must never panic. Use the type system to handle all error cases explicitly.

## Forbidden in Production Code

- `unwrap()`
- `expect()`
- `panic!()`
- `unreachable!()` (unless truly unreachable by type system proof)
- `todo!()` (except in very early spikes with explicit cleanup plans)

## Allowed Only In

- Tests (`#[cfg(test)]` modules)
- Benchmarks (`benches/`)
- Build scripts (`build.rs`)
- Early experiments in `experiments/` directory (with documented cleanup plan)

## What To Use Instead

- `Result<T, E>` for fallible operations
- `Option<T>` for nullable values
- `thiserror` for defining domain-specific error types
- Pattern matching with exhaustive handling
- `?` operator for early returns

## Examples

### Bad
```rust
let value = some_option.unwrap();
let parsed = str.parse::<i32>().expect("must be a number");
```

### Good
```rust
let value = some_option.ok_or(MyError::MissingValue)?;
let parsed = str.parse::<i32>().map_err(MyError::InvalidNumber)?;
```

## Rationale

Panics crash the process. In a proxy service like Union Square, a panic means dropped LLM requests and lost session data. The type system gives us the tools to handle every case gracefully.

## Enforcement

This rule is enforced by:
- `ast-grep` rules scanning for `unwrap`, `expect`, `panic!`
- The `cargo-check-on-edit` plugin running `cargo clippy` with relevant lints
- Code review by the `type-theory-reviewer` agent
