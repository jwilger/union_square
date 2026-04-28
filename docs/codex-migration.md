# Codex Harness Migration

The Codex-only migration is tracked under parent issue #210.

## Issue Tree

| Issue | Title | Blocks |
| --- | --- | --- |
| #211 | tooling: add canonical local command surface | #212, #213, #215 |
| #212 | codex: add foundation config and baseline safety hooks | #214, #217 |
| #213 | guardrails: migrate OpenCode rules to tool-neutral paths | #214, #218 |
| #214 | codex: convert expert agents and skills | none |
| #215 | bdd: implement behavior spec tool | #216, #218 |
| #216 | workflow: implement us-agent TDD state ledger | #217, #218, #219 |
| #217 | codex: wire hooks to spec and workflow state | #220 |
| #218 | architecture: implement us-fitness executable checks | #220 |
| #219 | testing: implement targeted test adversary gate | #220 |
| #220 | ci: integrate Codex harness checks and CodeRabbit loop | #221 |
| #221 | cleanup: remove legacy OpenCode and Claude harness | none |

Relationships were created with `gh issue-ext sub add` and `gh issue-ext blocking add`.

## Operating Model

Local tooling catches deterministic failures: workflow state, behavior-spec shape, architecture drift in changed files, and minimum test-quality evidence. CodeRabbit remains the qualitative review loop for design judgment and follow-up fixes.
