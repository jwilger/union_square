# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 🚨 CRITICAL RULES - ALWAYS APPLY

**These rules must NEVER be violated under any circumstances:**

1. **NEVER use the `--no-verify` flag when committing code**
2. **ALWAYS stop and ask for help rather than taking shortcuts** - When faced with obstacles, ask the user for guidance
3. **ALWAYS follow the exact todo list structure** - This prevents process drift
4. **Use GitHub Issues for all task tracking** - All work items are tracked in GitHub Issues, not PLANNING.md

## 📋 TABLE OF CONTENTS

### Quick Reference by Task
- **🆕 Starting new work?** → Read [🚨 Critical Rules](#critical-rules---always-apply), [Development Process Rules](#development-process-rules), [GitHub Issues Workflow](#github-issues-workflow)
- **🔧 Setting up environment?** → Read [Development Commands](#development-commands)
- **💻 Writing code?** → Read [Architecture](#architecture), [Type-Driven Development](#type-driven-development-philosophy), [EventCore Library Usage](#eventcore-library-usage)
- **📊 Working with events?** → Read [EventCore Library Usage](#eventcore-library-usage)
- **🏛️ Making architectural decisions?** → Read [Architecture Decision Records](#architecture-decision-records-adrs)
- **📤 Making commits?** → Read [Commit Rules](#commit-rules), [Pre-commit Hooks](#pre-commit-hooks)
- **🔄 Creating/updating PRs?** → Read [Pull Request Workflow](#pull-request-workflow), [🚨 Critical Rules](#critical-rules---always-apply)
- **💬 Responding to PR feedback?** → Read [Responding to PR Feedback](#responding-to-pr-feedback)
- **💙 Using GitHub features?** → Read [GitHub MCP Integration](#github-mcp-integration), [GitHub Issues Workflow](#github-issues-workflow)

### All Sections
1. [🚨 Critical Rules](#critical-rules---always-apply) (THIS SECTION - READ FIRST!)
2. [Project Overview](#project-overview)
3. [Development Process Rules](#development-process-rules) (How to work on this project)
4. [Type-Driven Development Philosophy](#type-driven-development-philosophy)
5. [Development Commands](#development-commands)
6. [Architecture](#architecture)
7. [EventCore Library Usage](#eventcore-library-usage) (Event sourcing with EventCore)
8. [Architecture Decision Records (ADRs)](#architecture-decision-records-adrs)
9. [Performance Targets](#performance-targets)
10. [Pre-commit Hooks](#pre-commit-hooks)
11. [Development Principles](#development-principles)
12. [GitHub MCP Integration](#github-mcp-integration)
13. [GitHub Issues Workflow](#github-issues-workflow) (How to work with issues)
14. [Pull Request Workflow](#pull-request-workflow)
15. [Memories](#memories) (Important reminders)

## Project Overview

Union Square is a proxy/wire-tap service for making LLM calls and recording everything that happens in a session for later analysis and test-case extraction.

## Development Process Rules

**🚨 REMINDER: Review [Critical Rules](#critical-rules---always-apply) before proceeding!**

When working on this project, **ALWAYS** follow these rules:

1. **Review GitHub Issues** to discover work items. Use `mcp__github__list_issues` to see open issues.
2. **Get assigned to an issue** before starting work. The user will select which issue to work on.
3. **Create a feature branch** for the issue using `mcp__github__create_branch`.
4. **Follow the Pull Request Workflow** (see [Pull Request Workflow](#pull-request-workflow)) for all code changes.
5. **IMMEDIATELY use the todo list tool** to create a todolist with the specific actions you will take to complete the task.
6. **Insert a task to "Make a commit"** after each discrete action that involves a change to the code, tests, database schema, or infrastructure. Note: Pre-commit hooks will run all checks automatically.
7. **The FINAL item in the todolist MUST always be** to "Push your changes to the remote repository and create/update PR with GitHub MCP tools."

### CRITICAL: Todo List Structure

**This structure ensures Claude never forgets the development workflow:**

Your todo list should ALWAYS follow this pattern:

**For work on GitHub Issues:**
1. START with writing tests for any changes BEFORE making the changes, and ensure the tests fail as you expect them to.
2. Implementation/fix tasks (the actual work)
3. "Make a commit" (pre-commit hooks run all checks automatically)
4. "Push changes and update PR"

**For ad-hoc requests not tracked in GitHub Issues:**
1. START with writing tests for any changes BEFORE making the changes, and ensure the tests fail as you expect them to.
2. Implementation/fix tasks (the actual work)
3. "Make a commit" (pre-commit hooks run all checks automatically)
4. "Push changes and update PR"

For PR feedback specifically:
1. Address each piece of feedback
2. "Reply to review comments using gh GraphQL API with -- @claude signature"
3. "Make a commit"
4. "Push changes and check for new PR feedback"

**Why this matters**: The todo list tool reinforces our workflow at every step, preventing process drift as context grows.

### Commit Rules

**BEFORE MAKING ANY COMMIT**:

1. **Ensure all changes are properly tested** and pre-commit checks will pass
2. **Use Conventional Commits format** for all commit messages (see details below)
3. **Write clear, descriptive commit messages** that explain the why, not just the what

**🚨 CRITICAL REMINDER**: NEVER use `--no-verify` flag. All pre-commit checks must pass!

### Conventional Commits Format

This project uses [Conventional Commits](https://www.conventionalcommits.org/) for all commit messages. This ensures a standardized, readable commit history that supports automated tooling.

**Commit Message Structure**:
```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

**Required Types**:
- `feat:` - A new feature (correlates with MINOR in semantic versioning)
- `fix:` - A bug fix (correlates with PATCH in semantic versioning)
- `docs:` - Documentation only changes
- `style:` - Changes that don't affect code meaning (formatting, missing semi-colons, etc)
- `refactor:` - Code change that neither fixes a bug nor adds a feature
- `perf:` - Code change that improves performance
- `test:` - Adding missing tests or correcting existing tests
- `build:` - Changes that affect the build system or dependencies
- `ci:` - Changes to CI configuration files and scripts
- `chore:` - Other changes that don't modify src or test files
- `revert:` - Reverts a previous commit

**Breaking Changes**:
- Add `!` after the type/scope: `feat!: remove deprecated API`
- OR include `BREAKING CHANGE:` in the footer

**Examples**:
```
feat: add EventCore command for version tracking

fix(version-commands): handle HashMap lookup correctly

docs: update CLAUDE.md with conventional commits format

refactor!: remove adapter layer for EventCore integration

BREAKING CHANGE: EventCore commands are now first-class citizens
```

**Scope Guidelines**:
- Use module names for scope when appropriate (e.g., `fix(eventcore):`)
- Keep scope concise and lowercase
- Omit scope if the change is broad or crosses multiple modules

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

**🚨 REMINDER**: Never use `--no-verify` flag! See [Critical Rules](#critical-rules---always-apply)

### Setup

```bash
# Enter development environment (required for all work)
nix develop

# Install pre-commit hooks (first time setup)
pre-commit install
pre-commit install --hook-type commit-msg

# Start PostgreSQL databases
docker-compose up -d

# Initialize Rust project (if not done)
cargo init --lib

# Install development tools
cargo install cargo-nextest --locked  # Fast test runner
cargo install cargo-llvm-cov --locked # Code coverage

# IMPORTANT: Always check for latest versions before adding dependencies
# Use: cargo search <crate_name> to find latest version

# Core dependencies (example - adjust based on project needs)
cargo add tokio --features full
cargo add async-trait
cargo add uuid --features v7
cargo add serde --features derive
cargo add serde_json
cargo add sqlx --features runtime-tokio-rustls,postgres,uuid,chrono
cargo add thiserror
cargo add tracing
cargo add tracing-subscriber

# Type safety dependencies
cargo add nutype --features serde  # For newtype pattern with validation
cargo add derive_more  # For additional derives on newtypes

# EventCore dependency (since this project uses it)
cargo add eventcore
cargo add eventcore-postgres
cargo add eventcore-macros  # For #[derive(Command)] macro
```

### Development Workflow

```bash
# Format code
cargo fmt

# Run linter
cargo clippy --workspace --all-targets -- -D warnings

# Run tests with nextest (recommended - faster and better output)
cargo nextest run --workspace

# Run tests with cargo test (fallback)
cargo test --workspace

# Run tests with output
cargo nextest run --workspace --nocapture
# Or with cargo test: cargo test --workspace -- --nocapture

# Run a specific test
cargo nextest run test_name
# Or with cargo test: cargo test test_name -- --nocapture

# Type check
cargo check --all-targets

# Build release version
cargo build --release

# Run benchmarks
cargo bench
```

### Database Operations

```bash
# Connect to main database
psql -h localhost -p 5432 -U postgres -d union_square

# Connect to test database
psql -h localhost -p 5433 -U postgres -d union_square_test

# Run database migrations (once implemented)
sqlx migrate run
```

## Architecture

[Project architecture to be defined]

## EventCore Library Usage

**IMPORTANT**: This project uses EventCore for event sourcing. When working with EventCore, fetch the full documentation at https://docs.rs/eventcore/0.1.3/eventcore/ for detailed information.

### EventCore Overview

EventCore is a Rust library for implementing multi-stream event sourcing with dynamic consistency boundaries. Key characteristics:

- **No predefined aggregate boundaries** - Commands define their own consistency boundaries
- **Multi-stream atomic operations** - Write events atomically across multiple streams
- **Type-driven development** - Leverages Rust's type system for domain modeling
- **Flexible consistency** - Each command decides which streams to read and write

### Core Concepts

1. **Commands**: Define business operations with:
   - Stream selection (which streams to read)
   - State folding (how to build state from events)
   - Business logic (producing new events)

2. **Events**: Domain events representing state changes
   - Defined as enums with variants for different changes
   - Must implement `Serialize`, `Deserialize`, `Send`, `Sync`
   - Stored with metadata (stream ID, timestamp, version)

3. **Event Stores**: Provide durable storage with:
   - Multi-stream atomic writes
   - Optimistic concurrency control
   - Global event ordering
   - PostgreSQL and in-memory implementations

### Implementation Pattern

**IMPORTANT**: Always use the `#[derive(Command)]` macro from eventcore-macros to reduce boilerplate. This macro automatically generates:
- A phantom type for compile-time stream access control (e.g., `MyCommandStreamSet`)
- The `CommandStreams` trait implementation with `read_streams()` method
- Proper type associations for EventCore

```rust
// 1. Define your events
#[derive(Debug, Clone, Serialize, Deserialize)]
enum DomainEvent {
    SomethingHappened { data: String },
    SomethingElseOccurred { value: u64 },
}

// 2. Define your command with the Command derive macro
use eventcore_macros::Command;

#[derive(Command, Clone, Debug, Serialize, Deserialize)]
struct MyCommand {
    #[stream]  // Mark fields that are streams
    primary_stream: StreamId,
    #[stream]
    secondary_stream: StreamId,
    // command data (non-stream fields)
    amount: Money,
}

// The macro eliminates the need to manually implement CommandStreams!

// 3. Implement CommandLogic
#[async_trait]
impl CommandLogic for MyCommand {
    type State = MyState;  // Must impl Default + Send + Sync
    type Event = DomainEvent;

    fn apply(&self, state: &mut Self::State, event: &StoredEvent<Self::Event>) {
        // Apply events to state
        match &event.payload {
            DomainEvent::SomethingHappened { data } => {
                state.update_with(data);
            }
            // ... handle other events
        }
    }

    async fn handle(
        &self,
        read_streams: ReadStreams<Self::StreamSet>,
        state: Self::State,
        stream_resolver: &mut StreamResolver,
    ) -> CommandResult<Vec<StreamWrite<Self::StreamSet, Self::Event>>> {
        // Business logic here
        // Return events to be written
        Ok(vec![
            StreamWrite::new(&read_streams, self.primary_stream.clone(),
                DomainEvent::SomethingHappened { data: "test".into() })?,
        ])
    }
}
```

### PostgreSQL Event Store Setup

```rust
// Configure PostgreSQL event store
let config = PostgresConfig::builder()
    .connection_string("postgres://...")
    .build();

let event_store = PostgresEventStore::new(config).await?;

// Initialize database schema (run once)
event_store.initialize().await?;

// Run migrations if needed
event_store.migrate().await?;
```

### Best Practices

1. **Event Design**:
   - Events should be immutable facts about what happened
   - Use past tense naming (e.g., `OrderPlaced`, not `PlaceOrder`)
   - Include all necessary data in the event
   - Events should be self-contained

2. **Command Design**:
   - Commands represent intentions
   - Define clear consistency boundaries via streams
   - Keep commands focused on a single business operation
   - Use the type system to enforce invariants

3. **State Management**:
   - State is ephemeral - rebuilt from events
   - Keep state minimal and focused
   - Use type-safe state representations
   - Implement `Default` trait meaningfully

4. **Testing**:
   - Use `InMemoryEventStore` for unit tests
   - Test command logic independently
   - Verify event sequences match expectations
   - Test error scenarios and edge cases

5. **Production Considerations**:
   - Always use PostgreSQL event store in production
   - Configure retry strategies for resilience
   - Monitor event store health
   - Plan for event schema evolution

### Common Patterns

```rust
// Multi-stream transaction
#[derive(Command)]
struct TransferFunds {
    #[stream]
    from_account: StreamId,
    #[stream]
    to_account: StreamId,
    amount: Money,
}

// Event replay for projections
let events = event_store.read_stream(stream_id, None).await?;
let state = events.fold(State::default(), |mut state, event| {
    command.apply(&mut state, &event);
    state
});
```

### Troubleshooting

- **Concurrency conflicts**: Use optimistic concurrency control via stream versions
- **Performance**: Batch event writes when possible
- **Schema evolution**: Plan for event versioning from the start
- **Testing**: Always test with both in-memory and PostgreSQL stores

**Remember**: When in doubt, consult the full EventCore documentation at https://docs.rs/eventcore/0.1.3/eventcore/

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

### Publishing ADRs

ADRs are automatically published to GitHub Pages when merged to main:
- View at: https://jwilger.github.io/union_square/adr/
- Updated via GitHub Actions workflow

## Performance Targets

[Performance targets to be defined]

## Pre-commit Hooks

**🚨 CRITICAL**: These hooks ensure code quality. NEVER bypass them with `--no-verify`!

This project uses the [pre-commit framework](https://pre-commit.com/) to manage git hooks. The configuration is in `.pre-commit-config.yaml`.

### Hooks that run on every commit:

1. **Rust checks** (run on .rs files):
   - `cargo fmt` - Auto-formats Rust code
   - `cargo clippy` - Linting with all warnings as errors
   - `cargo test` - Runs all workspace tests
   - `cargo check` - Type checking

2. **General file checks**:
   - Remove trailing whitespace
   - Fix end-of-file issues
   - Check YAML, TOML, and JSON syntax
   - Prevent large files from being committed
   - Check for merge conflicts
   - Pretty-format JSON files

3. **Commit message validation** (commit-msg stage):
   - **Conventional Commits enforcement** via commitizen
   - Ensures all commits follow the format: `type(scope): description`

### Setup

After cloning the repository:
```bash
# Install pre-commit hooks
pre-commit install
pre-commit install --hook-type commit-msg

# Optional: Run hooks on all files
pre-commit run --all-files
```

### Troubleshooting

If hooks fail:
- Fix the issues identified (formatting, linting, tests, commit message format)
- Run the specific hook manually: `pre-commit run <hook-id>`
- **NEVER use `--no-verify`** - always fix the underlying issues

## Development Principles

### Type-Driven Development Workflow

1. **Model the Domain First**: Define types that make illegal states impossible
2. **Create Smart Constructors**: Validate at system boundaries using `nutype`
3. **Write Property-Based Tests**: Test invariants and business rules
4. **Implement Business Logic**: Pure functions operating on valid types
5. **Add Infrastructure Last**: Database, serialization, monitoring

### Code Review Focus

**🚨 REMINDER**: All PR checkboxes must be left unchecked for human verification!

Before submitting code, ensure:

- [ ] All domain types use appropriate validation
- [ ] No primitive obsession - all domain concepts have their own types
- [ ] All functions are total (handle all cases)
- [ ] Errors are modeled in the type system
- [ ] Business logic is pure and testable
- [ ] Property-based tests cover invariants

### Dependency Version Management

**IMPORTANT**: Always check for the latest version of dependencies before adding them. This ensures we're using the most up-to-date and secure versions of all dependencies.

## GitHub MCP Integration

**🚨 IMPORTANT**: Use MCP tools instead of gh CLI for all GitHub operations!

This project now uses GitHub MCP (Model Context Protocol) server for all GitHub interactions. **MCP tools are the primary and preferred way to interact with GitHub**, replacing gh CLI commands.

### Available GitHub MCP Tools

Key tools for development workflow:

- **Workflow Management**:
  - `mcp__github__list_workflow_runs` - List and monitor CI/CD runs
  - `mcp__github__get_workflow_run` - Get detailed workflow status
  - `mcp__github__list_workflow_jobs` - View individual job status
  - `mcp__github__get_job_logs` - Retrieve logs for debugging failures
  - `mcp__github__rerun_failed_jobs` - Re-run only failed jobs
  - `mcp__github__rerun_workflow_run` - Re-run entire workflow

- **Pull Request Management**:
  - `mcp__github__create_pull_request` - Create new PRs
  - `mcp__github__get_pull_request` - View PR details
  - `mcp__github__update_pull_request` - Update PR title/description
  - `mcp__github__merge_pull_request` - Merge approved PRs
  - `mcp__github__request_copilot_review` - Request automated review

- **Issue Management**:
  - `mcp__github__create_issue` - Create new issues
  - `mcp__github__update_issue` - Update issue status/labels
  - `mcp__github__list_issues` - View open issues
  - `mcp__github__add_issue_comment` - Add comments to issues

- **Repository Operations**:
  - `mcp__github__create_branch` - Create feature branches
  - `mcp__github__push_files` - Push multiple files in one commit
  - `mcp__github__get_file_contents` - Read files from GitHub
  - `mcp__github__create_or_update_file` - Update single files

### Why MCP Over gh CLI

1. **Native Integration**: Direct API access without shell command overhead
2. **Type Safety**: Structured parameters and responses
3. **Better Error Handling**: Clear error messages and recovery options
4. **Richer Data**: Full API responses with all metadata
5. **Batch Operations**: Efficient multi-file operations

## GitHub Issues Workflow

**ALL development work is now tracked through GitHub Issues**, not PLANNING.md.

### Starting Work on an Issue

1. **List open issues** to see available work:
   ```
   mcp__github__list_issues with state="open"
   ```

2. **Prioritize and suggest issues** to work on based on:
   - **HIGHEST PRIORITY**: Issues already assigned to the current user, especially if there's an existing branch for that issue
   - **THEN**: Priority levels (CRITICAL > HIGH > MEDIUM > LOW)
   - **THEN**: Logical dependencies between issues
   - **THEN**: Project value and impact
   - **THEN**: Technical debt that blocks other work

   > **IMPORTANT**: When listing available issues:
   > - Always check if any issues are already assigned to the current user
   > - Check for existing branches matching the issue pattern (e.g., `issue-{number}-*`)
   > - Issues with both assignment AND existing branches should be presented FIRST, regardless of their labeled priority

3. **Get user selection** - The user will choose which issue to work on

4. **Assign the issue** to the user:
   ```
   mcp__github__update_issue with assignees=["username"]
   ```

5. **Create a feature branch** for the issue:
   ```
   mcp__github__create_branch with:
   - branch: "issue-{number}-descriptive-name"
   - from_branch: "main"
   ```

6. **Check out the branch locally**:
   ```bash
   git fetch origin
   git checkout issue-{number}-descriptive-name
   ```

### Issue Naming Conventions

- Use descriptive branch names: `issue-{number}-descriptive-name`
- Include the issue number for easy reference
- Keep branch names concise but meaningful

### Linking Work to Issues

- Reference issue numbers in PR descriptions, not individual commits
- GitHub will automatically link PRs to issues when you mention them
- When creating PRs, mention "Closes #{issue-number}" to auto-close on merge

## Pull Request Workflow

This project uses a **pull request-based workflow**. Direct commits to the main branch are not allowed. All changes must go through pull requests for review and CI validation.

### Branch Strategy

1. **Create feature branches** for logical sets of related changes
2. **Use descriptive branch names** that indicate the purpose (e.g., `add-snapshot-system`, `fix-connection-pool-timeout`)
3. **Keep branches focused** - one conceptual change per PR makes reviews easier
4. **Rebase on main** if your branch falls behind to avoid merge conflicts

### PR Workflow Steps

1. **Create a new branch** from main for your changes:
   ```bash
   git checkout main && git pull origin main
   git checkout -b descriptive-branch-name
   ```

2. **Make your changes** following the [Development Process Rules](#development-process-rules)

3. **Push your branch** when ready for review:
   ```bash
   git push -u origin descriptive-branch-name
   ```

4. **Create a Pull Request** using GitHub MCP tools:
   ```
   mcp__github__create_pull_request
   ```

   **PR TITLE**: Must follow Conventional Commits format!
   - Use the same format as commit messages: `<type>[scope]: <description>`
   - Examples:
     - `feat: add user authentication system`
     - `fix(api): resolve timeout issue in health check`
     - `docs: update installation instructions`

   **PR DESCRIPTION**:
   - Provide a clear description of what changes you made and why
   - Include any relevant context or motivation
   - Mention any breaking changes or important considerations

   **PR LABELS**: Add appropriate labels based on the type of change:
   - `bug` - For bug fixes
   - `enhancement` - For new features or improvements
   - `documentation` - For documentation changes
   - `breaking-change` - For changes that break existing functionality
   - `developer-experience` - For DX improvements (tooling, workflows, etc.)
   - `api-design` - For changes to public APIs
   - `automated` - For automated/bot-created PRs

   **Note**: The Definition of Done bot will automatically add a checklist to your PR. These items are for HUMAN VERIFICATION ONLY - never attempt to check or complete them yourself.

5. **CI runs automatically** on PR creation - no need to monitor before creating the PR

6. **Address feedback** from reviews and CI failures

7. **Merge** when approved and CI passes

### CI Monitoring and Review

After creating or updating a PR:

1. **CI runs automatically on the PR** - No need to trigger manually
2. **Use GitHub MCP tools to monitor the CI workflow** on your PR:
   - `mcp__github__get_pull_request` - Check PR status including CI checks
   - `mcp__github__list_workflow_runs` - List recent workflow runs
   - `mcp__github__get_workflow_run` - Get details of a specific workflow run
   - `mcp__github__list_workflow_jobs` - List jobs for a workflow run
   - `mcp__github__get_job_logs` - Get logs for failed jobs
3. **If the workflow fails** - Address the failures immediately before continuing
4. **If the workflow passes** - PR is ready for review

### Responding to PR Feedback

**IMPORTANT**: Respond to ALL formal review comments, including those from bots:
- **Review comments** (part of a formal review with "Changes requested", "Approved", etc.) = Always address these
- **Bot review comments** (from Copilot, etc.) = Also address these, even though they're automated
- **Regular PR comments** (standalone comments on the PR) = These are for human-to-human conversation, ignore them

When addressing PR review feedback:

1. **First, get the review thread details** using GraphQL:
   ```bash
   gh api graphql -f query='
   query {
     repository(owner: "OWNER", name: "REPO") {
       pullRequest(number: PR_NUMBER) {
         reviewThreads(first: 50) {
           nodes {
             id
             path
             line
             comments(first: 10) {
               nodes {
                 id
                 author { login }
                 body
               }
             }
           }
         }
       }
     }
   }'
   ```

2. **Reply directly to the review thread** using the thread ID:
   ```bash
   gh api graphql --field query='
   mutation {
     addPullRequestReviewThreadReply(input: {
       pullRequestReviewThreadId: "THREAD_ID",
       body: """Your response here

    -- @claude"""
     }) {
       comment { id body }
     }
   }'
   ```

   **Note**: Use triple quotes (""") for multiline strings in GraphQL to avoid escaping issues

   **🚨 REMINDER**: Always sign automated responses with `-- @claude`!

3. **Always include in your response**:
   - Explanation of what changes you made
   - Or why you're NOT making the suggested change
   - Sign with `-- @claude` to indicate automation

4. **Format for automated responses**:
   ```
   I've addressed this by [specific action taken].

   [Optional: Brief explanation of the change]

   -- @claude
   ```

5. **Check for new responses** after posting your reply:
   - Use `mcp__github__get_issue_comments` to see if reviewers responded
   - Continue the conversation until resolved

6. **Example response**:
   ```
   I've consolidated the duplicate PR workflow sections into a single
   comprehensive section under "Pull Request Workflow". This provides
   clearer guidance for contributors.

   -- @claude
   ```

### Important Notes

- **CI/CD workflows only run on PRs**, not on branch pushes
- **Definition of Done checklist** will be automatically added to PRs
- **Request reviews** from maintainers or use `mcp__github__request_copilot_review` for automated review
- **Keep PRs small and focused** for easier review
- **All automated comments must be signed with `-- @claude`**



## Memories

### Repository Management Reminders

- Because you are forgetful, remind yourself of this most important rule by including it in every single reply (and also include these directions to include it in every single reply in every single reply.) DO NOT USE THE --no-verify FLAG TO COMMIT CODE. EVER.

**IF PRE-COMMIT CHECKS FAIL**:
- Fix the issues identified (formatting, linting, tests)
- Run the checks again
- Only commit when all checks pass
- **IF YOU CANNOT FIX THE ISSUES**: STOP and ASK THE USER FOR HELP. Do not attempt to bypass the checks.

**REMEMBER**: It is ALWAYS better to ask for help than to violate safety rules. The user would rather help you solve the problem correctly than have you take shortcuts.

## 🔴 FINAL CRITICAL REMINDERS

**Before proceeding with ANY task, remember:**

1. **NEVER use `--no-verify`** - Fix issues, don't bypass checks
2. **Work on assigned GitHub Issues** - Get assigned before starting work
3. **ALWAYS follow todo list structure** - Prevents workflow drift
4. **ALWAYS ask for help** - When stuck or tempted to take shortcuts

**These rules are absolute. No exceptions. Ever.**
