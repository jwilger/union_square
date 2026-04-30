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

# CI-appropriate validation and quick benchmark run
just bench-quick

# Heavier local benchmark suite
just bench-local

# Run memory profiling
cargo bench --bench memory_profiling

# Run ignored load tests without the database-pool test
cargo test --test load_testing --release test_500_rps_sustained_load -- --ignored --nocapture --test-threads=1
cargo test --test load_testing --release test_2000_rps_burst_load -- --ignored --nocapture --test-threads=1
cargo test --test load_testing --release test_1000_concurrent_users -- --ignored --nocapture --test-threads=1
```

## Benchmark Results Summary

The version-controlled baseline contract is maintained in
`docs/performance/baselines.md`. Update that document when changing hot-path,
ring-buffer, streaming, or audit handoff performance behavior.

**✅ ALL LATENCY REQUIREMENTS MET** - Based on comprehensive benchmarks:

### 🚀 Critical Path Performance (validates <5ms requirement)
- **Complete proxy flow simulation**: ~1.48µs (0.03% of 5ms budget)
- **Hot path latency distribution**: ~1.53µs
- **Complete hot path flow**: ~1.54µs
- **Latency requirement validation**: ~1.48µs

### 🔧 Component Performance Breakdown
- **Ring buffer write**: ~11.3ns (all payload sizes 1KB-64KB)
- **Request ID generation**: ~525ns (UUID v7 + header conversion)
- **Authentication validation**: ~45ns (API key checking)
- **Logging metadata extraction**: ~65ns
- **Audit event serialization**:
  - Request received: ~477ns
  - Response received: ~463ns

### 🎯 Key Performance Achievements

1. **✅ Sub-5ms requirement exceeded**: Total proxy overhead is **1.5µs**, leaving **99.97%** of budget for network I/O
2. **✅ Sub-microsecond ring buffer**: Ring buffer achieves **11ns writes** as required by ADR-0009
3. **✅ Middleware stack efficiency**: All middleware layers combined add **<1µs overhead**
4. **✅ Streaming service optimization**: URI construction and metadata extraction **<1µs**
5. **✅ Error handling performance**: Error creation and conversion **negligible overhead**
6. **✅ Concurrent performance**: Lock-free operations scale linearly

### 📊 Performance Budget Analysis
- **Ring buffer writes**: 11ns (0.0002% of budget)
- **Request ID middleware**: 525ns (0.01% of budget)
- **Authentication middleware**: 45ns (0.001% of budget)
- **Audit event creation**: 477ns (0.01% of budget)
- **Total proxy overhead**: ~1.5µs (0.03% of budget)
- **Available for network I/O**: ~4.998ms (99.97% of budget)

### 🎯 MVP Load Testing Targets

Per architect guidance, Union Square must handle:

1. **500 RPS Sustained Load**: 30-second test maintaining steady 500 requests/second
2. **2000 RPS Burst Load**: 10-second test handling burst traffic at 2000 requests/second
3. **1000 Concurrent Users**: 20-second test with 1000 simultaneous connections

#### ⚠️ Important: Load Testing Infrastructure

**Load tests should NOT be run on GitHub Actions** due to resource constraints:
- GitHub Actions runners have only 2 CPU cores and 7GB RAM
- This is insufficient for realistic load testing scenarios
- Results from resource-constrained environments can be misleading

**Recommended approach for load testing:**
1. Run load tests locally on dedicated hardware or cloud instances
2. Use production-like infrastructure with adequate resources
3. Consider using dedicated load testing services (e.g., k6 Cloud, BlazeMeter)
4. Document baseline performance metrics from representative hardware

Run the resource-heavy load tests locally with:
```bash
just bench-local
```

### 🔍 Memory Profiling

Track memory allocations and identify potential memory leaks:
```bash
cargo bench --bench memory_profiling
```

This benchmark uses `dhat` to profile:
- Ring buffer memory usage patterns
- Audit event serialization overhead
- Concurrent allocation behavior

### 📈 Performance Benchmarks on GitHub Actions

The CPU-bound performance benchmarks (Criterion benchmarks) **ARE suitable for GitHub Actions** because:

1. **Relative measurements**: We're tracking regressions, not absolute performance
2. **Consistent environment**: GitHub Actions provides consistent (if limited) hardware
3. **Statistical validity**: Multiple samples over 10-15 seconds provide reliable comparisons
4. **No I/O dependency**: These benchmarks test algorithmic performance, not system capacity

**Note**: Absolute timings may be slower on GitHub Actions than production hardware, but relative changes between commits remain meaningful for regression detection.

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
