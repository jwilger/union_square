#!/usr/bin/env bash
set -euo pipefail

input="$(cat)"
command="$(printf '%s' "$input" | jq -r '.tool_input.command // .tool_input.cmd // .command // empty' 2>/dev/null || true)"
path="$(printf '%s' "$input" | jq -r '.tool_input.path // .tool_input.file_path // .tool_input.filePath // empty' 2>/dev/null || true)"

if [[ -n "$path" ]] && [[ "$path" =~ (^|/)(\\.env|.*credentials.*|.*secret.*|.*token.*|id_rsa|.*\\.pem|.*\\.key)$ ]]; then
  echo "Access to potentially sensitive file is blocked: $path" >&2
  exit 1
fi

if [[ -z "$command" ]]; then
  exit 0
fi

if [[ "$command" =~ git[[:space:]]+commit([^\\n]*)--no-verify ]]; then
  echo "The --no-verify flag is forbidden. Fix hooks or ask for help." >&2
  exit 1
fi

if [[ "$command" =~ git[[:space:]]+push([^\\n]*)(main|master) ]]; then
  echo "Direct pushes to main/master are forbidden. Use a PR." >&2
  exit 1
fi

if [[ "$command" =~ git[[:space:]]+reset[[:space:]]+--hard|git[[:space:]]+clean|git[[:space:]]+rebase ]]; then
  echo "Destructive git command blocked by project policy." >&2
  exit 1
fi

if [[ "$command" =~ gh[[:space:]]+pr[[:space:]]+create|gh[[:space:]]+pr[[:space:]]+ready ]]; then
  if ! cargo run --manifest-path tools/us-agent/Cargo.toml -- require pr_ready >/dev/null 2>&1; then
    echo "PR actions require us-agent state pr_ready." >&2
    exit 1
  fi
  ledger_path="$(find .codex/state -maxdepth 1 -name 'issue-*.json' -print -quit 2>/dev/null || true)"
  active_issue="${ledger_path##*/}"
  active_issue="${active_issue#issue-}"
  active_issue="${active_issue%.json}"
  if [[ -z "$active_issue" ]] || ! cargo run --manifest-path tools/us-spec/Cargo.toml -- check --issue "$active_issue" >/dev/null 2>&1; then
    echo "PR actions require a valid behavior spec for the active issue." >&2
    exit 1
  fi
fi

if [[ "$command" =~ git[[:space:]]+commit ]]; then
  if ! cargo run --manifest-path tools/us-agent/Cargo.toml -- require commit_ready >/dev/null 2>&1; then
    echo "Commits require us-agent state commit_ready." >&2
    exit 1
  fi
  ledger_path="$(find .codex/state -maxdepth 1 -name 'issue-*.json' -print -quit 2>/dev/null || true)"
  active_issue="${ledger_path##*/}"
  active_issue="${active_issue#issue-}"
  active_issue="${active_issue%.json}"
  if [[ -z "$active_issue" ]] || ! cargo run --manifest-path tools/us-spec/Cargo.toml -- check --issue "$active_issue" >/dev/null 2>&1; then
    echo "Commits require a valid behavior spec for the active issue." >&2
    exit 1
  fi
fi
