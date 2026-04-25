---
name: event-model-conformance
description: Validate event sourcing implementation against EventCore patterns and event modeling best practices
license: MIT
compatibility: opencode
---

## What I do
- Verify events are named in past tense
- Check events carry all necessary data for projections
- Validate stream boundaries are correct
- Ensure event schema evolution follows incremental rules
- Review command logic for business rule enforcement

## When to use me
Use this skill when implementing or reviewing event-sourced features, designing new events, or modifying command handlers.

## Review Checklist
- [ ] Events are past tense (`SessionRecorded`, not `RecordSession`)
- [ ] Events are immutable and self-contained
- [ ] Stream boundaries match consistency requirements
- [ ] Commands use `require!` and `emit!` macros
- [ ] Event schema changes follow incremental field rules
