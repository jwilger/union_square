# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 🚨 CRITICAL RULES - ALWAYS APPLY

**These rules must NEVER be violated under any circumstances:**

1. **NEVER use the `--no-verify` flag when committing code**
2. **ALWAYS stop and ask for help rather than taking shortcuts** - When faced with obstacles, ask the user for guidance
3. **ALWAYS follow the exact todo list structure** - This prevents process drift
4. **Use GitHub Issues for all task tracking** - All work items are tracked in GitHub Issues, not PLANNING.md

**🚨 CRITICAL REMINDER**: The `--no-verify` flag is forbidden when committing code.

## 📋 QUICK REFERENCE

- **🆕 Starting new work?** → [Development Workflow](#development-workflow), [GitHub Issues](#github-issues-workflow)
- **🔧 Setting up environment?** → [Development Commands](#development-commands)
- **💻 Writing code?** → Read `.opencode/rules/*.md` for coding standards
- **🤖 Need expert guidance?** → Use `.opencode/agents/` expert subagents
- **📤 Making commits/PRs?** → [Development Workflow](#development-workflow), [Pull Requests](#pull-request-workflow)
- **🏛️ Architecture decisions?** → Read `docs/architecture/ARCHITECTURE.md`, [ADRs](#architecture-decision-records-adrs)

## Project Overview

Union Square is a proxy/wire-tap service for making LLM calls and recording everything that happens in a session for later analysis and test-case extraction.

## Development Workflow

**🚨 ALWAYS follow this exact workflow and todo list structure:**

### Workflow Steps

1. **Review GitHub Issues** - Use GitHub MCP tools (`mcp__github__list_issues`) or `gh issue list`
2. **Get assigned to an issue** - User selects which issue to work on
3. **Create feature branch** - Use GitHub MCP tools (`mcp__github__create_branch`) or `gh`, with pattern: `issue-{number}-descriptive-name`
4. **IMMEDIATELY create todo list** with this exact structure:
   - START with writing tests BEFORE implementation (ensure tests fail as expected)
   - Implementation/fix tasks (the actual work)
   - "Make a commit" (pre-commit hooks run all checks automatically)
   - "Push changes and update PR"

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

## Development Commands

### Environment Setup
```bash
nix develop                                    # Enter dev environment
lefthook install                               # Install hooks (first time)
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

See `docs/architecture/ARCHITECTURE.md` for the implementation source of truth.

## Guardrails & Standards

The following authoritative sources define how code is written in this project:

- **Type-Driven Development**: `.opencode/rules/type-driven-development.md`
- **EventCore Patterns**: `.opencode/rules/eventcore-command-pattern.md`
- **TDD Workflow**: `.opencode/rules/outside-in-tdd-execution.md`
- **No Panics**: `.opencode/rules/no-panics-in-production.md`
- **Error Handling**: `.opencode/rules/thiserror-for-errors.md`
- **ADR Conventions**: `.opencode/rules/adrs.md`
- **Review Protocol**: `.opencode/rules/review-feedback-protocol.md`
- **PR Hygiene**: `.opencode/rules/pr-scope-hygiene.md`

## Expert Agent Coordination

This project includes specialized AI agents that embody the expertise of renowned software architects. Invoke them via `@agentname` or the Task tool.

### Available Expert Agents

| Persona            | Agent Name                     | Domain Expertise                                                           |
| ------------------ | ------------------------------ | -------------------------------------------------------------------------- |
| Simon Peyton Jones | `type-theory-reviewer`         | Type theory, functional programming, making illegal states unrepresentable |
| Greg Young         | `event-sourcing-architect`     | Event sourcing, CQRS, distributed systems                                  |
| Alberto Brandolini | `event-modeling-expert`        | Event storming, domain discovery, bounded contexts                         |
| Edwin Brady        | `type-driven-development-expert` | Type-driven development, dependent types, formal verification              |
| Niko Matsakis      | `rust-type-system-expert`      | Rust type system, ownership, lifetimes, trait design                       |
| Michael Feathers   | `event-sourcing-test-architect`| Testing event-sourced systems, characterization tests                      |
| Kent Beck          | `tdd-coach`                    | Test-driven development, red-green-refactor cycle                          |
| Rich Hickey        | `functional-architecture-expert`| Functional design, simplicity, immutability                                |
| Nicole Forsgren    | `engineering-effectiveness-expert`| DORA metrics, development workflow optimization                          |
| Teresa Torres      | `product-discovery-coach`      | Continuous discovery, outcome-driven development                           |
| Jared Spool        | `ux-research-expert`           | User research, API design, developer experience                            |
| Jez Humble         | `continuous-delivery-architect`| CI/CD, deployment strategies, zero-downtime deployments                    |
| Yoshua Wuyts       | `async-rust-expert`            | Async Rust, concurrent systems, performance optimization                   |
| Martin Fowler      | `refactoring-patterns-architect`| Refactoring, design patterns, evolutionary architecture                    |
| Prem Sichanugrist  | `git-workflow-architect`       | Git workflows, GitHub automation, version control strategies               |
| Security Reviewer  | `security-reviewer`            | Security audits, vulnerability identification                              |
| Design Reviewer    | `design-reviewer`              | Code design, maintainability, coupling/cohesion                            |
| PR Feedback        | `pr-feedback-processor`        | Processing and responding to PR review comments                            |

### Core Architectural Principles

When multiple experts are involved in a decision, these principles guide resolution:

1. **Type Safety First**: When conflicts arise, type system recommendations (Simon Peyton Jones/Niko Matsakis) take precedence
2. **Event Sourcing is Non-Negotiable**: Greg Young's event patterns are foundational - other patterns must adapt to this
3. **TDD is the Process**: Kent Beck drives the implementation workflow - no code without tests
4. **Functional Core, Imperative Shell**: Rich Hickey owns the boundary between pure and impure code

### Decision Hierarchy

When experts disagree, follow this hierarchy:

1. **Domain Modeling Conflicts**: Primary: Alberto Brandolini → Secondary: Greg Young → Tiebreaker: Edwin Brady
2. **Implementation Approach Conflicts**: Primary: Affected domain expert → Secondary: Niko Matsakis → Tiebreaker: Rich Hickey
3. **Performance vs Correctness**: Default: Correctness first (Edwin Brady/Niko Matsakis) → Exception: Measurable UX impact (Nicole Forsgren) → Resolution: Yoshua Wuyts

## Architecture Decision Records (ADRs)

This project uses Architecture Decision Records (ADRs) to document all significant architectural decisions.

See `.opencode/rules/adrs.md` for full conventions. Key points:
- Filename: `NNNN-descriptive-name.md`
- Title: `# NNNN. Title`
- Template: `docs/adr/template.md`
- Old ADRs are immutable; supersede by creating a new ADR

## Pre-commit Hooks & Code Quality

**🚨 NEVER bypass with `--no-verify`!**

Hooks run automatically on commit via lefthook:
- **Rust**: `cargo fmt`, `cargo clippy`, `cargo test`, `cargo check`
- **Files**: Whitespace cleanup, syntax checks, large file prevention
- **Commits**: Conventional Commits format enforcement
- **Structural**: `ast-grep` rules for no-unwrap/no-panic/no-expect in production

**Setup**: `lefthook install`

**If hooks fail**: Fix the issues, don't bypass them. Ask for help if needed.

## GitHub Issues Workflow

**ALL work is tracked through GitHub Issues.** Both MCP tools and `gh` CLI are acceptable.

### Starting Work

1. **List issues**: Use MCP (`mcp__github__list_issues`) or `gh issue list --state open`
   - **🚨 CRITICAL**: API paginates! Check ALL pages with `perPage=5` until empty results.
2. **Priority order**: Assigned to you > CRITICAL > HIGH > MEDIUM > LOW
3. **Get assigned**: User selects issue; use MCP (`mcp__github__update_issue`) or `gh issue edit {number} --add-assignee @me`
4. **Create branch**: Use MCP (`mcp__github__create_branch`) or `gh issue develop {number} --checkout`, with pattern: `issue-{number}-descriptive-name`
5. **Local checkout**: `git fetch origin && git checkout issue-{number}-descriptive-name`

### GitHub Tooling

Both MCP tools and `gh` CLI are acceptable. Use whichever is more convenient.

**MCP**: `list_issues`, `update_issue`, `add_issue_comment`, `create_branch`, `create_pull_request`, `update_pull_request`, `list_workflow_runs`, `get_job_logs`, `rerun_failed_jobs`

**gh CLI**: `gh issue`, `gh pr`, `gh run`, `gh release`, etc.

## Pull Request Workflow

**All changes require PRs - no direct commits to main.**

### Creating PRs

1. **Push branch**: `git push -u origin branch-name`
2. **Create PR**: Use MCP (`mcp__github__create_pull_request`) or `gh pr create`
   - **Title**: Follow Conventional Commits format (`feat: add feature`)
   - **Description**: Clear explanation of changes and motivation
   - **Labels**: `bug`, `enhancement`, `documentation`, `breaking-change`, etc.
   - Mention "Closes #{issue-number}" to auto-close issues
3. **CI runs automatically** - Monitor with MCP tools or `gh pr checks` / `gh run view`

### Responding to Reviews

**Address ALL formal review comments (including bot reviews):**

1. **Get review details** using GraphQL API
2. **Reply to threads** using GraphQL mutation with `-- @claude` signature
3. **Format**: "I've addressed this by [action]. -- @claude"
4. **Check for responses** and continue conversation until resolved

## 🔴 FINAL REMINDERS

**Before ANY task:**
1. **NEVER use `--no-verify`** - Fix issues, don't bypass checks
2. **Work on assigned GitHub Issues** - Get assigned before starting work
3. **Follow exact todo list structure** - Prevents workflow drift
4. **Ask for help when stuck** - Don't take shortcuts

**If pre-commit checks fail**: Fix the issues, run again, only commit when all pass. **IF YOU CANNOT FIX**: STOP and ASK FOR HELP.

**These rules are absolute. No exceptions. Ever.**
