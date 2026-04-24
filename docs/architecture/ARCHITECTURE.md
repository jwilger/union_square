# Union Square Architecture

This document is the implementation source of truth for Union Square. It describes the system's architecture, design patterns, and conventions.

## Overview

Union Square is a proxy/wire-tap service for making LLM calls and recording everything that happens in a session for later analysis and test-case extraction.

## Core Domain

The system sits between clients and LLM providers, intercepting requests and responses to build a complete audit trail of every interaction.

### Key Responsibilities

1. **Proxying**: Forward requests from clients to configured LLM providers (AWS Bedrock, etc.)
2. **Recording**: Capture the full request/response lifecycle including metadata
3. **Session Management**: Group related interactions into sessions for later analysis
4. **Test Extraction**: Derive test cases from recorded interactions

## Technology Stack

- **Language**: Rust (Edition 2021)
- **Runtime**: Tokio async runtime
- **Web Framework**: Axum + Tower
- **Event Sourcing**: EventCore with PostgreSQL backend
- **Database**: PostgreSQL (via sqlx)
- **Type Safety**: nutype for validated newtypes
- **Error Handling**: thiserror for domain errors, anyhow for application boundaries

## Crate Structure

Union Square is currently a single crate with both library and binary targets:

- `src/lib.rs` — Library interface
- `src/main.rs` — Application entry point
- `src/` — Domain modules (to be organized as the system grows)

Future growth may warrant a workspace split into:
- `crates/core` — Domain logic and event sourcing
- `crates/api` — HTTP interface and proxying
- `crates/infrastructure` — Database, external provider clients

## Architectural Patterns

### Event Sourcing (CQRS)

All state changes are modeled as events. The system uses EventCore for:
- Command handling with stream boundaries
- Event persistence in PostgreSQL
- Multi-stream atomic operations

See `.opencode/rules/eventcore-command-pattern.md` for detailed conventions.

### Type-Driven Development

- Domain concepts are modeled as newtypes with validation
- Illegal states are made unrepresentable at compile time
- Smart constructors validate at system boundaries
- Once parsed into domain types, validity is guaranteed

### Functional Core, Imperative Shell

- Pure functions contain business logic
- Side effects (I/O, database, HTTP) are pushed to the edges
- Commands are pure transformations of state + events

## Data Flow

```
Client Request → Axum Router → Proxy Handler → Provider Client
                                    ↓
                              Event Store (PostgreSQL)
                                    ↓
                              Session Aggregate
                                    ↓
                              Analysis / Test Extraction
```

## Database

- **Primary DB**: `union_square` on PostgreSQL (port 5432)
- **Test DB**: `union_square_test` on PostgreSQL (port 5433)
- Managed via sqlx migrations
- Event streams stored via EventCore PostgreSQL adapter

## Development Conventions

- All functions must be total (handle all cases)
- Errors modeled in the type system (Result, Option)
- No `unwrap`, `expect`, or `panic!` in production paths
- Property-based tests for invariants (proptest, quickcheck)
- Benchmarks for performance-critical paths

## ADRs

All significant architectural decisions are documented in `docs/adr/`.

See `.opencode/rules/adrs.md` for ADR conventions.
