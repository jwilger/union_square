# 0003. Proxy Implementation Strategy

- Status: accepted
- Deciders: John Wilger, Technical Architecture Team
- Date: 2025-07-14

## Context and Problem Statement

Union Square must proxy LLM API calls with minimal latency (< 5ms target) while capturing comprehensive session data. The proxy needs to handle multiple provider APIs (OpenAI, Anthropic, Bedrock, Vertex AI) with their unique protocols, support streaming responses, and allow applications to bypass the proxy if it becomes unavailable. The implementation must balance performance, maintainability, and extensibility.

## Decision Drivers

- **Ultra-low latency**: < 5ms overhead for proxy operations
- **Streaming support**: Handle SSE and chunked responses efficiently
- **Provider compatibility**: Support different API patterns and protocols
- **Bypass capability**: Applications must be able to fall back to direct calls
- **Async recording**: Capture data without blocking the request path
- **Extensibility**: Easy to add new providers

## Considered Options

- **Option 1**: Generic HTTP proxy with provider detection
- **Option 2**: Provider-specific proxy implementations
- **Option 3**: Middleware-based Tower service stack
- **Option 4**: Sidecar proxy pattern (like Envoy)

## Decision Outcome

Chosen option: **"Middleware-based Tower service stack"** because it provides the best balance of performance, composability, and type safety while leveraging Rust's async ecosystem effectively.

### Architecture Components

1. **Tower Service Stack**
   ```
   Request → Router → Provider Detection → Recording → Forward → Response
   ```

2. **Provider-Specific Handlers**
   - Trait-based abstraction for providers
   - Provider detection from URL path
   - Streaming response handling per provider

3. **Async Recording Pipeline**
   - MPSC channel for events
   - Non-blocking event emission
   - Batched writes to storage

### Implementation Design

```rust
// Core proxy trait
trait ProxyProvider: Send + Sync {
    fn match_path(&self, path: &str) -> bool;
    fn transform_request(&self, req: Request<Body>) -> Result<ProxiedRequest>;
    fn handle_response(&self, resp: Response<Body>) -> Result<ProxiedResponse>;
}

// Tower middleware for recording
struct RecordingMiddleware<S> {
    inner: S,
    events: mpsc::Sender<RecordingEvent>,
}

// Streaming response handler
async fn proxy_streaming_response(
    mut response: Response<Body>,
    mut events: mpsc::Sender<RecordingEvent>,
) -> Result<Response<Body>> {
    let (tx, body) = Body::channel();

    tokio::spawn(async move {
        let mut full_response = Vec::new();
        while let Some(chunk) = response.body_mut().data().await {
            let chunk = chunk?;
            full_response.extend_from_slice(&chunk);
            tx.send_data(chunk).await?;
        }
        events.send(RecordingEvent::Response(full_response)).await?;
    });

    Ok(Response::new(body))
}
```

### Positive Consequences

- **Performance**: Tower's zero-cost abstractions minimize overhead
- **Composability**: Middleware can be mixed and matched
- **Type Safety**: Compile-time verification of service composition
- **Streaming**: Natural support for async streaming
- **Testing**: Each middleware can be tested independently

### Negative Consequences

- **Complexity**: Tower service composition has a learning curve
- **Debugging**: Layered middlewares can make debugging harder
- **Memory**: Buffering responses for recording uses memory

## Pros and Cons of the Options

### Option 1: Generic HTTP proxy with provider detection

Single proxy implementation detecting providers dynamically.

- Good, because simple to implement initially
- Good, because single code path to maintain
- Bad, because provider-specific logic becomes tangled
- Bad, because harder to optimize for specific providers
- Bad, because type safety is lost with dynamic dispatch

### Option 2: Provider-specific proxy implementations

Separate proxy implementation for each provider.

- Good, because each can be optimized specifically
- Good, because clear separation of provider logic
- Bad, because significant code duplication
- Bad, because harder to maintain consistency
- Bad, because more complex routing logic

### Option 3: Middleware-based Tower service stack

Composable middleware layers using Tower.

- Good, because leverages Rust async ecosystem
- Good, because highly composable and testable
- Good, because zero-cost abstractions
- Good, because natural streaming support
- Bad, because Tower has a learning curve
- Bad, because more complex initial setup

### Option 4: Sidecar proxy pattern

Separate proxy process like Envoy with custom filters.

- Good, because language agnostic
- Good, because proven in production
- Bad, because adds operational complexity
- Bad, because harder to maintain custom logic
- Bad, because additional network hop

## Implementation Details

### URL Routing Strategy

```
https://proxy.example.com/openai/v1/chat/completions → OpenAI
https://proxy.example.com/anthropic/v1/messages → Anthropic
https://proxy.example.com/bedrock/model/invoke → Bedrock
https://proxy.example.com/vertex-ai/v1/projects/... → Vertex AI
```

### Headers for Session Tracking

```
X-Union-Square-Session-ID: unique-session-id
X-Union-Square-Metadata: {"user_id": "123", "feature": "chat"}
X-Union-Square-Do-Not-Record: true  # Privacy control
```

### Circuit Breaker Pattern

Applications can implement fallback:

```rust
// Application code
let response = match proxy_client.post(url).send().await {
    Ok(resp) => resp,
    Err(_) => {
        // Fallback to direct provider call
        provider_client.post(direct_url).send().await?
    }
};
```

## Links

- Influenced by [ADR-0001](0001-overall-architecture-pattern.md) - Implements imperative shell pattern
- Influenced by [ADR-0002](0002-storage-solution.md) - Async recording to PostgreSQL
- Influences [ADR-0004](0004-type-system.md) - Type-safe provider abstractions
