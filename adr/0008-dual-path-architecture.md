# ADR-0008: Dual-path Architecture (Hot Path vs Audit Path)

- Status: accepted
- Deciders: John Wilger, Technical Architecture Team
- Date: 2025-07-15

## Context

Union Square acts as a proxy between applications and LLM providers, requiring us to:
1. Forward requests to LLM providers with minimal latency (<5ms overhead)
2. Capture comprehensive audit data for every request/response
3. Provide analytics, testing, and debugging capabilities
4. Never become a single point of failure

These requirements create competing concerns:
- Fast forwarding requires minimal processing
- Comprehensive capture requires significant processing
- Reliability requires the proxy to be optional

## Decision

We will implement a dual-path architecture that separates concerns:

1. **Hot Path (Request Forwarding)**
   - Receives incoming LLM API requests
   - Performs minimal validation
   - Immediately forwards to the appropriate provider
   - Captures raw request/response data in a ring buffer
   - Returns the provider's response to the caller
   - Target: <5ms overhead, <1μs for ring buffer write

2. **Audit Path (Async Processing)**
   - Reads from the ring buffer asynchronously
   - Processes captured data into EventCore events
   - Handles all non-critical operations:
     - Session tracking
     - Analytics calculation
     - Test case extraction
     - Privacy compliance (PII detection)
     - Cost tracking
     - Error analysis

### Data Flow

```
Client Request → Hot Path → LLM Provider
       ↓            ↓            ↓
  Ring Buffer ← Raw Data ← Provider Response
       ↓                         ↓
  Audit Path                Client Response
       ↓
  EventCore Events
```

### Key Design Principles

1. **Fire and Forget**: Hot path never waits for audit path
2. **Graceful Degradation**: If ring buffer is full, drop audit data rather than block
3. **Eventual Consistency**: Analytics and session data are eventually consistent
4. **Bypass Capability**: Clients can fallback to direct provider connections

## Consequences

### Positive

- Minimal latency impact on LLM API calls
- System remains responsive even under heavy audit load
- Can scale hot path and audit path independently
- Audit processing can be paused/resumed without affecting traffic
- Supports complex processing without impacting response times

### Negative

- Audit data may be lost if ring buffer overflows
- Debugging requires correlating across two paths
- Real-time analytics have eventual consistency delays
- Additional complexity in deployment and monitoring
- Requires careful capacity planning for ring buffer

### Mitigation Strategies

1. **Ring Buffer Monitoring**: Alert on high watermarks before overflow
2. **Backpressure**: Slow audit processing triggers capacity scaling
3. **Correlation IDs**: Unique IDs link hot path and audit path data
4. **Health Checks**: Monitor both paths independently
5. **Replay Capability**: Store raw data for replay if audit processing fails

## Alternatives Considered

1. **Single Path Processing**
   - Process everything inline
   - Rejected: Would violate <5ms latency requirement

2. **Queue-based Separation**
   - Use message queue between paths
   - Rejected: Adds external dependency and latency

3. **Sidecar Pattern**
   - Run audit processing as separate process
   - Rejected: More complex deployment, harder to maintain <1μs handoff

4. **Database Write-through**
   - Write directly to database from hot path
   - Rejected: Database writes would exceed latency budget

## Related Decisions

- ADR-0007: EventCore as Central Audit Mechanism (audit path output)
- ADR-0009: Ring Buffer Pattern for Event Recording (handoff mechanism)
- ADR-0010: Tiered Projection Strategy (audit path processing)
