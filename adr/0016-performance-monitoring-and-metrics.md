# 0016. Performance Monitoring and Metrics Collection

- Status: accepted
- Deciders: John Wilger, Technical Architecture Team
- Date: 2025-07-15

## Context

Union Square must collect comprehensive metrics for:
1. Performance monitoring (latency, throughput)
2. Cost tracking (tokens, API calls)
3. Error analysis (provider errors, rate limits)
4. Business metrics (usage patterns, F-scores)
5. Operational health (resource usage, queue depths)

Critical requirement: Metrics collection cannot impact the <5ms latency budget in the hot path.

## Decision

We will implement a multi-layered metrics architecture optimized for minimal overhead:

### Metrics Collection Strategy

1. **Hot Path Metrics** - Collected with zero allocation
2. **Audit Path Metrics** - Rich metrics processed asynchronously
3. **System Metrics** - Infrastructure and resource metrics

### Hot Path Metrics Design

```rust
/// Zero-allocation metrics using atomic counters
struct HotPathMetrics {
    requests_total: AtomicU64,
    requests_failed: AtomicU64,
    bytes_sent: AtomicU64,
    bytes_received: AtomicU64,

    // Per-provider counters (fixed size array)
    provider_requests: [AtomicU64; MAX_PROVIDERS],
    provider_latency_sum: [AtomicU64; MAX_PROVIDERS],

    // Histogram buckets for latency (powers of 2: 1ms, 2ms, 4ms, etc.)
    latency_buckets: [AtomicU64; 16],
}

impl HotPathMetrics {
    /// Record metrics with ~10ns overhead
    #[inline(always)]
    fn record(&self, provider: usize, latency_us: u64, success: bool) {
        self.requests_total.fetch_add(1, Ordering::Relaxed);

        if !success {
            self.requests_failed.fetch_add(1, Ordering::Relaxed);
        }

        self.provider_requests[provider].fetch_add(1, Ordering::Relaxed);
        self.provider_latency_sum[provider].fetch_add(latency_us, Ordering::Relaxed);

        // Find histogram bucket (fast bit operation)
        let bucket = (64 - latency_us.leading_zeros()).min(15) as usize;
        self.latency_buckets[bucket].fetch_add(1, Ordering::Relaxed);
    }
}
```

### Audit Path Metrics Processing

```rust
struct MetricsAggregator {
    // Add fields as needed for metrics aggregation
}

impl MetricsAggregator {
    /// Processes rich metrics from ring buffer data
    async fn process_metrics(&self, event: &AuditEvent) {
        // Extract detailed metrics
        let metrics = DetailedMetrics {
            timestamp: event.timestamp,
            provider: event.provider,
            model: event.model,

            // Latency breakdown
            latency_total_ms: event.latency_total,
            latency_ttfb_ms: event.latency_ttfb,  // Time to first byte
            latency_proxy_ms: event.latency_proxy,

            // Token usage
            tokens_input: event.tokens_input,
            tokens_output: event.tokens_output,
            tokens_total: event.tokens_total(),

            // Cost calculation
            cost_usd: calculate_cost(event),

            // Request details
            cache_hit: event.cache_hit,
            streaming: event.streaming,
            error_code: event.error_code,

            // Session info
            session_id: event.session_id,
            user_id: event.user_id,
            application_id: event.application_id,
        };

        // Store in time-series storage
        self.store_metrics(metrics).await;
    }
}
```

### Metrics Storage Schema

```sql
-- Optimized for time-series queries
CREATE TABLE metrics_1min (
    timestamp TIMESTAMPTZ NOT NULL,
    application_id UUID NOT NULL,
    provider TEXT NOT NULL,
    model TEXT NOT NULL,

    -- Aggregated metrics
    request_count INT,
    error_count INT,
    cache_hits INT,

    -- Latency percentiles (microseconds)
    latency_p50 INT,
    latency_p90 INT,
    latency_p99 INT,
    latency_max INT,

    -- Token usage
    tokens_input_sum BIGINT,
    tokens_output_sum BIGINT,

    -- Cost
    cost_usd_sum DECIMAL(10, 6),

    PRIMARY KEY (application_id, timestamp)
) PARTITION BY RANGE (timestamp);

-- Create partitions for efficient retention
CREATE TABLE metrics_1min_2024_01 PARTITION OF metrics_1min
    FOR VALUES FROM ('2024-01-01') TO ('2024-02-01');
```

