# Justfile — Union Square task runner
# Install just: cargo install just
# This file is the source of truth for CI and local checks.

set fallback := true

# Default recipe — show available commands
default:
    @just --list

# ─── Core CI Contract ────────────────────────────────────────────────────────

# Run the complete local CI contract (format, lint, check, test, security, architecture)
ci: lint test type-check architecture-lint security-lint

# ─── Lint ────────────────────────────────────────────────────────────────────

# Format code
fmt:
    cargo fmt --all

# Check formatting without modifying files
fmt-check:
    cargo fmt --all -- --check

# Run clippy with standard warnings
clippy:
    cargo clippy --workspace --all-targets -- -D warnings

# Run clippy with architecture-enforced denies.
# Baseline debt: existing code has unwrap/expect violations.
# This is run in CI as an informational check until the baseline is clean.
clippy-architecture:
    cargo clippy --workspace --all-targets -- \
        -D warnings \
        -D clippy::unwrap_used \
        -D clippy::expect_used \
        -D clippy::panic \
        -D clippy::todo \
        -D clippy::unimplemented \
        -D clippy::print_stdout \
        -D clippy::print_stderr \
        -D clippy::dbg_macro

# Type check all targets
type-check:
    cargo check --all-targets

# Run all lint steps
lint: fmt-check clippy type-check

# ─── Test ────────────────────────────────────────────────────────────────────

# Run unit and integration tests
test:
    cargo test --workspace

# Run tests with nextest (preferred when available)
nextest:
    cargo nextest run --workspace

# Run doctests
doctest:
    cargo test --doc

# ─── Architecture Lint ───────────────────────────────────────────────────────

# Run ast-grep architecture checks
ast-grep:
    ast-grep scan --globs '!tests/**/*.rs' --globs '!benches/**/*.rs'

# Run ast-grep rule tests
ast-grep-test:
    ast-grep test

# Run all architecture lint steps
architecture-lint: ast-grep ast-grep-test

# ─── Security Lint ───────────────────────────────────────────────────────────

# Run cargo-deny license, bans, and sources checks
# Advisories are handled by cargo-audit to avoid duplicating advisory DB logic.
cargo-deny:
    cargo deny check licenses bans sources

# Run cargo-audit for known vulnerabilities
cargo-audit:
    cargo audit

# Lint GitHub Actions with zizmor
# --no-exit-codes ensures the command succeeds while still reporting findings.
# Remove this flag once all baseline findings are resolved.
zizmor:
    zizmor --no-exit-codes .github/workflows/

# Lint GitHub Actions with actionlint
actionlint:
    actionlint .github/workflows/*.yml

# Run all security lint steps
security-lint: cargo-deny zizmor actionlint

# ─── Performance ─────────────────────────────────────────────────────────────

# Run performance validation tests
perf-test:
    cargo test --test benchmark_validation

# Run quick benchmarks
perf-bench:
    cargo bench --bench proxy_performance -- --quick --noplot

# ─── Documentation ───────────────────────────────────────────────────────────

# Build documentation
docs:
    cargo doc --workspace --no-deps

# Build and open documentation
docs-open:
    cargo doc --workspace --no-deps --open

# ─── Database ────────────────────────────────────────────────────────────────

# Run database migrations
migrate:
    sqlx migrate run

# ─── Development ─────────────────────────────────────────────────────────────

# Install development dependencies and hooks
dev-setup:
    lefthook install

# Run pre-commit hooks manually
pre-commit:
    lefthook run pre-commit

# Run pre-push hooks manually
pre-push:
    lefthook run pre-push

# Clean build artifacts
clean:
    cargo clean

# Build release binary
build-release:
    cargo build --release
