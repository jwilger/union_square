# Codex Command Surface

Use `just` as the stable command API for local agents, developers, and CI.

Core checks:

- `just fmt-check`
- `just clippy`
- `just check`
- `just test`
- `just test-doc`
- `just ast-grep`
- `just fitness`

Harness checks:

- `just check-tools`
- `just clippy-tools`
- `just test-tools`
- `just test-hooks`
- `just ci-harness`

Workflow commands:

- `just spec ISSUE=<number>`
- `just test-adversary ISSUE=<number>`
- `just agent start-issue <number>`
- `just agent record-branch <number>`
- `just agent ready-to-commit <number>`
- `just agent ready-to-pr <number>`

Local services:

- `just db-up`

`just db-up` starts PostgreSQL on host port `55432` and test PostgreSQL on
host port `55433` by default. Override with `POSTGRES_PORT` and
`POSTGRES_TEST_PORT` when a local workflow needs different ports.

CI uses the same command surface where practical. Add new repeated commands here
first, then call them from hooks, CI, or agent instructions.
