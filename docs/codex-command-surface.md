# Codex Command Surface

Use `just` as the stable command API for local agents, developers, and CI.

Core checks:

- `just fmt-check`
- `just clippy`
- `just check`
- `just test`
- `just test-doc`
- `just ast-grep`
- `just ast-grep-branch`
- `just fitness`
- `just audit`
- `just coverage`
- `just build`
- `just build-release`
- `just bench-quick`
- `just bench-local`

Harness checks:

- `just check-tools`
- `just clippy-tools`
- `just test-tools`
- `just test-hooks`
- `just ci-harness`
- `just legacy-harness-check`
- `just ci-rust`
- `just ci-full`

Workflow commands:

- `just spec ISSUE=<number>`
- `just test-adversary ISSUE=<number>`
- `just agent start-issue <number>`
- `just agent record-branch <number>`
- `just agent record-spec <number>`
- `just agent record-test-list <number>`
- `just agent record-red <number>`
- `just agent record-green <number>`
- `just agent record-test-adversary <number>`
- `just agent record-fitness <number>`
- `just agent record-refactor <number>`
- `just agent record-review <number>`
- `just agent ready-to-commit <number>`
- `just agent ready-to-pr <number>`

Local services:

- `just db-up`

`just db-up` starts PostgreSQL on host port `55432` and test PostgreSQL on
host port `55433` by default. Override with `POSTGRES_PORT` and
`POSTGRES_TEST_PORT` when a local workflow needs different ports.

Token-saving shell wrapper:

- `rtk rewrite '<command>'`
- `rtk <command>`
- `rtk proxy <command>`

Use RTK for high-output read-heavy commands such as `git diff`, `rg`,
`cargo test`, and `docker compose logs`. Do not use RTK or `rtk proxy` to bypass
project policies; hooks normalize RTK-wrapped commands before enforcing git and
workflow guardrails.

CI uses the same command surface where practical. Add new repeated commands here
first, then call them from hooks, CI, or agent instructions. The `Justfile` is
the executable source of truth, and `.github/workflows/ci.yml` must call these
recipes rather than duplicating command bodies where practical.
