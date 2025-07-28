# Audit Command Integration Guide

This document explains how to integrate the new LLM request parsing functionality into the audit path.

## Overview

The audit system now supports parsing actual LLM request data instead of using placeholders. This is achieved through:

1. **LLM Request Parser** (`llm_request_parser.rs`) - Parses various LLM provider formats
2. **Audit Buffer** (`audit_buffer.rs`) - Buffers request/response chunks until complete
3. **ProcessRequestBody Command** - Processes complete request bodies and emits proper events

## Integration Steps

### 1. Update the Audit Path Handler

In your audit path handler, you'll need to:

```rust
use crate::domain::commands::{
    audit_buffer::AuditBufferManager,
    ProcessRequestBody,
    RecordRequestReceived,
    RecordRequestForwarded,
    RecordResponseReceived,
};

struct AuditPathHandler {
    buffer_manager: AuditBufferManager,
    event_store: EventCoreService,
    // ... other fields
}

impl AuditPathHandler {
    async fn handle_audit_event(&mut self, event: AuditEvent) -> Result<()> {
        match &event.event_type {
            AuditEventType::RequestReceived { method, uri, headers, body_size } => {
                // Store metadata for later use when body is complete
                self.store_request_metadata(event.request_id, event.clone());

                // Don't emit LlmRequestReceived yet - wait for body
            }

            AuditEventType::RequestChunk { offset, data } => {
                // Buffer the chunk
                self.buffer_manager.add_request_chunk(
                    event.request_id,
                    offset.clone(),
                    data.clone()
                );

                // Check if we have the complete body
                if let Some(body) = self.buffer_manager.get_complete_request_body(&event.request_id) {
                    // Get the stored metadata
                    if let Some(metadata) = self.get_request_metadata(&event.request_id) {
                        // Process the complete request
                        self.process_complete_request(metadata, body).await?;
                    }
                }
            }

            AuditEventType::RequestBody { content, truncated: _ } => {
                // Handle non-chunked body
                if let Some(metadata) = self.get_request_metadata(&event.request_id) {
                    self.process_complete_request(metadata, content.clone()).await?;
                }
            }

            // ... handle other event types
        }

        Ok(())
    }

    async fn process_complete_request(
        &self,
        metadata: AuditEvent,
        body: Vec<u8>
    ) -> Result<()> {
        if let AuditEventType::RequestReceived { method, uri, headers, .. } = &metadata.event_type {
            let command = ProcessRequestBody {
                session_stream: StreamId::try_new(format!("session-{}", metadata.session_id))?,
                request_stream: StreamId::try_new(format!("request-{}", metadata.request_id))?,
                request_id: metadata.request_id,
                session_id: SessionId::new(*metadata.session_id.as_ref()),
                method: method.clone(),
                uri: uri.clone(),
                headers: headers.clone(),
                body,
                timestamp: Timestamp::try_new(metadata.timestamp)?,
            };

            self.event_store.execute_command(command).await?;
        }

        Ok(())
    }
}
```

### 2. Alternative: Use RecordRequestReceived with Body

If you prefer to keep the existing flow, you can use `RecordRequestReceived::with_body()`:

```rust
match &event.event_type {
    AuditEventType::RequestReceived { .. } => {
        let mut command = RecordRequestReceived::from_audit_event(&event)?;

        // If you have the body available immediately
        if let Some(body) = get_request_body_somehow() {
            command = command.with_body(&body);
        }

        self.event_store.execute_command(command).await?;
    }
}
```

## Supported LLM Formats

The parser automatically detects and handles:

1. **OpenAI Format** (GPT models)
   - Endpoint: `/v1/chat/completions`, `/v1/completions`
   - Extracts: model, messages/prompt, parameters

2. **Anthropic Format** (Claude models)
   - Endpoint: `/v1/messages`
   - Extracts: model (from body or header), messages/prompt, parameters

3. **Bedrock Format** (AWS Bedrock)
   - Endpoint: `/bedrock/*`, `/invoke`
   - Model extracted from URI
   - Supports various prompt field names

## Error Handling

If parsing fails, the system automatically creates a fallback request with:
- Provider: "unknown"
- Model: "unknown-model"
- Prompt: Error message explaining the parsing failure
- Empty parameters

This ensures the event stream continues even when encountering unknown formats.

## Testing

See the test cases in:
- `audit_commands.rs` - Integration tests
- `llm_request_parser.rs` - Parser unit tests
- `audit_buffer.rs` - Buffer management tests
