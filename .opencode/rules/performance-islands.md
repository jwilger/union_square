# Rule: Performance Islands

Performance islands are narrow implementation areas where latency or throughput requirements justify imperative or structural-data-heavy implementation techniques.

## Allowed Candidates

The following areas MAY become performance islands when benchmark or deterministic validation evidence justifies the exception:

- Ring-buffer internals.
- Hot-path request forwarding.
- Streaming body handling.
- Audit handoff between proxy forwarding and asynchronous persistence.

## Documented Exceptions

### Ring Buffer (`src/proxy/ring_buffer.rs`)

The ring buffer is an explicit performance island with the following measured characteristics:

- **Write latency**: <1μs per write under single-threaded load (validated by `ring_buffer_performance_test.rs`)
- **Concurrent throughput**: >1M ops/sec under multi-threaded contention (validated by stress tests)
- **Zero heap allocations**: After initialization, all slot data is pre-allocated
- **Lock-free**: Uses atomic CAS operations for coordination

Justification: The ring buffer sits on the critical path between proxy forwarding and async audit persistence. Any allocation, lock, or channel operation in this path would add unpredictable latency to request processing.

Constraints:
- Unsafe code is restricted to this single module (`#![allow(unsafe_code)]` at module level only).
- The public API (`write`, `read`, `stats`, `overflow_count`) exposes only semantic types (`RequestId`, `Vec<u8>`) and primitive counters.
- Clock calls (e.g., `chrono::Utc::now()`) MUST be captured outside `unsafe` blocks to avoid hidden side effects inside the performance-critical path.

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
