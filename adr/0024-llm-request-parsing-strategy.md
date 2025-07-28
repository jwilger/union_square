# 0024. LLM Request Parsing Strategy

Date: 2025-07-28

## Status

Accepted

## Context

The proxy needs to extract metadata from LLM API requests for audit purposes. Different LLM providers (OpenAI, Anthropic, etc.) use different request formats:

- OpenAI uses a `model` field directly
- Anthropic uses nested structures
- Request bodies may be malformed or incomplete
- Parsing failures should not prevent request proxying

The initial implementation used placeholder data, which provided no value for audit analysis.

## Decision

We will implement a multi-provider parsing strategy that:
- Attempts to parse known formats (OpenAI, Anthropic)
- Falls back to safe defaults on parsing failure
- Emits error events when parsing fails
- Extracts model, prompt, and parameters when possible

The parser will:
```rust
pub fn parse_llm_request(
    body: &[u8],
    headers: &Headers,
) -> Result<ParsedLlmRequest, LlmRequestParseError> {
    // Try parsing as different formats
    // Return extracted data or error with context
}
```

Fallback behavior ensures the proxy continues functioning even when parsing fails.

## Consequences

### Positive

- **Multi-provider support**: Handles different LLM API formats
- **Graceful degradation**: Failures don't break request flow
- **Audit completeness**: Best-effort extraction of metadata
- **Extensibility**: Easy to add new provider formats
- **Error visibility**: Parsing failures are tracked as events

### Negative

- **Incomplete data**: Some requests may have fallback values
- **Provider coupling**: Need to update when APIs change
- **Parsing overhead**: Additional processing for each request

### Implementation Notes

- Use serde_json for parsing with permissive error handling
- Provider detection based on URL patterns and request structure
- Fallback values are clearly marked (e.g., "unknown-provider")
- All parsing errors are emitted as events for analysis

## References

- OpenAI API documentation
- Anthropic API documentation
- PR #153 implementation
