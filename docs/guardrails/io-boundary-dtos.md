# Rule: IO Boundary DTOs

Structural IO data MUST stay at system boundaries. Domain-facing APIs MUST use semantic domain facts.

## Boundary DTOs

Boundary DTOs MAY contain:

- Raw strings from HTTP, configuration, or provider payloads.
- Raw bytes from request and response bodies.
- Header maps or header tuples.
- URI strings and query parameters.
- Provider-specific JSON values.
- Database row shapes.

Boundary DTOs MUST be named and located so their IO role is explicit.

## Domain Facts

Domain facts MUST represent parsed business meaning, not transport structure.

Domain-facing APIs MUST NOT expose:

- `serde_json::Value` as provider request facts.
- Raw request or response bytes.
- Raw URI strings.
- Header maps or header tuples.
- Provider DTOs or HTTP framework types.

Conversions from boundary DTOs to domain facts MUST be fallible when parsing or validation can fail.

Parsing failures MUST be represented as explicit semantic facts when they need to be audited. They MUST NOT be recorded as placeholder successful facts.

## Enforcement

- Architecture review against `docs/architecture/ARCHITECTURE.md`.
- `ast-grep` rules for forbidden structural types in `src/domain/**`.
- Code review by `type-theory-reviewer` and `design-reviewer`.
