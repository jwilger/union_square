# Rule: Acceptance Test Boundaries

Acceptance tests must verify behavior at the system's external boundaries, not internal implementation.

## Boundaries

For Union Square, the external boundaries are:
- **HTTP API** — Requests in, responses out
- **Event Store** — Events persisted and retrievable
- **External LLM Providers** — Requests forwarded, responses received

## What to Test

- Given an HTTP request to the proxy endpoint, the request is forwarded to the provider
- Given a provider response, the proxy returns it to the client
- Given a series of interactions, a session aggregate is built correctly
- Given a completed session, test cases can be extracted

## What NOT to Test

- Internal function behavior (use unit tests)
- Database query details (use integration tests)
- Event serialization format (use unit tests)
- Specific error message wording (test error type, not text)

## Test Doubles

- Use mock servers for external LLM providers (e.g., `mockito`)
- Use `InMemoryEventStore` only for unit tests, not acceptance tests
- Use a real (test) PostgreSQL database for acceptance tests

## Enforcement

- Code review by `event-sourcing-test-architect`
