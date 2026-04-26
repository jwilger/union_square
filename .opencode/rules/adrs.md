# Rule: ADR Conventions

All significant architectural decisions must be documented in an Architecture Decision Record (ADR). ADRs are immutable historical records of why decisions were made; they are not the source of current architectural guidance.

Current architectural guidance must live in `docs/architecture/ARCHITECTURE.md`.

## Purpose

ADRs must document:

- The context that made a decision necessary.
- The decision drivers that mattered at the time.
- The options considered.
- The decision outcome.
- The rationale and tradeoffs behind the decision.
- The consequences accepted by the team.

ADRs must not serve as living architecture manuals or the only place where current implementation rules are documented.

## Relationship to ARCHITECTURE.md

`docs/architecture/ARCHITECTURE.md` is the implementation source of truth for Union Square's current target architecture.

When an ADR changes the intended architecture:

1. Create a new ADR to explain why the decision was made.
2. Update `docs/architecture/ARCHITECTURE.md` to describe the resulting current architecture.
3. Keep active guidance self-sufficient in `docs/architecture/ARCHITECTURE.md`.

Contributors must not need to reconstruct current guidance from ADR supersession chains.

## When to Create an ADR

Create an ADR for:

- Technology choices such as databases, frameworks, or libraries.
- Architectural patterns such as event sourcing, CQRS, effects, or module boundaries.
- API design decisions with broad architectural impact.
- Security approaches.
- Performance optimization strategies.
- Testing strategies.
- Deployment and infrastructure decisions.

## Format

ADRs follow the template in `docs/adr/template.md`:

- Context and problem statement.
- Decision drivers.
- Considered options with pros and cons.
- Decision outcome.
- Rationale.
- Consequences, both positive and negative.
- Current guidance pointer back to `docs/architecture/ARCHITECTURE.md`.

## Naming Convention

- Filename: `NNNN-descriptive-name.md` where NNNN is zero-padded, for example `0001-overall-architecture-pattern.md`.
- Document title: first line must be `# NNNN. Title`.
- Keep ADR numbers sequential and never reuse numbers.
- The ADR number appears in both filename and title.

## Immutability

Old ADRs are immutable. When a decision changes:

1. Mark the old ADR as superseded by the newer ADR.
2. Create a new ADR explaining why the decision changed.
3. Update `docs/architecture/ARCHITECTURE.md` to reflect the resulting current architecture.
4. Do not modify the content of the old ADR beyond supersession metadata.

## Review Checklist

- The ADR explains why the decision was made.
- The ADR does not act as the only source of current guidance.
- `docs/architecture/ARCHITECTURE.md` reflects the current architecture after the decision.
- `docs/architecture/ARCHITECTURE.md` remains understandable without reading ADRs.

## Enforcement

- Code review by `design-reviewer`.
- Code review must verify accepted ADRs are not modified beyond supersession metadata.
