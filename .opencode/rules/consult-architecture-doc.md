# Rule: Consult Architecture Doc

Before making significant architectural changes, consult `docs/architecture/ARCHITECTURE.md`.

## When to Consult

- Adding a new major component or service
- Changing the data flow
- Introducing a new technology or dependency
- Modifying the event model
- Changing database schema

## What to Check

1. Does the change align with the documented architecture?
2. Does it follow established patterns (EventCore, type-driven, functional core)?
3. Are there existing ADRs that cover this area?
4. Does the change require a new ADR?

## If the Architecture Doc is Out of Date

1. Update `docs/architecture/ARCHITECTURE.md` to reflect reality
2. Create an ADR explaining why the architecture changed
3. Proceed with the change

## Enforcement

- Code review by `design-reviewer`
- ADR required for architectural changes
