# 0021. Unified Audit Command Architecture

Date: 2025-07-28

## Status

Accepted

## Context

The initial EventCore integration implemented four separate command types for audit events:
- `RecordRequestReceived`
- `RecordRequestForwarded`
- `RecordResponseReceived`
- `RecordResponseReturned`

This approach led to:
- Significant code duplication across commands
- Complex coordination logic in the proxy layer
- Difficulty in maintaining consistency across command implementations
- Increased cognitive load for developers

The functional-architecture-expert agent identified this as unnecessary complexity that violated the principle of simplicity.

## Decision

We will consolidate all audit event processing into a single `RecordAuditEvent` command that:
- Accepts an `AuditEventType` enum to differentiate between event types
- Maintains a unified state machine for request lifecycle
- Shares common logic for stream management and event emission
- Delegates type-specific logic based on the event type

## Consequences

### Positive

- **Reduced code duplication**: Common logic is shared across all audit event types
- **Simplified mental model**: Developers only need to understand one command
- **Easier maintenance**: Changes to audit logic happen in one place
- **Better testability**: Single command with clear inputs and outputs
- **Improved composability**: Event type becomes data, not structure

### Negative

- **Single responsibility concern**: One command handles multiple event types
- **Potential for growth**: Command may become complex if many event types are added

### Mitigation

- Keep event-specific logic minimal and well-separated within the command
- Consider splitting if the command grows beyond reasonable complexity
- Use the type system to ensure event types are handled exhaustively

## References

- EventCore documentation on command design
- Functional programming principles of simplicity
- PR #153 implementation
