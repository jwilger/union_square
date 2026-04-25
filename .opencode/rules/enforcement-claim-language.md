# Rule: Enforcement Claim Language

When documenting rules or requirements, use precise, enforceable language.

## Language Levels

- **MUST / MUST NOT** — Absolute requirements. Violations are bugs.
- **SHOULD / SHOULD NOT** — Strong recommendations. Violations need justification.
- **MAY** — Optional. Truly optional features or approaches.

## Forbidden Language

- "Consider..." — Too vague. Either require it or don't mention it.
- "Try to..." — Not enforceable. Use MUST or SHOULD.
- "Ideally..." — Not enforceable. Use SHOULD or remove.

## Examples

### Bad
```markdown
Developers should consider using newtypes for domain concepts.
Try to avoid unwrap in production code.
```

### Good
```markdown
Domain concepts MUST use newtypes with validation.
Production code MUST NOT use unwrap, expect, or panic!.
```

## Why

Precise language makes rules actionable and reviewable. When a rule says "MUST NOT use unwrap", code review is straightforward. When a rule says "try to avoid unwrap", every instance becomes a negotiation.

## Enforcement

- Self-enforcing through code review
- Rules in `.opencode/rules/` must follow this convention
