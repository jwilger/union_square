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
- **Zero heap allocations (write/internal-slot path)**: After initialization, slot storage is pre-allocated; the read path materializes payload via `Vec<u8>` which may allocate heap
- **Lock-free**: Uses atomic CAS operations for coordination

Justification: The ring buffer sits on the critical path between proxy forwarding and async audit persistence. Any allocation, lock, or channel operation in the write path would add unpredictable latency to request processing.

Constraints:
- Unsafe code is restricted to this single module (`#![allow(unsafe_code)]` at module level only).
- The public API (`write`, `read`, `stats`, `overflow_count`) is narrow: `write` accepts a semantic `RequestId` and borrows payload bytes (`&[u8]`) which it copies into pre-allocated slot storage; `read` returns `Option<(RequestId, Vec<u8>)>`; counters are primitive types.
- Clock calls (e.g., `chrono::Utc::now()`) MUST be captured outside `unsafe` blocks to avoid hidden side effects inside the performance-critical path.

## Regression Threshold Rationale

CI validation thresholds are intentionally broad. They catch severe regressions
and accidental blocking work while avoiding flakes from shared CI hardware. Local
benchmark thresholds are tighter relative checks because Criterion results are
more useful when compared on the same hardware and under similar load.

The 5ms proxy budget refers to the maximum critical-path latency enforced by CI
deterministic validation (`benchmark_validation`: average <1ms, max <5ms) and
serves as the user-visible latency guard. The ring-buffer write path remains a
documented performance island and should stay comfortably sub-microsecond in
local benchmarks. Any future refactor that routes hot-path or ring-buffer
behavior through new abstractions must update the relevant baseline evidence
before it is accepted.

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
