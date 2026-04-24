# Rule: CI Flake Defined Invocations

All CI commands must be defined in version-controlled configuration, not embedded in CI YAML.

## Why

- CI commands should be runnable locally for debugging
- Centralized command definitions prevent drift between local and CI behavior
- Changes to checks are code-reviewed as part of normal PR flow

## Pattern

Define commands in a task runner (`Justfile`) or script:

```justfile
# Justfile
test:
    cargo nextest run --workspace

check:
    cargo fmt --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo check --all-targets

lint:
    cargo fmt
    cargo clippy --workspace --all-targets -- -D warnings
    ast-grep scan

ci: lint test
```

Then invoke these from CI:

```yaml
# .github/workflows/ci.yml
- name: Check
  run: just ci
```

## Rules

1. **No inline `cargo` commands in CI YAML** — Always call through the task runner
2. **Local and CI use the same commands** — `just ci` should pass locally before pushing
3. **Document commands** — The `Justfile` is the source of truth for how to build, test, and lint

## Enforcement

- CI review by `continuous-delivery-architect`
