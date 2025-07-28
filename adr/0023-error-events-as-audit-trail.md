# 0023. Error Events as Audit Trail

Date: 2025-07-28

## Status

Accepted

## Context

In event-sourced systems, errors and failures are important parts of the system's history. The initial implementation would log errors or return them as command results, but they were not persisted as events.

This approach had limitations:
- Errors were ephemeral and could be lost
- No audit trail of what went wrong and when
- Difficult to analyze failure patterns over time
- Incomplete system history for debugging

The event-sourcing-architect agent advocated for treating errors as first-class events in the event stream.

## Decision

We will emit specific error events whenever failures occur:
- `LlmRequestParsingFailed`: When request body parsing fails
- `InvalidStateTransition`: When events arrive in invalid order
- `AuditEventProcessingFailed`: For unhandled audit event types

Error events will:
- Contain full context (request IDs, error messages, timestamps)
- Be emitted to the same streams as success events
- Use fallback data to ensure processing can continue
- Maintain the complete audit trail

## Consequences

### Positive

- **Complete audit trail**: All failures become part of permanent history
- **Debugging support**: Full context available for investigating issues
- **Pattern analysis**: Can identify recurring failures over time
- **Compliance**: Maintains complete record for audit purposes
- **Graceful degradation**: System continues operating despite errors

### Negative

- **Event volume**: More events in the stream
- **Storage overhead**: Error events consume storage
- **Complexity**: Additional event types to handle in projections

### Design Decisions

- Error events are emitted alongside success events, not to separate error streams
- Parsing failures include both the error and fallback data used
- Invalid state transitions include the current state and attempted transition

## References

- Event Sourcing pattern documentation
- "Your Mouse is a Database" - Erik Meijer
- PR #153 implementation
