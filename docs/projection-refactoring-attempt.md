# Projection Refactoring Attempt

## Summary

This document describes an attempt to refactor the projection system to use EventCore's built-in Projection trait, as requested in the PR review feedback.

## What Was Attempted

1. Created a `FunctionalProjection` trait to maintain immutable state updates
2. Implemented `SessionSummaryProjection` and `UserActivityProjection`
3. Created a `ProjectionAdapter` to bridge to EventCore's Projection trait
4. Built a `ProjectionManager` for lifecycle management
5. Created `MaterializedQueryService` to read from projections

## Why It Failed

EventCore's Projection trait has specific requirements that made integration difficult:

1. **All methods are async** - Our functional approach used synchronous state updates
2. **Different method signatures** - EventCore expects mutable state references and Event types (not StoredEvent)
3. **Complex lifetime requirements** - The trait has specific lifetime bounds that don't match our implementation
4. **Different error types** - EventCore uses ProjectionResult instead of Result

## Key Learnings

1. EventCore's Projection trait is designed for stateful, mutable projections
2. Our functional approach with immutable updates doesn't align with EventCore's design
3. The current query-time projection approach, while inefficient, works with the existing codebase

## Recommendation

Rather than forcing a functional approach onto EventCore's imperative Projection trait, consider:

1. Using EventCore's Projection trait as designed (with mutable state)
2. Creating a separate functional projection layer if immutability is required
3. Accepting the query-time projection approach for now and optimizing later

## Code Status

The attempted implementation has compilation errors and is not ready for production use. The code demonstrates the concept but would need significant rework to properly integrate with EventCore.
