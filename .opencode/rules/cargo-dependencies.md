# Rule: Cargo Dependencies

Manage Rust dependencies carefully to keep the project maintainable and secure.

## Adding Dependencies

**ALWAYS use `cargo add` for latest compatible versions:**
```bash
cargo add tokio --features full
cargo add nutype --features serde
cargo add eventcore eventcore-postgres eventcore-macros
```

## Rules

1. **Prefer fewer, well-maintained crates** — Don't add dependencies for trivial functionality
2. **Pin major versions** — Use `cargo add` which respects semver
3. **Audit before adding** — Check:
   - Maintenance status (last commit, issue response time)
   - License compatibility (MIT/Apache-2.0 preferred)
   - Number of transitive dependencies
4. **Keep EventCore versions in sync** — All `eventcore-*` crates must use the same version

## Dependency Categories

### Core (Required)
- `tokio` — Async runtime
- `axum` / `tower` — Web framework
- `sqlx` — Database access
- `eventcore` / `eventcore-*` — Event sourcing
- `nutype` — Validated newtypes
- `thiserror` — Error definitions

### Avoid Unless Justified
- Heavy serialization frameworks beyond `serde`
- Multiple HTTP clients (stick with `hyper`/`axum` stack)
- ORMs (use `sqlx` query builder)

## Enforcement

- `deny.toml` for license and audit checking
- `cargo-deny` in CI
- Code review by `rust-type-system-expert`
