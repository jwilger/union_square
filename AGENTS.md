# AGENTS.md

This file is the primary Codex instruction file for Union Square.

## Critical Rules

1. Never use `--no-verify`.
2. Never push directly to `main` or `master`.
3. Never use destructive git commands such as `git reset --hard`, `git clean`, or `git rebase` unless the user explicitly asks for that exact operation.
4. Track all implementation work through GitHub issues.
5. Use strict outside-in BDD/TDD for behavior changes.
6. Follow `docs/architecture/ARCHITECTURE.md` and `docs/guardrails/`.
7. Stop and surface the blocker instead of bypassing tests, hooks, architecture checks, or review feedback.

## Standard Workflow

1. Start from an assigned GitHub issue.
2. Create or checkout an issue branch using `gh issue develop`.
3. Write a behavior spec with `just spec ISSUE=<number>` before tests or production edits.
4. Create the `us-agent` ledger with `just agent start-issue <number>`.
5. Record branch creation with `just agent record-branch <number>`.
6. Record the valid spec with `just agent record-spec <number>`.
7. Write the test list, then record it with `just agent record-test-list <number>`.
8. Write exactly one failing outside-in test for the next behavior example.
9. Run the narrow test command and record the red result with `just agent record-red <number>`.
10. Implement the minimum code needed to pass that test.
11. Record green with `just agent record-green <number>`.
12. Run `just test-adversary ISSUE=<number>` for behavior-changing Rust code and record it with `just agent record-test-adversary <number>`.
13. Run `just fitness` and record it with `just agent record-fitness <number>`.
14. Refactor only while tests are green, then record it with `just agent record-refactor <number>`.
15. Run relevant expert review agents and record it with `just agent record-review <number>`.
16. Run `just ci-rust`, then `just agent ready-to-commit <number>`.
17. Commit with Conventional Commits. Do not bypass hooks.
18. Push the branch, open or update the PR after `just agent ready-to-pr <number>`, and address CodeRabbit feedback normally.

## Architecture Source Of Truth

- Current architecture: `docs/architecture/ARCHITECTURE.md`
- Guardrails: `docs/guardrails/`
- Historical rationale: active ADRs live in `adr/`. Use `docs/adr/template.md`
  when creating new ADRs under `adr/`.

ADRs explain why decisions were made. They are not the active implementation manual. If an ADR changes current architecture, update `docs/architecture/ARCHITECTURE.md` in the same PR.

## Commands

Use `just` as the canonical local command surface:

- `just fmt`
- `just fmt-check`
- `just clippy`
- `just check`
- `just test`
- `just test-doc`
- `just ast-grep`
- `just ast-grep-branch`
- `just spec ISSUE=<number>`
- `just test-adversary ISSUE=<number>`
- `just fitness`
- `just ci-harness`
- `just ci-rust`
- `just ci-full`
- `just db-up`

Use `cargo add` for Rust dependencies. Keep EventCore crate versions aligned.

## TDD Discipline

No production behavior change is allowed without a failing test first. Tests should begin at the external boundary when the behavior is user-visible, then move inward only as needed. Every red test must trace to a behavior-spec example.

Red-green-refactor means:

1. Red: one failing test for one behavior.
2. Green: minimum implementation to pass.
3. Refactor: improve structure with tests green.

## PR Hygiene

PRs must be focused on one concern. Do not mix unrelated refactors with behavior changes. Use CodeRabbit feedback as part of the normal loop: deterministic local gates catch enforceable issues; CodeRabbit can catch qualitative review issues that are cheaper to fix after review.
