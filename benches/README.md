# Performance Benchmarks

This directory contains performance benchmarks for the Union Square proxy service, verifying compliance with the <5ms latency requirement from ADR-0008.

## Running Benchmarks

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark suite
cargo bench --bench proxy_performance

# Quick mode for development
cargo bench --bench proxy_performance -- --quick
```

## Benchmark Results Summary

Based on current implementation:

### Hot Path Performance
- **Complete hot path flow**: ~1.65µs (0.033% of 5ms budget)
- **Ring buffer write**: ~11-12ns per operation
- **Request ID generation**: ~507ns
- **Audit event serialization**: ~470-480ns

### Key Findings

1. **Sub-microsecond ring buffer**: The ring buffer achieves <1µs handoff as required by ADR-0009
2. **Minimal hot path overhead**: Total proxy overhead is ~1.6µs, leaving 99.97% of the 5ms budget for network I/O
3. **Linear scaling**: Performance scales linearly with payload size up to 64KB
4. **Concurrent performance**: 10 concurrent writes complete in ~137µs total

## Benchmark Categories

### `ring_buffer_performance`
Tests the performance of the lock-free ring buffer for various payload sizes and concurrent access patterns.

### `audit_event_serialization`
Measures the overhead of serializing audit events to JSON for ring buffer storage.

### `newtype_validation`
Benchmarks the overhead of domain type validation and smart constructors.

### `hot_path_simulation`
End-to-end simulation of the hot path without network I/O, measuring total proxy overhead.

### `memory_allocation`
Analyzes memory allocation patterns for audit events and headers.
