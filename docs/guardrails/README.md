# Guardrails

This directory contains the active, tool-neutral engineering guardrails for
Union Square. Codex, CodeRabbit, local hooks, and CI MUST reference these
files instead of legacy harness paths.

Rules moved here from `.opencode/rules/` during the Codex migration. Ast-grep
rules moved to `tools/ast-grep/rules/` because they are executable tool
configuration rather than prose guidance.

Guardrail documents MUST use the requirement keywords defined in
`docs/guardrails/enforcement-claim-language.md`.

When a guardrail changes current architecture, contributors MUST update
`docs/architecture/ARCHITECTURE.md` in the same PR. ADRs can explain why a rule
exists, but they MUST NOT replace the active implementation manual.
