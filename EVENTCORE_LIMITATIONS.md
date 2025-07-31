# EventCore Limitations Found During Migration

This document outlines limitations we discovered in EventCore 0.1.8 that prevented full usage of the built-in projection system. These should be filed as issues on the EventCore GitHub repository (jwilger/eventcore).

## 1. Missing Timestamp Filtering in ReadOptions

**Issue**: EventCore's `ReadOptions` doesn't support `after_timestamp` filtering, which is needed for catching up projections from a specific point in time.

**Found in**: `src/infrastructure/eventcore/projections/core.rs` (line 160, now removed)

**Use Case**: When a projection needs to rebuild from a checkpoint, it needs to read only events after that timestamp.

**Workaround Needed**: Manual filtering of events after reading, which is inefficient.

## 2. Limited Timestamp Arithmetic

**Issue**: EventCore's `Timestamp` type doesn't support arithmetic operations (add, subtract, comparison with durations).

**Found in**: Multiple places in our projection code

**Use Cases**:
- Calculating session durations
- Finding inactive sessions (e.g., sessions with no activity for 30 minutes)
- Time-based analytics and metrics

**Workaround Needed**: Convert to chrono or another time library for calculations.

## 3. No Wildcard Stream Subscriptions

**Issue**: EventCore doesn't support wildcard subscriptions like `session:*` to subscribe to all streams matching a pattern.

**Found in**: `src/infrastructure/eventcore/projections/service.rs` (line 200, now removed)

**Use Case**: Projections that need to aggregate data across multiple streams of the same type (e.g., all session streams).

**Workaround Needed**: Maintain a list of all relevant streams and subscribe individually.

## 4. Multi-Stream Atomic Projections

**Issue**: No built-in support for projections that atomically process events from multiple streams.

**Use Case**: Cross-stream consistency requirements, such as maintaining user activity across all their sessions.

## 5. Advanced Event Filtering

**Issue**: No built-in support for sophisticated event filtering beyond type matching.

**Use Case**: Projections that only care about specific event attributes or complex conditions.

## 6. Batch Processing Configuration

**Issue**: No way to configure batch sizes for event processing to optimize performance.

**Use Case**: High-throughput systems that need to balance latency vs efficiency.

## 7. Type-Erased Projection Collections

**Issue**: Difficult to manage heterogeneous collections of projections with different state types.

**Use Case**: A projection supervisor that manages multiple different projection types.

## Next Steps

These limitations should be evaluated to determine:
1. Which are genuine missing features vs misunderstandings of EventCore's design
2. Which can be worked around vs which need EventCore enhancements
3. Priority of each limitation based on real-world usage

Before filing issues, we should:
1. Verify these limitations still exist in the latest EventCore version
2. Check if there are existing issues or PRs addressing these
3. Provide concrete examples and use cases for each limitation
