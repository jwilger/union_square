---
name: pr-feedback
description: Process CodeRabbit and human PR feedback without bypassing project gates.
license: MIT
---

# PR Feedback

Use this skill when addressing PR review comments.

Rules:
- Classify every comment before editing.
- Follow `docs/guardrails/review-feedback-protocol.md`.
- Add or update tests for behavior-changing feedback.
- Re-run the relevant `just` recipes and update `us-agent` evidence.
- Reply with what changed or why the suggestion was not taken.
