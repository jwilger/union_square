# Union Square Architecture

This document is the implementation source of truth for Union Square's current target architecture. It is self-sufficient: contributors must be able to use this document to understand current architectural rules without reconstructing history from decision records.

## Purpose

Union Square is a proxy and wire-tap service for LLM calls. It forwards client requests to LLM providers while recording the complete request and response lifecycle for later analysis, audit, and test-case extraction.

## Architectural Drivers

- Correctness and auditability of recorded LLM interactions are primary product requirements.
- The domain model must use semantic types and explicit state transitions so illegal states are unrepresentable where practical.
- Side effects must stay at IO boundaries unless a documented performance island allows otherwise.
- Ultra-low latency and high throughput are first-class requirements for proxy forwarding, streaming bodies, audit handoff, and ring-buffer infrastructure.
- Performance exceptions must be narrow, measured, isolated, and guarded.
- During the architecture alignment initiative, no backward compatibility is required for current persisted events, event schemas, or deployed behavior.

## System Responsibilities

- Proxy client requests to configured LLM providers.
- Capture request, response, timing, provider, and session metadata.
- Persist audit facts through EventCore-backed event storage.
- Build read models for session analysis and test extraction.
- Preserve hot-path latency and throughput where the proxy is on the critical path.

## Technology Stack

- Language: Rust, edition 2021.
- Runtime: Tokio in the imperative shell only.
- Web framework: Axum and Tower at HTTP boundaries only.
- Event sourcing: EventCore with PostgreSQL persistence.
- Database access: sqlx in infrastructure adapters only.
- Type safety: `nutype`, enums, and smart constructors for semantic domain concepts.
- Error handling: `thiserror` for typed domain and adapter errors; `anyhow` only at application boundaries.

## Module Boundaries

Union Square is currently a single crate. The crate must still maintain explicit architectural layers.

### Domain Core

Domain modules contain semantic types, pure state transitions, domain events, command state, and business rules.

Domain code must not import or depend on:

- HTTP framework types.
- Proxy transport types.
- Provider client DTOs.
- Database clients or SQL types.
- Async runtime APIs.
- Environment variables or process-global configuration.
- Clocks, randomness, tracing, metrics, or logging backends.
- Raw JSON values, raw byte buffers, URI strings, or header tuples as domain facts.

### Application Layer

The application layer coordinates domain decisions and effect interpretation. Non-hot-path orchestration should use explicit step or trampoline execution when it improves clarity, testability, retries, or observability.

Application orchestration may depend on domain types and boundary traits. It must not hide IO inside domain functions.

### Proxy And Provider Adapters

Proxy and provider adapters own structural IO data:

- HTTP requests and responses.
- Header maps and raw header tuples.
- URI strings and query parameters.
- Raw request and response bytes.
- Provider-specific JSON shapes.
- Streaming body primitives.

Adapters must parse external data into semantic domain facts before invoking domain behavior. If parsing fails, adapters must produce explicit parse-failure facts rather than placeholder successful facts.

### Infrastructure

Infrastructure modules own database connections, EventCore persistence adapters, filesystem access, environment loading, telemetry backends, and runtime integration.

Infrastructure code may use primitives and structural types internally. Those types must not cross into domain APIs except through explicit semantic conversions.

### Tests

Tests may use structural test data at setup boundaries. Domain tests should assert semantic behavior through domain types. Acceptance tests should exercise external boundaries and verify user-visible outcomes.

## Semantic Domain Types

Domain-facing APIs must use semantic types, not raw primitives, for domain concepts.

Allowed domain concepts include:

- Session identifiers.
- Request identifiers.
- Provider identifiers.
- Model identifiers.
- Prompt and message content facts.
- Token counts.
- Timestamps supplied by the shell.
- Recorded request and response facts.
- Parse-success and parse-failure facts.

Boundary DTOs may use strings, bytes, JSON values, header tuples, and HTTP types. Domain types must not expose those structural forms directly.

Conversions from DTOs to domain facts must be fallible when parsing or validation can fail. Validation belongs at system boundaries; once a value enters the domain core, its type should guarantee the relevant invariant.

## Functional Core And Imperative Shell

Union Square uses a strict functional-core and imperative-shell architecture.

