# Accepted Replay Schemas

This directory contains the canonical records for event schemas accepted for ongoing historical replay.

No schema is accepted for compatibility obligations unless it has a YAML file at `.opencode/accepted-replay/<schema-id>.yaml` with these fields:

```yaml
schema_id: session-events
accepted_version_or_commit: v1.0.0
accepted_at: "2026-04-25T00:00:00Z"
approver: "username <user@example.com>"
approver_signature: "approved-by: username"
pr: "https://github.com/jwilger/union_square/pull/000"
```

After a schema has an acceptance record, changes MUST follow the post-alignment compatibility rules in `.opencode/rules/incremental-event-fields.md` and `.opencode/rules/event-model-readiness.md`.
