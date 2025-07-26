# 0007. EventCore as Central Audit Mechanism

- Status: accepted
- Deciders: John Wilger, Claude
- Date: 2025-07-15

## Context and Problem Statement

Union Square needs to maintain comprehensive audit trails for both operational data (LLM interactions) and configuration changes to meet regulatory compliance requirements. The system must be able to answer questions like "What was the exact system state when this incident occurred?" and "Who made this configuration change and why?" How should we implement a unified audit mechanism that provides complete traceability while maintaining the performance requirements of the proxy service?

## Decision Drivers

- **Regulatory Compliance**: Need complete audit trails for all system changes and operations
- **Performance**: Must not impact the <5ms proxy latency requirement
- **Consistency**: Unified approach for both operational and configuration auditing
- **Temporal Queries**: Ability to reconstruct exact system state at any point in time
- **Change Authorization**: Track who approved changes with supporting documentation
- **Debugging**: Need to understand system behavior during incidents

## Considered Options

1. **EventCore for Everything**: Use event sourcing for all data including configuration
2. **EventCore for Operations Only**: Event source LLM interactions, use traditional CRUD for configuration
3. **Separate Audit Systems**: Different audit mechanisms for different concerns
4. **Traditional Audit Tables**: Use PostgreSQL triggers and audit tables

## Decision Outcome

Chosen option: "EventCore for Everything", because it provides a unified, immutable audit trail that naturally supports regulatory compliance requirements and temporal queries. The benefits of complete auditability outweigh the additional complexity.

### Positive Consequences

- **Complete Audit Trail**: Every change to the system is captured as an event with full context
- **Temporal Queries**: Can reconstruct exact system state at any moment for compliance audits
- **Unified Approach**: Single pattern for all audit needs reduces complexity
- **Natural Authorization**: Events carry metadata about approvers, tickets, reasons
- **Debugging Power**: Can replay events to understand exactly what happened
- **Compliance by Design**: Audit requirements are met inherently, not bolted on

### Negative Consequences

- **Bootstrap Complexity**: Need minimal config to start the system that loads config
- **Operational Learning Curve**: Can't just edit config files in emergencies
- **Testing Setup**: Tests need to build event streams for configuration
- **Initial Development Time**: More upfront work to implement event sourcing

## Pros and Cons of the Options

### EventCore for Everything

Use event sourcing for all auditable data including LLM interactions, configuration, access control, and API keys.

- Good, because provides complete immutable audit trail
- Good, because enables temporal queries across all system aspects
- Good, because enforces proper change control (no ad-hoc config edits)
- Good, because unified pattern reduces cognitive load
- Good, because naturally supports compliance requirements
- Bad, because requires bootstrap configuration outside event store
- Bad, because increases initial implementation complexity
- Bad, because emergency changes require going through event system

### EventCore for Operations Only

Event source only LLM interactions, use traditional CRUD for configuration with separate audit logging.

- Good, because simpler configuration management
- Good, because allows direct config changes in emergencies
- Good, because reduces scope of event sourcing complexity
- Bad, because two different audit mechanisms to maintain
- Bad, because configuration audit trail may be incomplete
- Bad, because harder to correlate config state with operational events
- Bad, because temporal queries require joining different data models

### Separate Audit Systems

Use different audit mechanisms optimized for each domain (e.g., EventCore for operations, audit tables for config).

- Good, because can optimize each mechanism for its use case
- Good, because teams can work independently on different parts
- Bad, because multiple patterns increase overall complexity
- Bad, because difficult to get unified view of system state
- Bad, because more code to maintain and test
- Bad, because compliance audits need to understand multiple systems

### Traditional Audit Tables

Use PostgreSQL triggers to maintain audit tables for all changes.

- Good, because familiar pattern for most developers
- Good, because can query audit data with standard SQL
- Good, because integrates with existing PostgreSQL choice
- Bad, because triggers can impact performance
- Bad, because audit tables can be modified (not immutable)
- Bad, because no built-in event replay capability
- Bad, because harder to maintain consistency as schema evolves

## Implementation Approach

1. **Event Categories**:

   - **Operational Events**: SessionStarted, RequestRecorded, ResponseRecorded
   - **Configuration Events**: ApiKeyAdded, ApiKeyRevoked, RateLimitChanged
   - **Access Control Events**: ModelAccessGranted, UserPermissionChanged
   - **System Events**: ServiceStarted, ServiceShutdown, HealthCheckFailed

2. **Architecture Boundaries**:

   - EventCore never touches the proxy request path
   - Events emitted to async channels for processing
   - Cached projections for frequently accessed configuration
   - Bootstrap config (DB connection, ports) remains file-based

3. **Event Metadata**:
   - Timestamp
   - Actor (user/service that initiated change)
   - Authorization (approval ticket, policy reference)
   - Reason (human-readable explanation)
   - Correlation ID (link related events)

## Links

- Refined by [ADR-0001](0001-overall-architecture-pattern.md) - Aligns with event-driven recording architecture
- Refined by [ADR-0002](0002-storage-solution.md) - Uses PostgreSQL with eventcore-postgres adapter
- Refined by [ADR-0004](0004-type-system-and-domain-modeling.md) - Events are strongly typed domain concepts
