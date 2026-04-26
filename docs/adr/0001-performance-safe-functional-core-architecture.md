# 0001. Performance-Safe Functional Core Architecture

## Status

Accepted

## Context

Union Square records LLM request and response lifecycles while proxying traffic to external providers. Correctness and auditability require a domain model that is explicit, deterministic, and easy to replay or test. Proxy forwarding, streaming bodies, audit handoff, and ring-buffer infrastructure also have latency and throughput requirements that cannot be treated as secondary concerns.

The codebase previously mixed active guidance across architecture documentation, rule files, and planned decision records. That made it too easy for contributors to treat historical rationale as current implementation guidance or to miss the final architecture after decisions were superseded.

## Decision Drivers

- Domain behavior must remain deterministic and testable.
- IO, clocks, runtime concerns, database access, HTTP types, and provider DTOs must stay out of the domain core.
- Domain APIs must use semantic types rather than raw strings, bytes, headers, URIs, or JSON values.
- Event-sourced state changes must remain explicit and auditable.
- Performance-sensitive paths must preserve measured latency and throughput.
- Contributors need one self-sufficient current architecture document for day-to-day work.
- ADRs need to remain useful for historical research without becoming living architecture manuals.

## Considered Options

- Keep architecture guidance spread across ADRs, rules, and architecture documents.
- Make ADRs the primary architecture guidance and update them as decisions change.
- Make `docs/architecture/ARCHITECTURE.md` the current architecture source of truth and use ADRs only to record historical rationale.

## Decision Outcome

Union Square will use a strict functional-core and imperative-shell architecture, with semantic domain types inside the core and explicit IO boundary DTOs in adapters. Non-hot-path orchestration may use effect, step, and trampoline patterns to keep side-effect sequencing testable. Ring-buffer internals, hot-path forwarding, streaming body handling, and audit handoff may use narrow performance islands when measurement justifies the exception.

`docs/architecture/ARCHITECTURE.md` is the self-sufficient implementation source of truth for the current target architecture. ADRs record why significant architectural decisions were made, including context, alternatives, and consequences. ADRs are not the source of ongoing architectural guidance.

## Rationale

A strict functional core makes audit behavior easier to test, replay, and reason about. Keeping structural IO data in adapters prevents external provider and transport concerns from shaping the domain model. Semantic domain types make invalid states harder to construct and reduce ambiguity in event schemas and command logic.

Effect and trampoline orchestration keeps non-hot-path workflows explicit without hiding IO inside domain functions. It supports deterministic tests and clear retry or observability seams.

Performance islands acknowledge that parts of a proxy service must be optimized for latency and throughput. Requiring isolation, documentation, and benchmark evidence keeps those exceptions from weakening the broader architecture.

Separating ADRs from current guidance preserves both needs: historical research can answer why a rule exists, while implementation work uses a single current-state architecture document.

## Consequences

### Positive

- Contributors can consult one architecture document for current guidance.
- ADRs remain immutable historical records instead of living manuals.
- Domain code has clearer import and data-shape boundaries.
- Performance exceptions are allowed without normalizing structural data in the domain.
- Later refactoring issues can enforce architecture rules mechanically.

### Negative

- Significant architecture-changing work must create a new historical decision record and update the current architecture projection when applicable.
- Some existing code will violate the target boundaries until follow-up refactors complete.
- Performance islands require benchmark or deterministic validation evidence before they are accepted.

## Current Guidance

This ADR is a historical record. Current architectural guidance belongs in `docs/architecture/ARCHITECTURE.md`.
