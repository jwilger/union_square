# Overall Architecture Pattern

- Status: accepted
- Deciders: John Wilger, Technical Architecture Team
- Date: 2024-07-15

## Context and Problem Statement

Union Square needs to act as a transparent proxy for LLM API calls while adding minimal latency (< 5ms target) and providing comprehensive observability features. The architecture must support high concurrency across different sessions, be resilient to failures, and allow applications to bypass the proxy if needed. The system needs to handle both data collection and analysis without impacting the critical path of LLM API requests.

## Decision Drivers

- **Performance**: Must add < 5ms latency to API calls
- **Resilience**: Must not become a single point of failure
- **Scalability**: Must handle high request rates with concurrent sessions
- **Maintainability**: Clear separation of concerns for long-term development
- **Type Safety**: Leverage Rust's type system to prevent runtime errors
- **Testability**: Architecture must support comprehensive testing

## Considered Options

- **Option 1**: Traditional Three-Tier Architecture (Presentation, Business Logic, Data)
- **Option 2**: Microservices with Event-Driven Communication
- **Option 3**: Functional Core, Imperative Shell with Event-Driven Recording
- **Option 4**: Actor-Based Architecture (similar to Erlang/Elixir OTP)

## Decision Outcome

Chosen option: **"Functional Core, Imperative Shell with Event-Driven Recording"** because it provides the best balance of performance, maintainability, and type safety while allowing us to keep the proxy path extremely lightweight.

### Architecture Components

1. **Proxy Layer** (Imperative Shell)
   - Thin HTTP proxy using Axum/Tower
   - Minimal processing in request path
   - Fire-and-forget event emission for recording

2. **Domain Core** (Functional Core)
   - Pure functions for business logic
   - Type-safe domain models using newtypes
   - No side effects or I/O

3. **Recording Pipeline** (Event-Driven)
   - Async channel-based event processing
   - Decoupled from proxy path
   - Handles persistence and analysis

4. **Web Interface** (Leptos)
   - Server-side rendering with reactive components
   - Shares domain types with backend

### Positive Consequences

- **Performance**: Proxy path does minimal work, just forwards requests and emits events
- **Type Safety**: Domain logic is pure and fully type-checked at compile time
- **Testability**: Pure functions are easy to test, side effects are isolated
- **Scalability**: Event-driven recording can be scaled independently
- **Maintainability**: Clear boundaries between pure and impure code

### Negative Consequences

- **Complexity**: Developers need to understand functional programming concepts
- **Event Ordering**: Must carefully handle event ordering for session reconstruction
- **Memory Usage**: In-flight events may consume memory under high load

## Pros and Cons of the Options

### Option 1: Traditional Three-Tier Architecture

Simple, well-understood pattern with clear layers.

- Good, because it's familiar to most developers
- Good, because it has clear separation between layers
- Bad, because it typically involves synchronous processing that would add latency
- Bad, because business logic often becomes entangled with I/O operations

### Option 2: Microservices with Event-Driven Communication

Separate services for proxy, recording, analysis, and UI.

- Good, because services can be scaled independently
- Good, because failure isolation between services
- Bad, because adds network latency between services
- Bad, because increases operational complexity significantly
- Bad, because distributed tracing becomes necessary for debugging

### Option 3: Functional Core, Imperative Shell with Event-Driven Recording

Pure business logic with I/O at the edges, async recording pipeline.

- Good, because proxy path is extremely lightweight
- Good, because business logic is pure and testable
- Good, because leverages Rust's type system effectively
- Good, because recording doesn't block request forwarding
- Bad, because requires discipline to maintain pure/impure boundaries
- Bad, because event processing adds some complexity

### Option 4: Actor-Based Architecture

Message-passing actors similar to Erlang/Elixir OTP.

- Good, because excellent fault isolation
- Good, because natural concurrency model
- Bad, because Rust's actor libraries are less mature than BEAM
- Bad, because message passing overhead could impact latency
- Bad, because debugging actor systems can be challenging

## Links

- Influences [ADR-0002](0002-storage-solution.md) - Storage must support event-driven writes
- Influences [ADR-0003](0003-proxy-implementation.md) - Proxy design follows from this pattern
- Related to [ADR-0004](0004-type-system.md) - Type system supports functional core