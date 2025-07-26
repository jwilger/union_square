# 0009. Ring Buffer Pattern for Event Recording

- Status: accepted
- Deciders: John Wilger, Technical Architecture Team
- Date: 2025-07-15

## Context

The dual-path architecture (ADR-0008) requires a mechanism to hand off data from the hot path to the audit path with minimal overhead. Our requirements are:

1. <1 microsecond write latency from hot path
2. No memory allocations in hot path
3. No blocking operations
4. Handle variable-sized LLM request/response payloads
5. Graceful handling of buffer overflow
6. Support concurrent writers (multiple request threads)

Traditional approaches like queues, channels, or direct database writes would violate our latency requirements.

## Decision

We will implement a lock-free ring buffer specifically designed for our use case:

### Design

1. **Fixed-size Ring Buffer**
   - Pre-allocated memory pool (configurable, default 1GB)
   - Divided into fixed-size slots (default 64KB per slot)
   - Power-of-2 slot count for efficient modulo operations

2. **Lock-free Multi-Producer Single-Consumer (MPSC)**
   - Multiple hot path threads write concurrently
   - Single audit path thread reads sequentially
   - Uses atomic operations for coordination

3. **Slot Structure**
   ```rust
   struct Slot {
       state: AtomicU8,      // EMPTY, WRITING, READY, READING
       size: AtomicU32,      // Actual payload size
       timestamp: u64,       // Capture timestamp
       request_id: Uuid,     // Correlation ID
       data: [u8; SLOT_SIZE] // Raw payload
   }
   ```

4. **Write Algorithm**
   ```
   1. Atomically claim next write position
   2. If slot not EMPTY, increment overflow counter and return
   3. CAS slot state to WRITING
   4. Copy data (with size limit)
   5. Store size and metadata
   6. CAS slot state to READY
   ```

5. **Read Algorithm**
   ```
   1. Check slot at read position
   2. If READY, CAS to READING
   3. Process data
   4. CAS to EMPTY
   5. Advance read position
   ```

### Overflow Handling

- Large payloads are truncated to slot size
- Truncation flag is set in metadata
- Full payloads can be requested via separate async path
- Overflow metrics are tracked for capacity planning

### Memory Layout

```
[Header (4KB)]
[Slot 0 (64KB)] [Slot 1 (64KB)] ... [Slot N (64KB)]
```

Header contains:
- Write position (atomic)
- Read position (atomic)
- Overflow counter (atomic)
- Configuration parameters

## Consequences

### Positive

- Predictable <1μs write latency
- No memory allocations after initialization
- No system calls in hot path
- CPU cache-friendly sequential access
- Graceful degradation under load
- Simple crash recovery (can resume from read position)

### Negative

- Fixed memory overhead (1GB default)
- Large requests/responses need truncation
- Lost data on overflow (by design)
- Single audit reader constraint
- Complex testing of concurrent scenarios

### Mitigation Strategies

1. **Dynamic Sizing**: Monitor typical payload sizes and adjust slot size
2. **Overflow Handling**: Track overflow rate and auto-scale buffer size
3. **Chunking**: Split large payloads across multiple slots if needed
4. **Backup Writer**: Overflow data can write to secondary storage
5. **Monitoring**: Extensive metrics on buffer utilization

## Alternatives Considered

1. **LMAX Disruptor Pattern**
   - More complex, designed for different use case
   - Rejected: Overkill for our simple handoff needs

2. **Channel/Queue Libraries**
   - Standard MPSC channels
   - Rejected: Allocation overhead, unpredictable latency

3. **Memory-mapped Files**
   - Persistent buffer with mmap
   - Rejected: System call overhead, page fault risks

4. **Direct Audit Path Calls**
   - Call audit path directly with async handoff
   - Rejected: Thread pool overhead, allocation costs

5. **Shared Memory with Semaphores**
   - Traditional IPC approach
   - Rejected: System call overhead for synchronization

## Implementation Notes

- Use Rust's `std::sync::atomic` with `Ordering::Relaxed` for counters
- Use `Ordering::AcqRel` for state transitions
- Align slots to cache line boundaries (64 bytes)
- Use `#[repr(C)]` for stable memory layout
- Benchmark with real LLM payloads to tune parameters

## Related Decisions

- ADR-0008: Dual-path Architecture (defines the need for this pattern)
- ADR-0007: EventCore as Central Audit Mechanism (consumer of ring buffer data)
- ADR-0016: Performance Monitoring (ring buffer metrics are critical)
