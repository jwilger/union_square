set shell := ["bash", "-cu"]

fmt:
    cargo fmt --all --
    cargo fmt --manifest-path tools/us-spec/Cargo.toml --
    cargo fmt --manifest-path tools/us-agent/Cargo.toml --
    cargo fmt --manifest-path tools/us-fitness/Cargo.toml --
    cargo fmt --manifest-path tools/us-test-adversary/Cargo.toml --

fmt-check:
    cargo fmt --all -- --check
    cargo fmt --manifest-path tools/us-spec/Cargo.toml -- --check
    cargo fmt --manifest-path tools/us-agent/Cargo.toml -- --check
    cargo fmt --manifest-path tools/us-fitness/Cargo.toml -- --check
    cargo fmt --manifest-path tools/us-test-adversary/Cargo.toml -- --check

clippy:
    cargo clippy --workspace --all-targets --all-features -- -D warnings

check:
    cargo check --all-targets

check-tools:
    cargo check --manifest-path tools/us-spec/Cargo.toml
    cargo check --manifest-path tools/us-agent/Cargo.toml
    cargo check --manifest-path tools/us-fitness/Cargo.toml
    cargo check --manifest-path tools/us-test-adversary/Cargo.toml

clippy-tools:
    cargo clippy --manifest-path tools/us-spec/Cargo.toml -- -D warnings
    cargo clippy --manifest-path tools/us-agent/Cargo.toml -- -D warnings
    cargo clippy --manifest-path tools/us-fitness/Cargo.toml -- -D warnings
    cargo clippy --manifest-path tools/us-test-adversary/Cargo.toml -- -D warnings

build:
    cargo build --workspace

build-release:
    cargo build --workspace --release

test *ARGS:
    if command -v cargo-nextest >/dev/null 2>&1; then cargo nextest run --workspace {{ARGS}}; else cargo test --workspace {{ARGS}}; fi

test-doc:
    cargo test --doc

test-tools:
    cargo test --manifest-path tools/us-spec/Cargo.toml
    cargo test --manifest-path tools/us-agent/Cargo.toml
    cargo test --manifest-path tools/us-fitness/Cargo.toml
    cargo test --manifest-path tools/us-test-adversary/Cargo.toml

test-hooks:
    .codex/hooks/test-safety.sh
    .codex/hooks/test-hooks.sh

legacy-harness-check:
    tools/check-legacy-harness.sh

coverage:
    cargo llvm-cov --workspace --lcov --output-path lcov.info

audit:
    cargo audit

deny:
    cargo deny check advisories bans licenses sources

actions-security:
    actionlint
    zizmor --min-severity high .

ci-security: audit deny actions-security

ast-grep:
    files="$(git diff --name-only --diff-filter=ACMR; git diff --cached --name-only --diff-filter=ACMR)" \
      && files="$(printf '%s\n' "$files" | sort -u | grep -E '\.rs$' | grep -Ev '^(tests/|benches/|tools/ast-grep/rule-tests/)' || true)" \
      && if [ -n "$files" ]; then ast-grep scan $files; else echo "ast-grep skipped: no changed Rust source files"; fi

ast-grep-branch:
    base_ref="${US_AST_GREP_BASE_REF:-${US_FITNESS_BASE_REF:-origin/main}}" \
      && if git rev-parse --verify "$base_ref" >/dev/null 2>&1; then files="$(git diff --name-only --diff-filter=ACMR "$base_ref"...HEAD)"; else files="$(git diff --name-only --diff-filter=ACMR; git diff --cached --name-only --diff-filter=ACMR)"; fi \
      && files="$(printf '%s\n' "$files" | sort -u | grep -E '\.rs$' | grep -Ev '^(tests/|benches/|tools/ast-grep/rule-tests/)' || true)" \
      && if [ -n "$files" ]; then ast-grep scan $files; else echo "ast-grep skipped: no changed Rust source files"; fi

ast-grep-test:
    ast-grep test --skip-snapshot-tests

bench-quick:
    cargo test --test benchmark_validation
    cargo bench --bench proxy_performance -- --quick --noplot

spec ISSUE:
    issue="{{ISSUE}}"; issue="${issue#ISSUE=}"; cargo run --manifest-path tools/us-spec/Cargo.toml -- check --issue "$issue"

fitness:
    cargo run --manifest-path tools/us-fitness/Cargo.toml -- check --repo .

test-adversary ISSUE:
    issue="{{ISSUE}}"; issue="${issue#ISSUE=}"; cargo run --manifest-path tools/us-test-adversary/Cargo.toml -- check --issue "$issue"

agent *ARGS:
    cargo run --manifest-path tools/us-agent/Cargo.toml -- {{ARGS}}

db-up:
    docker compose up -d postgres postgres-test

ci-rust: fmt-check clippy clippy-tools check check-tools test-tools test-hooks test test-doc ast-grep-branch ast-grep-test fitness

ci-harness: check-tools clippy-tools test-tools test-hooks legacy-harness-check ast-grep-branch ast-grep-test fitness

ci-full: ci-rust ci-security build build-release bench-quick

ci: ci-full
