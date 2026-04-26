# Rule: Consult Architecture Source of Truth

Before making significant architectural changes, consult `docs/architecture/ARCHITECTURE.md`.

`docs/architecture/ARCHITECTURE.md` is the implementation source of truth for Union Square's current target architecture. It must be self-sufficient and must not require readers to inspect ADRs to understand current architectural guidance.

## When to Consult

- Adding a new major component or service.
- Changing data flow.
- Introducing a new technology or dependency.
- Modifying the event model.
- Changing database schema.
- Changing module boundaries or dependency direction.
- Creating or changing performance-island behavior.

## What to Check

1. Does the change align with the current target architecture?
2. Does the change preserve functional-core and imperative-shell boundaries?
3. Does the change keep structural IO data out of domain-facing APIs?
4. Does the change preserve EventCore and CQRS boundaries for durable state changes?
5. Does the change require an ADR because it makes a significant architectural decision?

## ADR Relationship

ADRs record why significant decisions were made. They are historical research artifacts, not active implementation guidance.

If a decision changes current architecture, both documents are required:

- ADR: records the historical context, options, rationale, and consequences.
- `docs/architecture/ARCHITECTURE.md`: records the resulting current target architecture.

Do not leave current guidance only in ADRs. Do not require contributors to reconstruct current practice from ADR supersession chains.

## If ARCHITECTURE.md Is Out of Date

1. Update `docs/architecture/ARCHITECTURE.md` to reflect the intended current architecture.
2. Create an ADR if the correction represents a significant architectural decision.
3. Keep the architecture document understandable without reading ADRs.

## Enforcement

- Code review by `design-reviewer`.
- ADR required for significant architectural decisions.
- Architecture review must verify that active guidance lives in `docs/architecture/ARCHITECTURE.md`.
