---
name: pr-review
description: Review a Union Square PR against deterministic checks and architecture guardrails.
license: MIT
---

Use this skill before asking for human review.

Checklist:
- `just ci-rust` passes.
- `just fitness` passes.
- Behavior specs trace examples to tests.
- Changes are scoped to one issue.
- CodeRabbit path guidance in `.coderabbit.yaml` remains applicable.
