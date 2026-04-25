# Rule: Workspace Dependencies Consistency

If Union Square becomes a workspace, all workspace crates must use consistent dependency versions.

## Why

- Multiple versions of the same crate cause compilation issues and binary bloat
- Inconsistent versions make debugging harder
- Security patches must be applied consistently

## Pattern

In the workspace `Cargo.toml`:

```toml
[workspace.dependencies]
tokio = { version = "1.52", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
eventcore = "0.7.1"
```

In member crates:

```toml
[dependencies]
tokio = { workspace = true }
serde = { workspace = true }
```

## Rules

1. **All shared dependencies use `workspace = true`**
2. **Version bumps happen in the workspace root** — Never in individual crates
3. **Use `cargo autoinherit`** — To automatically convert crates to workspace dependencies

## Current Status

Union Square is currently a single crate. If it becomes a workspace, this rule applies immediately.

## Enforcement

- `cargo autoinherit` in lefthook pre-commit
- Code review by `rust-type-system-expert`
