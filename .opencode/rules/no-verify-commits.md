# Rule: Never Use --no-verify

**CRITICAL**: Under no circumstances may you use the `--no-verify` flag when committing code.

## Why

The `--no-verify` flag bypasses pre-commit hooks that enforce:
- Code formatting (`cargo fmt`)
- Linting (`cargo clippy`)
- Tests (`cargo test`)
- Conventional commit format enforcement
- Structural checks (`ast-grep` rules)

Bypassing these checks defeats the purpose of having them and allows broken, unformatted, or non-compliant code into the repository.

## What To Do Instead

If pre-commit hooks fail:
1. Read the error output carefully
2. Fix the identified issues
3. Stage the fixes
4. Run the commit again
5. If you cannot fix the issue, STOP and ask for help

## Enforcement

This rule is enforced by:
- The `no-verify-blocker` plugin (throws at the tool execution layer)
- The `conventional-commit-guard` plugin (validates commit messages)
- Explicit reminder in every agent reply
