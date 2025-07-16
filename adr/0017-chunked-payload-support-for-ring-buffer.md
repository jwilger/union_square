# Chunked Payload Support for Ring Buffer

- Status: proposed
- Deciders: John Wilger, Claude
- Date: 2025-07-15

## Context and Problem Statement

The current ring buffer design (ADR-0009) truncates large payloads to fit within fixed-size slots (default 64KB). This loses data for larger LLM requests/responses, which can exceed 800KB for models with large context windows. Additionally, the system needs configurable strategies for handling buffer overflow - either dropping data to maintain latency guarantees or applying back-pressure for durability.

How can we support arbitrarily large payloads while maintaining sub-microsecond write latency and providing flexible overflow handling?

## Decision Drivers

- Must maintain <1 microsecond write latency for hot path
- Need to support full LLM payloads (up to 800KB+ for large context models)
- Should allow zero-allocation reconstruction on audit path
- Must provide configurable trade-off between latency and durability
- Should align with established patterns (UDP-style packet fragmentation)
- Need graceful handling of partial payloads when buffer wraps

## Considered Options

1. **Chunked Payloads with Header-based Reconstruction** - Split large payloads across multiple slots with headers for reassembly
2. **Dynamic Slot Sizing** - Allow variable-sized slots based on payload size
3. **Secondary Storage for Large Payloads** - Store large payloads externally with references in ring buffer
4. **Increased Fixed Slot Size** - Simply increase slot size to accommodate largest expected payload

## Decision Outcome

Chosen option: "Chunked Payloads with Header-based Reconstruction", because it maintains the performance characteristics of the original design while elegantly solving the large payload problem. This approach is battle-tested in network protocols and provides the flexibility needed for both overflow strategies.

### Positive Consequences

- No payload size limitations
- Maintains lock-free, allocation-free hot path
- Zero-copy reconstruction possible on audit path
- Flexible overflow handling strategies
- Natural alignment with network protocol patterns
- Efficient cache utilization (sequential slots)

### Negative Consequences

- Slightly more complex atomic operations for multi-slot claims
- Need to handle partial payload scenarios
- Additional 24 bytes overhead per slot for headers
- More complex testing scenarios

## Pros and Cons of the Options

### Chunked Payloads with Header-based Reconstruction

Each slot gets a header structure:

```rust
struct SlotHeader {
    payload_id: Uuid,      // Which payload this belongs to
    chunk_seq: u16,        // Chunk number (0-based)
    total_chunks: u16,     // Total expected chunks
    chunk_len: u32,        // This chunk's data length
    flags: u8,             // IS_LAST, IS_FIRST, CONTINUATION
    state: AtomicU8,       // Slot state (EMPTY, WRITING, READY, READING)
}
```

Overflow handling becomes configurable:

```rust
enum OverflowStrategy {
    Drop {
        // Maintain latency guarantee, track metrics
        dropped_payloads: AtomicU64,
    },
    Backpressure {
        // Block until space available
        max_wait_micros: u64,
    },
    Hybrid {
        // Try backpressure up to threshold, then drop
        pressure_threshold_micros: u64,
    },
}
```

- Good, because no size limitations on payloads
- Good, because maintains all performance guarantees from original design
- Good, because allows flexible durability vs latency trade-offs
- Good, because audit path can stream chunks without allocation
- Good, because follows proven UDP/TCP fragmentation patterns
- Bad, because requires coordinated multi-slot atomic claims
- Bad, because partial payloads need special handling

### Dynamic Slot Sizing

Allow slots to have variable sizes based on payload requirements.

- Good, because no wasted space for small payloads
- Good, because no chunking overhead
- Bad, because complex memory management and fragmentation
- Bad, because unpredictable memory access patterns hurt cache performance
- Bad, because requires complex allocation logic in hot path

### Secondary Storage for Large Payloads

Store large payloads in separate storage, keep references in ring buffer.

- Good, because ring buffer stays simple
- Good, because unlimited payload sizes
- Bad, because additional I/O in hot path violates latency requirements
- Bad, because introduces external dependencies
- Bad, because complex consistency management

### Increased Fixed Slot Size

Simply increase slot size to 1MB to accommodate largest payloads.

- Good, because simple - no code changes required
- Good, because maintains current performance characteristics
- Bad, because massive memory waste (most payloads are small)
- Bad, because poor cache utilization
- Bad, because still has a hard limit that could be exceeded

## Implementation Details

### Write Algorithm for Chunked Payloads

```rust
fn write_chunked(payload: &[u8]) -> Result<PayloadId, OverflowError> {
    let payload_id = Uuid::new_v7();
    let total_chunks = (payload.len() + CHUNK_SIZE - 1) / CHUNK_SIZE;
    
    // Atomically claim N consecutive slots
    let start_slot = claim_slots(total_chunks)?;
    
    for (i, chunk) in payload.chunks(CHUNK_SIZE).enumerate() {
        let slot = &slots[(start_slot + i) % slot_count];
        let header = SlotHeader {
            payload_id,
            chunk_seq: i as u16,
            total_chunks: total_chunks as u16,
            chunk_len: chunk.len() as u32,
            flags: if i == 0 { IS_FIRST } else { 0 } 
                 | if i == total_chunks - 1 { IS_LAST } else { 0 },
            state: AtomicU8::new(WRITING),
        };
        
        write_slot(slot, header, chunk);
        slot.header.state.store(READY, Release);
    }
    
    Ok(payload_id)
}
```

### Read Algorithm for Reconstruction

```rust
impl Iterator for ChunkedPayloadReader {
    type Item = &[u8];
    
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let slot = &self.slots[self.current_pos];
            
            if slot.matches_payload(self.payload_id, self.expected_seq) {
                self.expected_seq += 1;
                self.current_pos = (self.current_pos + 1) % self.slot_count;
                return Some(&slot.data[..slot.header.chunk_len as usize]);
            }
            
            // Handle missing chunk
            if self.timeout_exceeded() {
                return None;
            }
        }
    }
}
```

### Buffer Sizing Strategy

Configure based on model context windows:
- Base calculation: `2-3x max_context_size × expected_concurrent_requests`
- Example for GPT-4 (128K tokens ≈ 512KB): 512KB × 3 × 100 requests = 150MB buffer
- Monitor actual usage patterns and adjust

## Links

- Refines [ADR-0009: Ring Buffer Pattern](0009-ring-buffer-pattern.md)
- Related to [ADR-0008: Dual-path Architecture](0008-dual-path-architecture.md)
- Impacts [ADR-0016: Performance Monitoring](0016-performance-monitoring-and-metrics.md) - new metrics needed