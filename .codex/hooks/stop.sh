#!/usr/bin/env bash
set -euo pipefail

input="$(cat)"
active="$(printf '%s' "$input" | jq -r '.stop_hook_active // false' 2>/dev/null || true)"
if [[ "$active" == "true" ]]; then
  exit 0
fi

files="$(
  git diff --name-only --diff-filter=ACMRD HEAD
  git diff --cached --name-only --diff-filter=ACMRD HEAD
  git ls-files --others --exclude-standard
)"
files="$(printf '%s\n' "$files" | sort -u)"
if printf '%s\n' "$files" | grep -Eq '(^|/)(src|tests|benches)/|(^|/)Cargo\.(toml|lock)$'; then
  just fitness
fi
