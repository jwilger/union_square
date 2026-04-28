#!/usr/bin/env bash
set -euo pipefail

input="$(cat)"
active="$(printf '%s' "$input" | jq -r '.stop_hook_active // false' 2>/dev/null || true)"
if [[ "$active" == "true" ]]; then
  exit 0
fi

if git diff --quiet --exit-code && git diff --cached --quiet --exit-code; then
  exit 0
fi

files="$(git diff --name-only --diff-filter=ACMR; git diff --cached --name-only --diff-filter=ACMR)"
if printf '%s\n' "$files" | grep -Eq '(^src/|^tests/|^benches/|Cargo\.(toml|lock)$)'; then
  just fitness
fi
