# Rule: Prefer Borrows Over Ownership

When passing data to functions, prefer borrowing over taking ownership unless the function genuinely needs to consume the value.

## Guidelines

1. **Prefer `&T` over `T`** for function parameters when the function only needs to read
2. **Prefer `&mut T` over `T`** when the function needs to mutate but not consume
3. **Take ownership only when**:
   - The value is being moved into a struct field
   - The value is being sent to another thread
   - The function conceptually consumes the value (e.g., `into_inner`)

## Why

- Reduces unnecessary clones
- Makes APIs more flexible for callers
- Signals intent clearly through the type system
- Follows Rust idioms and zero-cost abstraction principles

## Examples

### Bad
```rust
fn process_name(name: String) -> String {
    name.to_uppercase()
}
```

### Good
```rust
fn process_name(name: &str) -> String {
    name.to_uppercase()
}
```

### Bad
```rust
fn validate_user(user: User) -> Result<(), Error> {
    // only reads fields
}
```

### Good
```rust
fn validate_user(user: &User) -> Result<(), Error> {
    // only reads fields
}
```

## Enforcement

- `ast-grep` rule: `prefer-borrows.yml`
- Code review by `rust-type-system-expert`
