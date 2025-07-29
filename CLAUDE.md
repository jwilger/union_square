# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 🚨 CRITICAL RULES - ALWAYS APPLY

**These rules must NEVER be violated under any circumstances:**

1. **NEVER use the `--no-verify` flag when committing code**
2. **ALWAYS stop and ask for help rather than taking shortcuts** - When faced with obstacles, ask the user for guidance
3. **ALWAYS follow the exact todo list structure** - This prevents process drift
4. **Use GitHub Issues for all task tracking** - All work items are tracked in GitHub Issues, not PLANNING.md

## Project Overview

Union Square is a proxy/wire-tap service for making LLM calls and recording everything that happens in a session for later analysis and test-case extraction.

## Development Process Rules

**🚨 REMINDER: Review [Critical Rules](#critical-rules---always-apply) before proceeding!**

When working on this project, **ALWAYS** follow these rules:

1. **Review GitHub Issues** to discover work items. Use `mcp__github__list_issues` to see open issues.
2. **Get assigned to an issue** before starting work. The user will select which issue to work on.
3. **Create a feature branch** for the issue using `mcp__github__create_branch`.
4. **Follow the Pull Request Workflow** (see `git-workflow-architect` agent) for all code changes.
5. **IMMEDIATELY use the todo list tool** to create a todolist with the specific actions you will take to complete the task.
6. **Insert a task to "Make a commit"** after each discrete action that involves a change to the code, tests, database schema, or infrastructure.
7. **The FINAL item in the todolist MUST always be** to "Push your changes to the remote repository and create/update PR with GitHub MCP tools."

### CRITICAL: Todo List Structure

Your todo list should ALWAYS follow this pattern:

**For work on GitHub Issues:**
1. START with writing tests for any changes BEFORE making the changes
2. Implementation/fix tasks (the actual work)
3. "Make a commit" (pre-commit hooks run all checks automatically)
4. "Push changes and update PR"

**Why this matters**: The todo list tool reinforces our workflow at every step, preventing process drift as context grows.

## Core Development Principles

- **Type-driven development**: See `type-theory-reviewer`, `type-driven-development-expert`, `rust-type-system-expert` agents
- **Event sourcing with EventCore**: See `event-sourcing-architect` agent for all EventCore patterns and usage
- **Test-driven development**: See `tdd-coach` agent for TDD practices
- **Git workflows**: See `git-workflow-architect` agent for commits, PRs, and GitHub operations

## Quick Setup

```bash
nix develop                # Enter development environment
pre-commit install         # Install git hooks
docker-compose up -d       # Start databases
```

For detailed development commands and setup, consult the appropriate expert agents.

## Available Expert Agents

**IMPORTANT**: Expert agents are active members of your development team who write code, not just reviewers!

**Note**: These are AI agents inspired by domain experts, not the actual people.

### 🚨 CRITICAL: Multi-Agent Collaboration

**You MUST launch multiple agents concurrently (up to 10) for complex tasks!** Single-agent launches should be the exception, not the rule.

#### When to Launch Multiple Agents:
- **New Features**: Always launch 3-5 relevant agents together
- **Bug Fixes**: Launch domain expert + implementation expert + test architect
- **Architecture Decisions**: Get multiple perspectives simultaneously
- **When Any Agent Makes an [AGENT_REQUEST]**: Immediately launch ALL requested agents

#### How to Launch Multiple Agents:
Include multiple Task tool invocations in a SINGLE message. This launches them concurrently for faster, more comprehensive solutions.

| Agent Name | Domain Expertise | When to Engage |
|------------|------------------|----------------|
| `type-theory-reviewer` | Type theory, making illegal states unrepresentable | Type safety improvements |
| `event-sourcing-architect` | Event sourcing, CQRS, EventCore patterns | All event sourcing work |
| `event-modeling-expert` | Event storming, domain discovery | Domain modeling |
| `type-driven-development-expert` | Type-driven development, dependent types | Type design |
| `rust-type-system-expert` | Rust type system, ownership, lifetimes | Rust-specific patterns |
| `event-sourcing-test-architect` | Testing event-sourced systems | Event testing strategies |
| `tdd-coach` | Test-driven development, red-green-refactor | All TDD practices |
| `functional-architecture-expert` | Functional design, simplicity | Architecture decisions |
| `engineering-effectiveness-expert` | DORA metrics, workflow optimization | Process improvements |
| `product-discovery-coach` | Continuous discovery, outcomes | Feature planning |
| `ux-research-expert` | User research, API design | Developer experience |
| `continuous-delivery-architect` | CI/CD, deployment strategies | Deployment patterns |
| `async-rust-expert` | Async Rust, concurrent systems | Async implementations |
| `refactoring-patterns-architect` | Refactoring, design patterns | Code improvements |
| `git-workflow-architect` | Git workflows, GitHub automation | All git/GitHub operations |

## Inter-Agent Communication Protocol

Expert agents cannot directly invoke each other. You must facilitate their communication.

### 🚨 CRITICAL: Always Watch for [AGENT_REQUEST] Blocks!

**You MUST monitor ALL agent responses for [AGENT_REQUEST] blocks and IMMEDIATELY launch the requested agents!**

### When an agent needs input from another agent:

1. **Look for [AGENT_REQUEST] blocks** in agent responses:
   ```
   [AGENT_REQUEST]
   TO: event-sourcing-architect, type-driven-development-expert
   QUESTION: How should we handle event versioning in this type-safe command structure?
   CONTEXT: [relevant context]
   [/AGENT_REQUEST]
   ```

2. **When you see [AGENT_REQUEST]**:
   - **IMMEDIATELY** launch ALL requested agents in a single message
   - Include the question, context, and requesting agent's name
   - Wait for their responses before proceeding

3. **Agents respond with [AGENT_RESPONSE]**:
   ```
   [AGENT_RESPONSE]
   TO: functional-architecture-expert
   RE: Your question about immutability in event streams
   RESPONSE: [detailed response]
   [/AGENT_RESPONSE]
   ```

4. **Continue exchanges until consensus is reached**
   - Pass responses back to the original agent
   - Facilitate multiple rounds if needed
   - Synthesize the final solution from all perspectives

**FAILURE TO FACILITATE**: If you ignore [AGENT_REQUEST] blocks, you're preventing the team from collaborating effectively!

Remember: Expert agents are active participants who write code, not just reviewers. They should be engaged to implement solutions, not just provide advice.

## Architecture Decision Records (ADRs)

Document significant architectural decisions using ADRs:
- Review existing ADRs in `docs/adr/` before making changes
- Create new ADRs for significant decisions
- Follow naming convention: `NNNN-descriptive-name.md`
- Published automatically to GitHub Pages

## Pre-commit Hooks

**🚨 CRITICAL**: These hooks ensure code quality. NEVER bypass them with `--no-verify`!

Pre-commit hooks run automatically on every commit:
- Rust formatting, linting, tests, and type checking
- File checks (whitespace, YAML/TOML/JSON syntax)
- Conventional Commits message validation

If hooks fail: Fix the issues, never use `--no-verify`!

## GitHub Operations

**Use GitHub MCP tools for all GitHub operations** - See `git-workflow-architect` agent for detailed workflows including:
- Issue management and prioritization
- Pull request creation and management
- CI/CD monitoring
- PR feedback responses

**Key points**:
- All work tracked through GitHub Issues
- Use MCP tools, not gh CLI
- Follow issue branch naming: `issue-{number}-descriptive-name`
- PR titles must follow Conventional Commits format
- Sign all automated responses with `-- @claude`

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
