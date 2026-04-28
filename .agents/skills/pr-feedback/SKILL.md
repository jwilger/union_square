---
name: pr-feedback
description: Process CodeRabbit and human PR feedback without bypassing project gates.
license: MIT
---

# PR Feedback

Use this skill when addressing PR review comments.

Rules:
- Every comment MUST be classified before editing.
- Review handling MUST follow `docs/guardrails/review-feedback-protocol.md`.
- Behavior-changing feedback MUST add or update tests.
- Relevant `just` recipes MUST be re-run and `us-agent` evidence MUST be updated.
- Replies MUST state what changed or why the suggestion was not taken.