### Metrics Export

Support multiple metrics backends:

```rust
trait MetricsExporter: Send + Sync {
    async fn export(&self, metrics: &[Metric]) -> Result<(), ExportError>;
}

// Built-in exporters
struct PrometheusExporter { /* ... */ }
struct DatadogExporter { /* ... */ }
struct OpenTelemetryExporter { /* ... */ }

// Metrics routing
struct MetricsRouter {
    exporters: Vec<Box<dyn MetricsExporter>>,

    async fn export(&self, metrics: &[Metric]) {
        // Fan out to all configured exporters
        for exporter in &self.exporters {
            tokio::spawn(exporter.export(metrics));
        }
    }
}
```

### Real-time Metrics API

```rust
// WebSocket endpoint for live metrics
ws://api/v1/metrics/stream

// Real-time updates
{
    "type": "metrics.update",
    "timestamp": "2024-01-15T10:30:00Z",
    "application_id": "...",
    "metrics": {
        "requests_per_second": 125.3,
        "error_rate": 0.002,
        "p99_latency_ms": 45.2,
        "active_sessions": 342
    }
}
```

### F-Score Tracking

Special handling for precision/recall metrics:

```rust
struct FScoreCalculator {
    /// Calculate F-scores from test execution results
    fn calculate_fscore(&self, results: &[TestResult]) -> FScore {
        let true_positives = results.iter().filter(|r| r.expected && r.actual).count();
        let false_positives = results.iter().filter(|r| !r.expected && r.actual).count();
        let false_negatives = results.iter().filter(|r| r.expected && !r.actual).count();

        let precision = if true_positives + false_positives == 0 {
            0.0
        } else {
            true_positives as f64 / (true_positives + false_positives) as f64
        };
        let recall = if true_positives + false_negatives == 0 {
            0.0
        } else {
            true_positives as f64 / (true_positives + false_negatives) as f64
        };

        FScore {
            precision,
            recall,
            f1: if (precision + recall) == 0.0 {
                0.0
            } else {
                2.0 * (precision * recall) / (precision + recall)
            },
            timestamp: Utc::now(),
        }
    }
}
```

### System Metrics Collection

```toml
[metrics.system]
# Resource metrics
collect_cpu = true
collect_memory = true
collect_disk_io = true
collect_network_io = true

# Application metrics
collect_ring_buffer = true
collect_connection_pool = true
collect_cache_stats = true

# Collection interval
interval_seconds = 10
```

## Consequences

### Positive

- Near-zero overhead in hot path
- Rich metrics for analysis
- Flexible export options
- Real-time monitoring capability
- Cost tracking built-in
- F-score tracking for ML metrics

### Negative

- Memory overhead for atomic counters
- Complex aggregation logic
- Multiple storage tiers
- Export bandwidth costs
- Time-series data growth

### Mitigation Strategies

1. **Sampling**: Sample detailed metrics for high-volume traffic
2. **Compression**: Compress metrics before storage/export
3. **Retention**: Automatic rollup and expiration
4. **Batching**: Batch metrics exports
5. **Circuit Breaking**: Disable metrics under extreme load

## Alternatives Considered

1. **External APM Only**
   - Use Datadog/New Relic exclusively
   - Rejected: Vendor lock-in, cost, latency

2. **Log-based Metrics**
   - Derive metrics from logs
   - Rejected: Too slow, expensive

3. **Synchronous Metrics**
   - Collect all metrics in hot path
   - Rejected: Violates latency requirements

4. **No Custom Metrics**
   - Basic metrics only
   - Rejected: Insufficient for requirements

5. **Pull-based Metrics**
   - Prometheus-style scraping only
   - Rejected: Need push for real-time

## Implementation Notes

- Use lock-free data structures
- Align atomic counters to cache lines
- Pre-allocate all metrics memory
- Test metrics overhead carefully
- Document metric definitions

## Related Decisions

- ADR-0008: Dual-path Architecture (metrics collection split)
- ADR-0009: Ring Buffer Pattern (metrics from ring buffer)
- ADR-0010: Tiered Projection Strategy (metrics storage)
