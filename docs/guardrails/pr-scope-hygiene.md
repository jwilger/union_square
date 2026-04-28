# Rule: PR Scope Hygiene

Pull requests must be focused, atomic, and contain only changes related to a single concern.

## Rules

1. **One concern per PR** — A PR should address exactly one issue or feature
2. **No unrelated refactoring** — If you notice code that needs cleanup while working on a feature, create a separate issue and PR
3. **No mixing feature and refactor** — Separate "change behavior" from "change structure"
4. **Include only necessary changes** — Don't commit formatting changes to unrelated files

## PR Size Guidelines

- **Ideal**: < 400 lines of change
- **Acceptable**: < 800 lines of change
- **Requires justification**: > 800 lines of change

If a change is naturally large (e.g., introducing a new major feature):
- Break it into stacked PRs
- Each PR should be reviewable independently
- Use draft PRs for intermediate steps

## Commit Messages Within PRs

Even within a PR, each commit should be atomic:
- `feat: add session recording domain types`
- `test: add property tests for SessionId validation`
- `feat: implement session recording command handler`
- `test: add integration test for session recording`

## Enforcement

- Code review by `design-reviewer`
- PR description must reference exactly one issue (or explain why multiple)
