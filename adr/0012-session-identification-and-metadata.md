# ADR-0012: Session Identification and Metadata Strategy

## Status

Accepted

## Context

Union Square needs to track related LLM API calls as coherent sessions for debugging and analysis. Challenges include:

1. LLM APIs don't have a built-in session concept
2. Applications use various architectures (stateless, stateful, microservices)
3. Must not interfere with provider APIs
4. Need to capture arbitrary application-specific metadata
5. Different applications define "session" differently:
   - Chat applications: user conversation
   - Agent systems: task execution
   - RAG systems: query processing pipeline

## Decision

We will use custom HTTP headers for session identification and metadata, designed to be ignored by LLM providers:

### Header Schema

```
X-UnionSquare-Session-Id: <session-identifier>
X-UnionSquare-Parent-Id: <parent-session-id>  # For nested sessions
X-UnionSquare-Metadata-<Key>: <value>         # Arbitrary metadata
X-UnionSquare-User-Id: <user-identifier>      # Optional user tracking
X-UnionSquare-Application-Context: <json>      # Structured metadata
```

### Session ID Requirements

- **Client-generated**: Applications create their own session IDs
- **Format-agnostic**: UUID, incrementing ID, or any string
- **Optional**: Requests without session ID are tracked individually
- **Hierarchical**: Support parent-child relationships for complex workflows

### Metadata Handling

1. **Simple Metadata** (Headers)
   ```
   X-UnionSquare-Metadata-Feature: chat
   X-UnionSquare-Metadata-Version: 2.1.0
   X-UnionSquare-Metadata-Environment: production
   ```

2. **Complex Metadata** (JSON)
   ```
   X-UnionSquare-Application-Context: {
     "workflow": "customer-support",
     "priority": "high",
     "tags": ["billing", "enterprise"],
     "custom_fields": {...}
   }
   ```

### Session Correlation Logic

```rust
struct SessionContext {
    session_id: Option<String>,
    parent_id: Option<String>,
    user_id: Option<String>,
    metadata: HashMap<String, String>,
    application_context: Option<serde_json::Value>,

    // Auto-captured context
    request_id: Uuid,           // Unique per request
    timestamp: DateTime<Utc>,
    source_ip: IpAddr,
    application_id: String,     // From auth context
}

impl SessionContext {
    fn from_headers(headers: &HeaderMap, auth: &AuthContext) -> Self {
        let session_id = headers
            .get("x-unionsquare-session-id")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());

        let metadata = headers
            .iter()
            .filter_map(|(name, value)| {
                name.as_str()
                    .strip_prefix("x-unionsquare-metadata-")
                    .map(|key| (key.to_string(), value.to_str().unwrap_or("").to_string()))
            })
            .collect();

        // Extract other fields and construct SessionContext
        SessionContext {
            session_id,
            parent_id: headers
                .get("x-unionsquare-parent-id")
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_string()),
            user_id: headers
                .get("x-unionsquare-user-id")
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_string()),
            metadata,
            application_context: headers
                .get("x-unionsquare-application-context")
                .and_then(|h| h.to_str().ok())
                .and_then(|s| serde_json::from_str(s).ok()),
            request_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            source_ip: "127.0.0.1".parse().unwrap(), // Should be extracted from request
            application_id: auth.application_id.clone(),
        }
    }
}
```

### Session Lifecycle

1. **Session Start**: First request with new session ID
2. **Session Continuation**: Subsequent requests with same ID
3. **Session Branching**: New session with parent ID reference
4. **Session End**: Implicit (timeout) or explicit (end marker)

### Storage Strategy

Sessions are stored as EventCore events and projected to appropriate tiers:

```rust
enum SessionEvent {
    SessionStarted {
        session_id: String,
        metadata: SessionContext,
    },
    RequestAdded {
        session_id: String,
        request_id: Uuid,
        sequence: u32,
    },
    SessionEnded {
        session_id: String,
        reason: EndReason,
    },
}
```

## Consequences

### Positive

- No provider API interference (X- headers ignored)
- Flexible for different application architectures
- Supports hierarchical workflows
- Applications control their session definition
- Backward compatible (sessions optional)
- Rich metadata without schema restrictions

### Negative

- Header size limits (typically 8KB total)
- No validation of session IDs
- Applications must generate good session IDs
- Potential header parsing overhead
- Session reconstruction complexity

### Mitigation Strategies

1. **Header Validation**: Reject requests exceeding size limits early
2. **Session ID Recommendations**: Document best practices
3. **Metadata Limits**: Limit number of metadata headers
4. **Performance**: Parse headers lazily in hot path
5. **SDK Support**: Provide client libraries for session management

## Alternatives Considered

1. **URL Parameters**
   - Add session ID to query string
   - Rejected: Modifies request signature, visible in logs

2. **Request Body Injection**
   - Add session data to request payload
   - Rejected: Modifies content, breaks signatures

3. **Out-of-Band Correlation**
   - Send session data separately
   - Rejected: Complex, race conditions, not real-time

4. **Provider-Specific Extensions**
   - Use each provider's custom fields
   - Rejected: Not portable, provider lock-in

5. **Proxy-Generated Sessions**
   - Union Square creates sessions automatically
   - Rejected: Can't understand application semantics

## Implementation Notes

- Case-insensitive header matching
- Validate header values are UTF-8
- Document header size limits clearly
- Support URL encoding for header values
- Add session search APIs to query by metadata

## Related Decisions

- ADR-0007: EventCore as Central Audit Mechanism (session events)
- ADR-0010: Tiered Projection Strategy (session storage)
- ADR-0006: Authentication and Authorization (application context)
