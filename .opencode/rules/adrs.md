# Rule: ADR Conventions

All significant architectural decisions must be documented in an Architecture Decision Record (ADR).

## When to Create an ADR

Create an ADR for:
- Technology choices (databases, frameworks, libraries)
- Architectural patterns (event sourcing, CQRS, etc.)
- API design decisions
- Security approaches
- Performance optimization strategies
- Testing strategies
- Deployment and infrastructure decisions

## Format

ADRs follow the template in `docs/adr/template.md`:
- Context and problem statement
- Decision drivers
- Considered options with pros/cons
- Decision outcome
- Consequences (positive and negative)

## Naming Convention

- **Filename**: `NNNN-descriptive-name.md` where NNNN is zero-padded (e.g., `0001-overall-architecture-pattern.md`)
- **Document Title**: First line must be `# NNNN. Title`
- Keep ADR numbers sequential and never reuse numbers
- The ADR number appears in both filename and title

## Immutability

Old ADRs are immutable. When a decision changes:
1. Mark old ADR as "superseded by [new ADR]"
2. Create new ADR explaining the change
3. Do not modify the content of the old ADR

## Enforcement

- Code review by `design-reviewer`
- CI check ensuring ADRs are not modified after merge