The functional core must:

- Be deterministic for the same inputs.
- Receive time, IDs, configuration, and external observations as input values.
- Return domain decisions, state transitions, events, or declarative effects.
- Avoid IO, clocks, randomness, runtime spawning, logging, metrics emission, and database access.

The imperative shell must:

- Read HTTP requests, provider responses, configuration, clocks, and environment data.
- Parse structural data into semantic DTOs and domain facts.
- Interpret effects and execute IO.
- Persist events through EventCore or the selected event-store adapter.
- Emit telemetry and operational logs.

## Effect, Step, And Trampoline Orchestration

Non-hot-path orchestration should use an explicit effect, step, or trampoline pattern when the workflow needs testable sequencing of side effects.

Effects describe intended work. They must not perform the work themselves.

Steps represent the next pure decision in a workflow. A step may complete with a domain result or request an effect for the shell to interpret.

The trampoline executes steps by interpreting effects in the imperative shell, feeding observations back into the pure workflow, and stopping when the workflow completes or fails.

Use this pattern for audit persistence coordination, session analysis workflows, test extraction, retryable provider-independent workflows, and other non-hot-path orchestration where explicit sequencing improves clarity.

Do not use this pattern inside measured hot-path forwarding or streaming loops unless benchmarks show the overhead is acceptable.

## Event Sourcing And CQRS

All durable state changes must use EventCore command patterns or the selected EventCore-backed adapter.

Commands define consistency boundaries with explicit streams. Business rules are enforced before events are emitted. Events record facts that already happened and must be named in past tense.

Read models and projections are separate from command logic. Queries read projections or read models; they must not mutate state. Command logic must not query read models to enforce invariants.

During the architecture alignment initiative, existing event schemas may be replaced when that is the cleanest path. A schema is accepted as part of the aligned architecture only when it has a canonical acceptance record in `.opencode/accepted-replay/<schema-id>.yaml`. After acceptance, event evolution must be additive or use a new event variant so historical replay remains safe.

## Performance Islands

Performance islands are narrow parts of the codebase that may use imperative, allocation-conscious, or structural-data-heavy implementation techniques because they protect latency or throughput.

Allowed performance-island candidates include:

- Ring-buffer internals.
- Hot-path request forwarding.
- Streaming body handling.
- Audit handoff between the proxy path and asynchronous persistence.

Performance islands must:

- Be isolated behind a small API.
- Preserve observable domain semantics.
- Avoid becoming a general-purpose domain modeling style.
- Include benchmark or validation evidence for the exception.
- Document the measured behavior that justifies the exception.
- Keep raw primitives and structural data from leaking into domain APIs.

Allowed example: a ring-buffer module may use atomics, preallocated storage, and raw byte-oriented slots internally when benchmarks show this is required for audit handoff throughput. Its public API must still expose semantic audit handoff concepts or boundary DTOs, not arbitrary domain primitives.

Forbidden example: a domain command must not accept `serde_json::Value`, raw request bytes, URI strings, or header tuples because parsing those values is adapter work, not domain work.

## Data Flow

```text
Client HTTP request
  -> Proxy adapter captures structural HTTP data
  -> Boundary parser converts structural data into semantic facts or parse-failure facts
  -> Application workflow invokes domain logic
  -> Domain logic emits events or declarative effects
  -> Imperative shell interprets effects and persists through EventCore
  -> Projections build read models for queries and test extraction
  -> Proxy adapter returns provider response to the client
```

Hot-path forwarding may take a shorter execution route when benchmark evidence requires it, but audit facts must still cross into persistence through documented boundary seams.

## Development Conventions

- Production code must not use `unwrap`, `expect`, `panic!`, `todo!`, `unimplemented!`, or `unreachable!` for recoverable cases.
- Domain functions should be total and handle all modeled cases explicitly.
- Domain concepts must use semantic types instead of primitive obsession.
- IO DTOs must be named and located so their boundary role is obvious.
- Architecture rules must use precise `MUST`, `MUST NOT`, `SHOULD`, and `MAY` language when written as guardrails.
- New features must have acceptance coverage at the relevant external boundary unless the change is documentation-only.
- Performance-sensitive changes must include benchmarks or deterministic validation appropriate to the risk.
