# Rule: Performance Islands

Performance islands are narrow implementation areas where latency or throughput requirements justify imperative or structural-data-heavy implementation techniques.

## Allowed Candidates

The following areas MAY become performance islands when benchmark or deterministic validation evidence justifies the exception:

- Ring-buffer internals.
- Hot-path request forwarding.
- Streaming body handling.
- Audit handoff between proxy forwarding and asynchronous persistence.

## Requirements

Performance islands MUST:

- Be isolated behind a small API.
- Preserve observable domain semantics.
- Document the measured behavior that justifies the exception.
- Include benchmark or deterministic validation coverage.
- Keep structural data from leaking into domain-facing APIs.
- Avoid hidden database writes, clocks, logging, metrics, or provider IO inside domain code.

## Forbidden Uses

Performance concerns MUST NOT justify:

- Accepting `serde_json::Value`, raw bytes, raw URI strings, or header tuples in domain commands.
- Bypassing EventCore or the selected EventCore-backed adapter for durable state changes.
- Moving provider DTOs into the domain core.
- Treating all proxy code as exempt from architectural boundaries.

## Enforcement

- Architecture review against `docs/architecture/ARCHITECTURE.md`.
- Benchmarks or deterministic performance validation for each exception.
- Code review by `async-rust-expert`, `functional-architecture-expert`, and `design-reviewer`.
