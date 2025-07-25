# 19. Unsafe Ring Buffer Implementation

Date: 2025-01-25

## Status

Accepted

## Context

The ring buffer is a critical component in Union Square's dual-path architecture, responsible for the hot path to audit path handoff. This handoff must maintain sub-microsecond overhead to meet our <5ms latency guarantee for the hot path.

Initially, we explored safe Rust implementations to maintain memory safety without the complexity of unsafe code:

1. **Mutex-based implementation**: Used `std::sync::Mutex` for synchronization
2. **RwLock-based implementation**: Used `std::sync::RwLock` for reader/writer separation
3. **Crossbeam-based implementation**: Used lock-free data structures from the crossbeam crate

Performance testing revealed significant overhead with these safe implementations:
- Mutex version: 10-50x slower than unsafe version under contention
- RwLock version: 5-20x slower, with reader starvation under heavy writes
- Crossbeam version: 2-5x slower, closest to performance requirements but still insufficient

The performance impact was particularly pronounced under concurrent load, where lock contention or atomic operation overhead became the bottleneck.

## Decision

We will use the unsafe ring buffer implementation with atomic state coordination for the following reasons:

1. **Performance Requirements**: The unsafe implementation consistently achieves <100ns write latency, well within our <1μs requirement
2. **Bounded Unsafe Usage**: All unsafe code is contained within a single module with clear safety invariants
3. **State Machine Safety**: Atomic state transitions ensure memory safety despite unsafe field access
4. **Proven Pattern**: Lock-free ring buffers are a well-established pattern in high-performance systems

## Consequences

### Positive

- **Performance**: Meets <1μs handoff requirement with significant headroom
- **Scalability**: No lock contention under concurrent load
- **Predictability**: Consistent latency without spikes from lock acquisition
- **Memory Efficiency**: Zero allocations after initialization

### Negative

- **Complexity**: Requires careful reasoning about memory ordering and state transitions
- **Maintenance**: Future maintainers must understand the safety invariants
- **Testing**: Requires more thorough testing, including race condition detection
- **Review Burden**: Changes to the ring buffer require extra scrutiny

### Mitigations

To address the negative consequences:

1. **Comprehensive Documentation**: Every unsafe operation is documented with its safety invariants
2. **Miri Testing**: Use Rust's Miri tool to detect undefined behavior
3. **State Machine Verification**: The atomic state machine prevents invalid state transitions
4. **Performance Benchmarks**: Continuous benchmarking ensures changes don't regress performance
5. **Alternative Implementations**: Keep safe implementations for testing and comparison

## Alternatives Considered

### 1. Mutex-Based Ring Buffer

```rust
struct MutexRingBuffer {
    slots: Mutex<Vec<Slot>>,
    // ...
}
```

**Rejected because**: Lock contention caused 10-50x performance degradation under load.

### 2. RwLock-Based Ring Buffer

```rust
struct RwLockRingBuffer {
    slots: RwLock<Vec<Slot>>,
    // ...
}
```

**Rejected because**: Writer starvation and 5-20x performance overhead.

### 3. Crossbeam Channel

```rust
type RingBuffer = crossbeam::channel::bounded<AuditEvent>;
```

**Rejected because**: 2-5x overhead and doesn't support our exact ring buffer semantics (overwrite on full).

### 4. SPSC Queue Libraries

Evaluated several single-producer single-consumer queue libraries.

**Rejected because**: We need MPSC (multiple producer, single consumer) semantics.

## Performance Comparison

From our benchmarks on a typical server (results may vary):

| Implementation | Single-thread latency | 4-thread concurrent | Memory overhead |
|----------------|----------------------|---------------------|-----------------|
| Unsafe         | ~50ns                | ~100ns              | Minimal         |
| Crossbeam      | ~150ns               | ~500ns              | +8 bytes/slot   |
| RwLock         | ~500ns               | ~5μs                | +40 bytes/slot  |
| Mutex          | ~1μs                 | ~50μs               | +40 bytes/slot  |

## Safety Invariants

The unsafe implementation maintains these invariants:

1. **Exclusive Write Access**: Only the thread that transitions a slot from Empty→Writing can access its data
2. **Atomic State Machine**: All state transitions use compare-and-swap operations
3. **Memory Ordering**: Acquire/Release ordering ensures visibility across threads
4. **No Data Races**: The state machine prevents concurrent access to the same slot
5. **Bounded Lifetime**: All pointers are valid for the lifetime of the ring buffer

## Related

- ADR-0008: Dual-path Architecture (establishes <1μs handoff requirement)
- ADR-0009: Ring Buffer Pattern (defines ring buffer semantics)
- Issue #27: Implement Tower middleware stack (contains performance requirements)
