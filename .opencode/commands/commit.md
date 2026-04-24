---
description: Stage and prepare a commit
agent: build
---

Stage all changes and draft a Conventional Commit message.

CRITICAL RULES:
- DO NOT use `--no-verify` under any circumstances
- Follow Conventional Commits format: `<type>[scope]: <description>`
- Explain the 'why' in the message, not just the 'what'
- Common types: `feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `chore:`
- Breaking changes: add `!` after type

Before committing, verify:
- `cargo fmt` passes
- `cargo clippy --workspace --all-targets -- -D warnings` passes
- `cargo test --workspace` passes

If any check fails, fix the issues first. Ask for help if you cannot fix them.
