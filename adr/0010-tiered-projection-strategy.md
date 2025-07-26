# 0010. Tiered Projection Strategy

- Status: accepted
- Deciders: John Wilger, Technical Architecture Team
- Date: 2025-07-15

## Context

Union Square needs to serve different query patterns with varying performance requirements:

1. **Real-time queries** (<10ms): Active session monitoring, current metrics
2. **Interactive queries** (<100ms): Recent session browsing, test execution
3. **Analytical queries** (seconds): Historical analysis, cost reports, ML training data

Using a single storage solution cannot efficiently serve all these patterns. EventCore events are the source of truth, but querying raw events for every request would be too slow.

## Decision

We will implement a three-tier projection strategy, where each tier is optimized for specific query patterns:

### Tier 1: In-Memory Projections (Real-time)

**Purpose**: Serve hot data with microsecond latency

**Technology**: Custom in-memory data structures

**Data Included**:
- Active sessions (last 15 minutes)
- Current rate limits per API key
- Live metrics (requests/sec, error rates)
- Circuit breaker states
- Active test execution state

**Characteristics**:
- Built from EventCore events via direct subscription
- Fixed memory budget (configurable, default 1GB)
- LRU eviction for session data
- Crash recovery via event replay from checkpoint

### Tier 2: PostgreSQL Projections (Interactive)

**Purpose**: Serve interactive queries and recent history

**Technology**: PostgreSQL with optimized schemas

**Data Included**:
- Session data (configurable retention, default 30 days)
- Test cases and execution history
- User audit logs
- Recent metrics (hourly aggregations)
- Configuration data

**Characteristics**:
- Built via EventCore's PostgreSQL projection adapter
- Denormalized for query performance
- Partitioned by time for efficient cleanup
- Indexed for common query patterns

**Key Tables**:
```sql
-- Optimized for session queries
CREATE TABLE sessions (
    id UUID PRIMARY KEY,
    app_id UUID NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL,
    status TEXT NOT NULL,
    metadata JSONB,
    INDEX idx_app_time (app_id, timestamp DESC)
);

-- Optimized for test queries
CREATE TABLE test_executions (
    id UUID PRIMARY KEY,
    test_id UUID NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL,
    status TEXT NOT NULL,
    duration INT NOT NULL,
    INDEX idx_test_time (test_id, timestamp DESC)
);

-- Pre-aggregated metrics
CREATE TABLE metrics_hourly (
    app_id UUID NOT NULL,
    hour TIMESTAMPTZ NOT NULL,
    requests BIGINT NOT NULL,
    errors BIGINT NOT NULL,
    p50_latency INT NOT NULL,
    p99_latency INT NOT NULL,
    PRIMARY KEY (app_id, hour)
);
```

### Tier 3: Elasticsearch Projections (Analytical) - Post-MVP

**Purpose**: Full-text search and complex analytics

**Technology**: Elasticsearch

**Data Included**:
- Full conversation content (searchable)
- Detailed request/response data
- Long-term metrics
- ML training datasets

**Characteristics**:
- Built asynchronously from EventCore events
- Optimized for search and aggregations
- Longer retention (configurable, default 1 year)
- Can be rebuilt from events if needed

## Data Flow

```
EventCore Events
    ├── Tier 1 Projector → In-Memory Store
    ├── Tier 2 Projector → PostgreSQL
    └── Tier 3 Projector → Elasticsearch (Post-MVP)
```

## Query Routing

The API layer automatically routes queries to the appropriate tier:

1. Check Tier 1 for active/recent data
2. Fall back to Tier 2 for historical data
3. Use Tier 3 for search and analytics (Post-MVP)

## Consequences

### Positive

- Optimal performance for each query pattern
- Can scale tiers independently
- Graceful degradation (can serve from higher tiers if lower tiers unavailable)
- Clear separation of concerns
- Can add new tiers without disrupting existing ones

### Negative

- Data duplication across tiers
- Eventual consistency between tiers
- Complex cache invalidation
- Higher operational overhead
- More complex debugging

### Mitigation Strategies

1. **Consistency Monitoring**: Track lag between tiers
2. **Automated Failover**: Route queries to next tier if primary fails
3. **Projection Health Checks**: Ensure projections stay in sync
4. **Clear Documentation**: Query patterns and tier selection
5. **Unified Query API**: Hide complexity from API consumers

## Alternatives Considered

1. **Single PostgreSQL Database**
   - Simpler but can't meet <10ms requirement for hot data
   - Rejected: Performance requirements too diverse

2. **Redis + PostgreSQL**
   - Redis for Tier 1, PostgreSQL for everything else
   - Rejected: Redis adds operational complexity, persistence concerns

3. **Time-series Database**
   - Use specialized TSDB for metrics
   - Rejected: Adds another system, PostgreSQL sufficient for MVP

4. **Event Store Only**
   - Query events directly with smart caching
   - Rejected: Can't meet latency requirements

5. **Denormalized Single Store**
   - One big denormalized database
   - Rejected: Can't optimize for competing access patterns

## Implementation Notes

- Start with Tier 1 and 2 for MVP
- Use PostgreSQL LISTEN/NOTIFY for cache invalidation
- Implement health endpoints for each tier
- Monitor memory usage in Tier 1 carefully
- Design for easy addition of new projections

## Related Decisions

- ADR-0007: EventCore as Central Audit Mechanism (source of projections)
- ADR-0002: Storage Solution (PostgreSQL as primary store)
- ADR-0008: Dual-path Architecture (projections built in audit path)
