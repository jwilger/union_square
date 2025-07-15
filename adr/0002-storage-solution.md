# Storage Solution for Session Data

- Status: accepted
- Deciders: John Wilger, Technical Architecture Team
- Date: 2024-07-15

## Context and Problem Statement

Union Square needs to store large volumes of LLM interaction data with high write throughput, support complex queries for analytics, maintain data for configurable retention periods, and provide fast retrieval for session replay and test execution. The storage solution must handle both structured metadata and unstructured conversation content while supporting the event-driven architecture established in ADR-001.

## Decision Drivers

- **Write Performance**: High throughput for concurrent session recording
- **Query Flexibility**: Support complex queries for analytics and search
- **Scalability**: Handle growing data volumes over time
- **Cost Efficiency**: Reasonable storage costs for potentially large datasets
- **Operational Simplicity**: Easy to deploy and maintain in self-hosted environments
- **Data Retention**: Efficient deletion of expired data
- **ACID Compliance**: Ensure data consistency for financial/compliance use cases

## Considered Options

- **Option 1**: PostgreSQL with JSONB for flexibility
- **Option 2**: ClickHouse for time-series analytics
- **Option 3**: Event Store (EventStore DB or similar)
- **Option 4**: Hybrid approach (PostgreSQL + S3/MinIO)
- **Option 5**: Apache Cassandra for distributed storage

## Decision Outcome

Chosen option: **"PostgreSQL with JSONB for flexibility"** initially, with a clear migration path to **"Hybrid approach (PostgreSQL + S3/MinIO)"** as data volumes grow. This provides the best balance of operational simplicity, query flexibility, and proven reliability while allowing future optimization.

### Initial Implementation (PostgreSQL Only)

1. **Session Metadata**: Relational tables with indexes
2. **Conversation Content**: JSONB columns for flexibility
3. **Events Table**: Append-only event log for recording pipeline
4. **Partitioning**: Time-based partitioning for efficient retention

### Future Migration Path (Hybrid)

1. **Hot Data**: Recent sessions in PostgreSQL
2. **Cold Data**: Archived sessions in S3/MinIO
3. **Metadata**: Always in PostgreSQL for fast queries
4. **Transparent Access**: Application layer handles location

### Positive Consequences

- **Operational Simplicity**: Single database to manage initially
- **Query Power**: Full SQL with JSONB operators for complex queries
- **Ecosystem**: Excellent Rust support via sqlx
- **Reliability**: Proven in production at scale
- **Flexibility**: JSONB allows schema evolution
- **Migration Path**: Clear path to hybrid approach when needed

### Negative Consequences

- **Storage Cost**: Less efficient than columnar stores for large datasets
- **Write Scaling**: May need connection pooling and write batching
- **JSONB Performance**: Complex queries on large JSONB fields can be slow

## Pros and Cons of the Options

### Option 1: PostgreSQL with JSONB

Relational database with JSON support for flexible schema.

- Good, because mature and well-understood
- Good, because excellent Rust ecosystem support
- Good, because supports complex queries with indexes
- Good, because ACID compliant for consistency
- Bad, because less storage efficient than columnar databases
- Bad, because may require tuning for high write loads

### Option 2: ClickHouse

Columnar database optimized for analytics.

- Good, because excellent compression for time-series data
- Good, because very fast analytical queries
- Good, because designed for high write throughput
- Bad, because limited Rust ecosystem support
- Bad, because eventual consistency model
- Bad, because more complex to operate

### Option 3: Event Store

Purpose-built for event sourcing.

- Good, because designed for event-driven architectures
- Good, because built-in projections for read models
- Good, because immutable append-only storage
- Bad, because limited query flexibility
- Bad, because smaller ecosystem
- Bad, because requires different mental model

### Option 4: Hybrid (PostgreSQL + S3/MinIO)

PostgreSQL for hot data, object storage for cold data.

- Good, because optimizes storage costs
- Good, because unlimited scalability for old data
- Good, because can use PostgreSQL foreign data wrappers
- Bad, because increases operational complexity
- Bad, because requires migration logic
- Bad, because two systems to monitor

### Option 5: Apache Cassandra

Distributed NoSQL database.

- Good, because horizontally scalable
- Good, because high write throughput
- Good, because tunable consistency
- Bad, because limited query flexibility
- Bad, because complex to operate
- Bad, because requires careful data modeling

## Implementation Details

### Schema Design (PostgreSQL)

```sql
-- Core tables
CREATE TABLE applications (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE TABLE sessions (
    id UUID PRIMARY KEY,
    application_id UUID NOT NULL REFERENCES applications(id),
    session_identifier TEXT NOT NULL,
    metadata JSONB NOT NULL DEFAULT '{}',
    started_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
) PARTITION BY RANGE (started_at);

CREATE TABLE events (
    id BIGSERIAL PRIMARY KEY,
    session_id UUID NOT NULL,
    event_type TEXT NOT NULL,
    event_data JSONB NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL
) PARTITION BY RANGE (occurred_at);

-- Indexes for common queries
CREATE INDEX idx_sessions_app_identifier ON sessions(application_id, session_identifier);
CREATE INDEX idx_sessions_metadata ON sessions USING GIN(metadata);
CREATE INDEX idx_events_session_time ON events(session_id, occurred_at);
```

### Connection Management

- Use PgBouncer for connection pooling
- Configure for high-concurrency writes
- Separate read/write connection pools

## Links

- Influenced by [ADR-0001](0001-overall-architecture-pattern.md) - Supports event-driven recording
- Influences [ADR-0003](0003-proxy-implementation.md) - Proxy must handle async writes
- Related to future ADR on data retention policies