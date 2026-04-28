---
name: feature-delivery
description: Drive one Union Square GitHub issue through the Codex BDD/TDD workflow.
license: MIT
---

Use this skill for behavior-changing implementation work.

Workflow:
- Read `AGENTS.md`, `docs/architecture/ARCHITECTURE.md`, and relevant docs under `docs/guardrails/`.
- Create or validate `.codex/specs/issue-<number>.yaml`.
- Use `just agent` to maintain the issue ledger.
- Write one red outside-in test at a time.
- Run `just test-adversary ISSUE=<number>` and `just fitness` before commit readiness.
- Export PR evidence with `just agent export-pr-summary <number>`.
