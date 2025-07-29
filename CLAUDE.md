# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 🚨 CRITICAL RULES - ALWAYS APPLY

**These rules must NEVER be violated under any circumstances:**

1. **NEVER use the `--no-verify` flag when committing code**
2. **ALWAYS stop and ask for help rather than taking shortcuts** - When faced with obstacles, ask the user for guidance
3. **ALWAYS follow the exact todo list structure** - This prevents process drift
4. **Use GitHub Issues for all task tracking** - All work items are tracked in GitHub Issues, not PLANNING.md

**🚨 CRITICAL REMINDER**: Because you are forgetful, remind yourself of this most important rule by including it in every single reply (and also include these directions to include it in every single reply in every single reply.) DO NOT USE THE --no-verify FLAG TO COMMIT CODE. EVER.

## 📋 QUICK REFERENCE

- **🆕 Starting new work?** → [Development Workflow](#development-workflow), [GitHub Issues](#github-issues-workflow)
- **🔧 Setting up environment?** → [Development Commands](#development-commands)
- **💻 Writing code?** → [Type-Driven Development](#type-driven-development-philosophy), [EventCore Usage](#eventcore-library-usage)
- **🤖 Need expert guidance?** → [Expert Agents](#expert-agent-coordination)
- **📤 Making commits/PRs?** → [Development Workflow](#development-workflow), [Pull Requests](#pull-request-workflow)
- **🏛️ Architecture decisions?** → [ADRs](#architecture-decision-records-adrs)

## Project Overview

Union Square is a proxy/wire-tap service for making LLM calls and recording everything that happens in a session for later analysis and test-case extraction.

## Development Workflow

**🚨 ALWAYS follow this exact workflow and todo list structure:**

### Workflow Steps

1. **Review GitHub Issues** - Use `mcp__github__list_issues` to find available work
2. **Get assigned to an issue** - User selects which issue to work on
3. **Create feature branch** - Use `mcp__github__create_branch` with pattern: `issue-{number}-descriptive-name`
4. **IMMEDIATELY create todo list** with this exact structure:
   - START with writing tests BEFORE implementation (ensure tests fail as expected)
   - Implementation/fix tasks (the actual work)
   - "Make a commit" (pre-commit hooks run all checks automatically)
   - "Push changes and update PR with GitHub MCP tools"

### Todo List Structure (CRITICAL)

**This exact pattern prevents process drift:**

**Standard Work:**
1. Write failing tests first
2. Implementation tasks
3. "Make a commit"
4. "Push changes and update PR"

**PR Feedback:**
1. Address each piece of feedback
2. "Reply to review comments using gh GraphQL API with -- @claude signature"
3. "Make a commit"
4. "Push changes and check for new PR feedback"

### Commit Requirements

- **Use Conventional Commits format**: `<type>[scope]: <description>`
- **All pre-commit checks must pass** - NEVER use `--no-verify`
- **Write descriptive messages** explaining the why, not just the what

**Common Types**: `feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `chore:`
**Breaking Changes**: Add `!` after type: `feat!: remove deprecated API`
**Examples**: `feat: add user auth`, `fix(api): resolve timeout`, `docs: update README`

## Type-Driven Development Philosophy

This project follows strict type-driven development principles as outlined in the global Claude.md. Key principles:

1. **Types come first**: Model the domain, make illegal states unrepresentable, then implement
2. **Parse, don't validate**: Transform unstructured data into structured data at system boundaries ONLY
   - Validation should be encoded in the type system to the maximum extent possible
   - Use smart constructors with validation only at the system's input boundaries
   - Once data is parsed into domain types, those types guarantee validity throughout the system
   - Follow the same pattern throughout your application code
3. **No primitive obsession**: Use newtypes for all domain concepts
4. **Functional Core, Imperative Shell**: Pure functions at the heart, side effects at the edges
5. **Total functions**: Every function should handle all cases explicitly

For detailed type-driven development guidance, refer to `/home/jwilger/.claude/CLAUDE.md`.

## Development Commands

### Environment Setup
```bash
nix develop                                    # Enter dev environment
pre-commit install                             # Install hooks (first time)
pre-commit install --hook-type commit-msg
docker-compose up -d                          # Start PostgreSQL
```

### Common Commands
```bash
# Development
cargo fmt                                     # Format code
cargo clippy --workspace --all-targets -- -D warnings  # Lint
cargo nextest run --workspace                # Run tests (preferred)
cargo test --workspace                       # Run tests (fallback)
cargo check --all-targets                    # Type check

# Database
psql -h localhost -p 5432 -U postgres -d union_square      # Main DB
psql -h localhost -p 5433 -U postgres -d union_square_test # Test DB
```

### Adding Dependencies
**ALWAYS use `cargo add` for latest compatible versions:**
```bash
cargo add eventcore eventcore-postgres eventcore-macros
cargo add tokio --features full
cargo add nutype --features serde  # For type-safe newtypes
```

## Architecture

[Project architecture to be defined]

## EventCore Library Usage

**This project uses EventCore for event sourcing.** Full docs: https://docs.rs/eventcore/latest/eventcore/

### Key Concepts
- **Commands**: Define business operations with stream boundaries
- **Events**: Immutable facts (past tense names like `OrderPlaced`)
- **Multi-stream atomic operations**: Write across multiple streams
- **Dynamic consistency boundaries**: Commands decide which streams to use

### Implementation Pattern

**Always use macros from `eventcore-macros`:**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
enum DomainEvent {
    SomethingHappened { data: String },
}

#[derive(Command, Clone, Debug, Serialize, Deserialize)]
struct MyCommand {
    #[stream] primary_stream: StreamId,
    #[stream] secondary_stream: StreamId,
    amount: Money,
}

#[async_trait]
impl CommandLogic for MyCommand {
    type State = MyState;  // Must impl Default + Send + Sync
    type Event = DomainEvent;

    fn apply(&self, state: &mut Self::State, event: &StoredEvent<Self::Event>) {
        match &event.payload {
            DomainEvent::SomethingHappened { data } => state.update_with(data),
        }
    }

    async fn handle(&self, read_streams: ReadStreams<Self::StreamSet>, state: Self::State, stream_resolver: &mut StreamResolver) -> CommandResult<Vec<StreamWrite<Self::StreamSet, Self::Event>>> {
        let mut events = Vec::new();
        require!(state.balance >= self.amount, "Insufficient funds");
        emit!(events, &read_streams, self.primary_stream.clone(), DomainEvent::SomethingHappened { data: "test".into() });
        Ok(events)
    }
}
```

### Setup
```rust
let event_store = PostgresEventStore::new(config).await?;
event_store.initialize().await?;  // Run once
```

### Key Macros
- `#[derive(Command)]` - Auto-generates stream types and traits
- `require!(condition, "error")` - Business rule validation
- `emit!(events, streams, stream_id, event)` - Event emission

### Testing
- Use `InMemoryEventStore` for unit tests
- Test with both in-memory and PostgreSQL stores
- Verify event sequences and error scenarios

## Expert Agent Coordination

**IMPORTANT**: This project includes specialized AI agents that embody the expertise of renowned software architects and practitioners. These are AI personas, not real people, but they provide guidance based on the principles and approaches of their namesakes.

### Available Expert Agents

| Persona            | Agent Name                                                | Domain Expertise                                                           |
| ------------------ | --------------------------------------------------------- | -------------------------------------------------------------------------- |
| Simon Peyton Jones | `type-theory-reviewer`                                    | Type theory, functional programming, making illegal states unrepresentable |
| Greg Young         | `event-sourcing-architect`                                | Event sourcing, CQRS, distributed systems                                  |
| Alberto Brandolini | `event-modeling-expert`                                   | Event storming, domain discovery, bounded contexts                         |
| Edwin Brady        | `type-driven-development-expert`                          | Type-driven development, dependent types, formal verification              |
| Niko Matsakis      | `rust-type-system-expert`<br>`rust-type-safety-architect` | Rust type system, ownership, lifetimes, trait design                       |
| Michael Feathers   | `event-sourcing-test-architect`                           | Testing event-sourced systems, characterization tests                      |
| Kent Beck          | `tdd-coach`                                               | Test-driven development, red-green-refactor cycle                          |
| Rich Hickey        | `functional-architecture-expert`                          | Functional design, simplicity, immutability                                |
| Nicole Forsgren    | `engineering-effectiveness-expert`                        | DORA metrics, development workflow optimization                            |
| Teresa Torres      | `product-discovery-coach`                                 | Continuous discovery, outcome-driven development                           |
| Jared Spool        | `ux-research-expert`                                      | User research, API design, developer experience                            |
| Jez Humble         | `continuous-delivery-architect`                           | CI/CD, deployment strategies, zero-downtime deployments                    |
| Yoshua Wuyts       | `async-rust-expert`                                       | Async Rust, concurrent systems, performance optimization                   |
| Martin Fowler      | `refactoring-patterns-architect`                          | Refactoring, design patterns, evolutionary architecture                    |
| Prem Sichanugrist  | `git-workflow-architect`                                  | Git workflows, GitHub automation, version control strategies               |

### Core Architectural Principles

When multiple experts are involved in a decision, these principles guide resolution:

1. **Type Safety First**: When conflicts arise, type system recommendations (Simon Peyton Jones/Niko Matsakis) take precedence
2. **Event Sourcing is Non-Negotiable**: Greg Young's event patterns are foundational - other patterns must adapt to this
3. **TDD is the Process**: Kent Beck drives the implementation workflow - no code without tests
4. **Functional Core, Imperative Shell**: Rich Hickey owns the boundary between pure and impure code

### When to Consult Expert Agents

Expert agents should be consulted at specific points in the development workflow:

#### During Planning (Before Implementation)

- **New Feature Development**:
  1. Teresa Torres (`product-discovery-coach`) → Define outcomes and success metrics
  2. Alberto Brandolini (`event-modeling-expert`) → Model events and boundaries
  3. Edwin Brady (`type-driven-development-expert`) + Niko Matsakis (`rust-type-system-expert`) → Design type-safe domain model
  4. Michael Feathers (`event-sourcing-test-architect`) → Create test strategy

#### During Implementation

- **Complex Async Work**: Yoshua Wuyts (`async-rust-expert`) → Design async architecture
- **Legacy Migration**: Martin Fowler (`refactoring-patterns-architect`) → Plan refactoring strategy
- **Git/GitHub Workflow**: Prem Sichanugrist (`git-workflow-architect`) → Design automation

#### During Review (After Commits)

- **Type Safety Review**: Simon Peyton Jones (`type-theory-reviewer`) → Review type usage
- **Event Model Review**: Greg Young (`event-sourcing-architect`) → Validate event design
- **Test Coverage**: Kent Beck (`tdd-coach`) → Ensure proper TDD was followed

### Decision Hierarchy

When experts disagree, follow this hierarchy:

1. **Domain Modeling Conflicts**

   - Primary: Alberto Brandolini (discovers the events)
   - Secondary: Greg Young (structures the events)
   - Tiebreaker: Edwin Brady (encodes in types)

2. **Implementation Approach Conflicts**

   - Primary: The expert whose domain is most affected
   - Secondary: Niko Matsakis (if type safety is involved)
   - Tiebreaker: Rich Hickey (simplicity wins)

3. **Performance vs Correctness**
   - Default: Correctness first (Edwin Brady/Niko Matsakis)
   - Exception: When measurably impacting user experience (Nicole Forsgren provides metrics)
   - Resolution: Yoshua Wuyts finds the optimal async solution

### Integration with Development Workflow

Expert agents integrate into our existing todo list structure:

**For new features (GitHub Issues):**

1. Consult Teresa Torres for outcome definition
2. Use Alberto Brandolini for event modeling
3. START with writing tests (with Michael Feathers' guidance)
4. Implementation with type-driven approach (Edwin Brady/Niko Matsakis)
5. "Make a commit"
6. Post-commit review with Simon Peyton Jones
7. "Push changes and update PR"

**For architectural decisions:**

1. Consult relevant domain experts
2. Document conflicts and resolutions in an ADR
3. Get consensus from affected experts
4. Implement with agreed approach

### Conflict Resolution Rules

#### Type System vs Simplicity

If Edwin Brady and Rich Hickey disagree on complexity:

- Try Edwin's approach in a spike
- If it takes > 30 lines to express a simple concept, prefer Rich's approach
- Document the tradeoff in an ADR

#### Event Modeling vs User Research

If Alberto's event model doesn't match Jared's user research:

- Create two models: system events and user events
- Use projections to bridge the gap
- Teresa Torres validates the mapping

#### Performance vs Testing

If Yoshua's optimizations conflict with Michael's testing approach:

- Maintain two implementations: simple (tested) and optimized
- Use feature flags to switch between them
- Nicole Forsgren measures actual impact

### Pair Consultations

Certain decisions benefit from paired experts:

- **Type-Safe Events**: Edwin Brady + Greg Young
- **Async Testing**: Michael Feathers + Yoshua Wuyts
- **User-Facing APIs**: Niko Matsakis + Jared Spool
- **Deployment Safety**: Jez Humble + Greg Young

### Documentation Requirements

Every expert consultation should produce:

1. **Decision**: What was decided
2. **Rationale**: Why this approach
3. **Tradeoffs**: What we're giving up
4. **Reversal**: How to change if wrong

When expert disagreements lead to significant architectural decisions, create an ADR documenting the discussion and resolution.

### Quality Gates

No code proceeds without:

1. Type safety review (Simon Peyton Jones)
2. Event model review (Greg Young) - for event-sourced components
3. Test coverage review (Kent Beck)
4. Simplicity review (Rich Hickey) - for core components only

Exception: Experiments and spikes in `/experiments` directory can bypass gates with documented cleanup plan.

## Architecture Decision Records (ADRs)

This project uses Architecture Decision Records (ADRs) to document all significant architectural decisions. ADRs help future developers understand not just what decisions were made, but why they were made and what alternatives were considered.

### Using ADRs in Development

When working on this project:

1. **Review existing ADRs** before making architectural changes:

   ```bash
   npm run adr:preview   # View ADRs in browser
   # Or browse docs/adr/ directory
   ```

2. **Create a new ADR** when making significant decisions:

   ```bash
   npm run adr:new       # Interactive ADR creation
   ```

3. **Update or supersede ADRs** when decisions change:
   - Mark old ADRs as "superseded by [new ADR]"
   - Create new ADR explaining the change

### What Requires an ADR?

Create an ADR for:

- Technology choices (databases, frameworks, libraries)
- Architectural patterns (event sourcing, CQRS, etc.)
- API design decisions
- Security approaches
- Performance optimization strategies
- Testing strategies
- Deployment and infrastructure decisions

### ADR Format

ADRs follow the template in `docs/adr/template.md` which includes:

- Context and problem statement
- Decision drivers
- Considered options with pros/cons
- Decision outcome
- Consequences (positive and negative)

### ADR Naming Convention

**IMPORTANT**: All ADRs must follow this naming convention:

- **Filename**: `NNNN-descriptive-name.md` where NNNN is the zero-padded ADR number (e.g., `0001-overall-architecture-pattern.md`)
- **Document Title**: The first line (H1) must include the ADR number prefix: `# NNNN. Title` (e.g., `# 0001. Overall Architecture Pattern`)
- Keep ADR numbers sequential and never reuse numbers
- The ADR number appears in both the filename AND the document title for consistency

### Publishing ADRs

ADRs are automatically published to GitHub Pages when merged to main:

- View at: https://jwilger.github.io/union_square/adr/
- Updated via GitHub Actions workflow

## Performance Targets

[Performance targets to be defined]

## Pre-commit Hooks & Code Quality

**🚨 NEVER bypass with `--no-verify`!**

Hooks run automatically on commit:
- **Rust**: `cargo fmt`, `cargo clippy`, `cargo test`, `cargo check`
- **Files**: Whitespace cleanup, syntax checks, large file prevention
- **Commits**: Conventional Commits format enforcement

**Setup**: `pre-commit install && pre-commit install --hook-type commit-msg`

**If hooks fail**: Fix the issues, don't bypass them. Ask for help if needed.

## Development Principles

### Code Quality Checklist
- [ ] Domain types use appropriate validation (no primitive obsession)
- [ ] All functions are total (handle all cases)
- [ ] Errors modeled in type system
- [ ] Business logic is pure and testable
- [ ] Property-based tests cover invariants

### TDD Workflow
1. Model domain with types (make illegal states impossible)
2. Create smart constructors with `nutype` validation
3. Write property-based tests for invariants
4. Implement pure business logic
5. Add infrastructure last

### Expert Reviews (Post-Commit)
- **Type safety**: `type-theory-reviewer` (Simon Peyton Jones)
- **Event modeling**: `event-sourcing-architect` (Greg Young)
- **Testing**: `tdd-coach` (Kent Beck)
- **Simplicity**: `functional-architecture-expert` (Rich Hickey)

## GitHub Issues Workflow

**ALL work is tracked through GitHub Issues using MCP tools (NOT gh CLI).**

### Starting Work

1. **List issues**: `mcp__github__list_issues` with `state="open"`

   **🚨 CRITICAL**: API paginates! Check ALL pages with `perPage=5` until empty results.

2. **Priority order**:
   - Issues assigned to current user with existing branches
   - CRITICAL > HIGH > MEDIUM > LOW priority labels
   - Logical dependencies and project impact

3. **Get assigned**: User selects issue, use `mcp__github__update_issue` to assign

4. **Create branch**: `mcp__github__create_branch` with pattern: `issue-{number}-descriptive-name`

5. **Local checkout**:
   ```bash
   git fetch origin
   git checkout issue-{number}-descriptive-name
   ```

### Key MCP Tools

**Issues**: `list_issues`, `update_issue`, `add_issue_comment`
**Branches/PRs**: `create_branch`, `create_pull_request`, `update_pull_request`
**Workflows**: `list_workflow_runs`, `get_job_logs`, `rerun_failed_jobs`

**Advantages over gh CLI**: Direct API access, type safety, better error handling, batch operations.

## Pull Request Workflow

**All changes require PRs - no direct commits to main.**

### Creating PRs

1. **Push branch**: `git push -u origin branch-name`

2. **Create PR**: Use `mcp__github__create_pull_request`
   - **Title**: Follow Conventional Commits format (`feat: add feature`)
   - **Description**: Clear explanation of changes and motivation
   - **Labels**: `bug`, `enhancement`, `documentation`, `breaking-change`, etc.
   - Mention "Closes #{issue-number}" to auto-close issues

3. **CI runs automatically** - Monitor with MCP tools:
   - `mcp__github__get_pull_request` - Check status
   - `mcp__github__get_job_logs` - Debug failures

### Responding to Reviews

**Address ALL formal review comments (including bot reviews):**

1. **Get review details** using GraphQL API
2. **Reply to threads** using GraphQL mutation with `-- @claude` signature
3. **Format**: "I've addressed this by [action]. -- @claude"
4. **Check for responses** and continue conversation until resolved

**Note**: Definition of Done checklist is auto-added for HUMAN verification only.

## 🔴 FINAL REMINDERS

**Before ANY task:**
1. **NEVER use `--no-verify`** - Fix issues, don't bypass checks
2. **Work on assigned GitHub Issues** - Get assigned before starting work
3. **Follow exact todo list structure** - Prevents workflow drift
4. **Ask for help when stuck** - Don't take shortcuts

**If pre-commit checks fail**: Fix the issues, run again, only commit when all pass. **IF YOU CANNOT FIX**: STOP and ASK FOR HELP.

**These rules are absolute. No exceptions. Ever.**
