# Rule: Functional Core Boundaries

Union Square uses a strict functional-core and imperative-shell architecture.

## Functional Core

Domain code MUST be deterministic for the same inputs.

Domain code MUST NOT directly perform or depend on:

- HTTP framework operations.
- Provider client operations.
- Database access.
- Filesystem access.
- Environment variables or process-global configuration.
- Clocks or randomness.
- Tokio runtime APIs or task spawning.
- Tracing, logging, or metrics backends.

Domain functions MUST receive observed facts as values from the imperative shell.

## Imperative Shell

The imperative shell owns IO and external observations. It MUST:

- Read HTTP requests and provider responses.
- Read configuration and environment data.
- Obtain timestamps and generated identifiers.
- Parse structural data into semantic domain facts.
- Interpret declarative effects.
- Persist events and emit telemetry.

## Enforcement

- Architecture review against `docs/architecture/ARCHITECTURE.md`.
- `ast-grep` rules for forbidden domain imports.
- Code review by `design-reviewer` and `functional-architecture-expert`.
