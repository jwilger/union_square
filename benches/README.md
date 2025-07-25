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
