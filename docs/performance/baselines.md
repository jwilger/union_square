# Performance Baselines

Issue: #182

This document records the baseline gates that protect Union Square's documented
performance islands before architecture refactoring touches ring-buffer, proxy
hot-path, streaming, or audit handoff code.

## Commands

### CI validation

Run:

```bash
just bench-quick
```

`just bench-quick` is the CI-appropriate performance command. It runs
`cargo test --test benchmark_validation` for deterministic validation and
`cargo bench --bench proxy_performance -- --quick --noplot` for fast Criterion
coverage of CPU-bound benchmark scenarios.

This command is allowed in CI because it avoids network IO, database IO, and
external providers. The deterministic test portion is the regression gate; the
quick Criterion run adds trend evidence without being the only correctness
signal.

### Local benchmark

Run:

```bash
just bench-local
```

`just bench-local` is the heavier local benchmark suite. It runs the full
`proxy_performance` Criterion benchmark, the `memory_profiling` benchmark, and
release-mode load tests. Use it on a quiet workstation or representative
performance host when validating changes to hot-path, ring-buffer, streaming, or
audit handoff behavior.

## Coverage Inventory

| Scenario | CI validation | Local benchmark | Baseline | Regression threshold |
| --- | --- | --- | --- | --- |
| Ring-buffer write latency | `tests/benchmark_validation.rs::test_critical_path_performance` and `src/proxy/ring_buffer_performance_test.rs::test_single_threaded_performance` | `benches/proxy_performance.rs::ring_buffer_performance/write_*kb` | 11.888ns median for 1KB writes, 11.975ns for 10KB writes, and 11.590ns for 64KB writes in the `just bench-quick` run for issue #182; deterministic test requires average below 1ms and max below 5ms for 1KB writes | CI must remain below the deterministic 1ms average and 5ms max budgets; local Criterion median should not regress by more than 2x without documented hardware or workload justification |
| Ring-buffer read throughput | ring-buffer correctness and concurrent tests exercise readable handoff without loss under capacity | `benches/memory_profiling.rs::profile_ring_buffer` plus full proxy benchmark readback-adjacent scenarios | read path remains single-consumer and bounded by preallocated slot size; no existing standalone read-throughput Criterion case | Add a dedicated read-throughput benchmark before changing read internals; until then, no read-path change may ship without `just bench-local` evidence and updated baseline numbers |
| Ring-buffer overflow behavior | `src/proxy/ring_buffer_tests.rs` property and stress coverage validates under-capacity, oversized payload, wraparound, and stats behavior | release-mode load tests exercise sustained pressure | overflow is explicit via `DroppedEventCount` and fail-when-busy semantics | Overflow counters must remain monotonic and no successful write may corrupt readable payloads; any changed overflow policy requires a new deterministic validation test |
| Hot-path proxy overhead | `tests/benchmark_validation.rs::test_critical_path_performance` | `benches/proxy_performance.rs::hot_path_simulation` and `complete_proxy_flow_simulation` | 717.68ns median for complete hot-path simulation, 740.03ns for hot-path latency distribution, and 1.7269us for simulated complete proxy request in the `just bench-quick` run for issue #182 | CI must stay below the 5ms hot-path budget; local Criterion median should not regress by more than 2x without documented architecture approval |
| Representative audit handoff cost | `tests/benchmark_validation.rs::test_allocation_performance` | `benches/proxy_performance.rs::audit_event_serialization` and `memory_allocation` | request serialization 541.90ns median, response serialization 501.95ns median, and audit event allocation 1.0235us median in the `just bench-quick` run for issue #182 | Per-event deterministic validation must stay below 1ms; local Criterion median should not regress by more than 2x without documenting the added semantic work |

## Threshold Rationale

The CI validation thresholds are intentionally broad. They catch severe
regressions and accidental blocking work while avoiding flakes from shared CI
hardware. The local benchmark thresholds are tighter relative checks because
Criterion results are more useful when compared on the same hardware and under
similar load.

The 5ms proxy budget is the user-visible latency guard. The ring-buffer write
path remains a documented performance island and should stay comfortably
sub-microsecond in local benchmarks. Any future refactor that routes hot-path or
ring-buffer behavior through new abstractions must update this document with
fresh baseline evidence before it is accepted.
