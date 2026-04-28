# Guardrails

This directory contains the active, tool-neutral engineering guardrails for
Union Square. Codex, CodeRabbit, local hooks, and CI should reference these
files instead of legacy harness paths.

Rules moved here from `.opencode/rules/` during the Codex migration. Ast-grep
rules moved to `tools/ast-grep/rules/` because they are executable tool
configuration rather than prose guidance.

When a guardrail changes current architecture, update
`docs/architecture/ARCHITECTURE.md` in the same PR. ADRs can explain why a rule
exists, but they are not the active implementation manual.
