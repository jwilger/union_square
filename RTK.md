# RTK - Rust Token Killer

RTK is the preferred wrapper for high-output, read-heavy shell commands in
Codex sessions. It reduces token use by summarizing command output while keeping
the original command shape visible.

## Use RTK For

Prefer the RTK rewrite for inspection and verification commands that can produce
large output:

```bash
rtk git status --short
rtk git diff
rtk git log --oneline -20
rtk rg EventCore
rtk find . -name Cargo.toml
rtk cargo test --workspace
rtk cargo check --workspace
rtk docker compose logs
```

If the pre-tool hook rejects a raw read-heavy command with an RTK suggestion,
rerun the suggested `rtk ...` command.

## Guardrails

RTK is a token-saving wrapper, not a policy bypass. The project rules still
apply to RTK-wrapped commands and `rtk proxy` commands.

Do not use RTK to bypass these rules:

- no `--no-verify`
- no direct pushes to `main` or `master`
- no force pushes
- no destructive git commands unless the user explicitly asks for that exact
  operation
- no commit or PR actions before the `us-agent` workflow permits them

Use `rtk proxy <cmd>` only when you need complete unfiltered output for a
specific diagnostic or review task.

## Meta Commands

```bash
rtk gain
rtk gain --history
rtk rewrite 'git status --short'
rtk proxy <cmd>
```
