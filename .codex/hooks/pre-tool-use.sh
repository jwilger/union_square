#!/usr/bin/env bash
set -euo pipefail

input="$(cat)"
if ! printf '%s' "$input" | jq -e . >/dev/null 2>&1; then
  echo "pre-tool-use hook requires valid JSON input." >&2
  exit 1
fi

command="$(printf '%s' "$input" | jq -r '.tool_input.command // .tool_input.cmd // .command // empty')"
path="$(printf '%s' "$input" | jq -r '.tool_input.path // .tool_input.file_path // .tool_input.filePath // empty')"

if [[ -n "$path" ]] && [[ "$path" =~ (^|/)(\\.env|.*credentials.*|.*secret.*|.*token.*|id_rsa|.*\\.pem|.*\\.key)$ ]]; then
  echo "Access to potentially sensitive file is blocked: $path" >&2
  exit 1
fi

if [[ -z "$command" ]]; then
  if [[ -n "$path" ]]; then
    exit 0
  fi
  echo "pre-tool-use hook requires a command or path to evaluate." >&2
  exit 1
fi

active_issue() {
  local newest
  newest="$(
    { find .codex/state -maxdepth 1 -name 'issue-*.json' -print0 2>/dev/null || true; } \
      | while IFS= read -r -d '' ledger; do
          updated_at="$(jq -r '.updated_at_unix // 0' "$ledger" 2>/dev/null || printf '0')"
          printf '%s\t%s\n' "$updated_at" "$ledger"
        done \
      | sort -rn \
      | head -n 1 \
      | cut -f2-
  )"
  newest="${newest##*/}"
  newest="${newest#issue-}"
  newest="${newest%.json}"
  printf '%s' "$newest"
}

require_valid_spec_for_active_issue() {
  local action="$1"
  local issue
  issue="$(active_issue)"
  if [[ -z "$issue" ]] || ! cargo run --manifest-path tools/us-spec/Cargo.toml -- check --issue "$issue" >/dev/null 2>&1; then
    echo "$action requires a valid behavior spec for the active issue." >&2
    exit 1
  fi
}

if [[ "$command" =~ git[[:space:]]+commit.*--no-verify ]]; then
  echo "The --no-verify flag is forbidden. Fix hooks or ask for help." >&2
  exit 1
fi

if [[ "$command" =~ git[[:space:]]+push.*(^|[[:space:]:/])(main|master)($|[[:space:]]) ]]; then
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
  require_valid_spec_for_active_issue "PR actions"
fi

if [[ "$command" =~ git[[:space:]]+commit ]]; then
  if ! cargo run --manifest-path tools/us-agent/Cargo.toml -- require commit_ready >/dev/null 2>&1; then
    echo "Commits require us-agent state commit_ready." >&2
    exit 1
  fi
  require_valid_spec_for_active_issue "Commits"
fi
