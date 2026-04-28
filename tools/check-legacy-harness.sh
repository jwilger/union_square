#!/usr/bin/env bash
set -euo pipefail

legacy_paths=(
  ".opencode"
  ".claude"
  "opencode.jsonc"
  "CLAUDE.md"
)

for path in "${legacy_paths[@]}"; do
  if [[ -e "$path" ]]; then
    echo "legacy harness path still exists: $path" >&2
    exit 1
  fi
done

if git ls-files "${legacy_paths[@]}" | grep -q .; then
  echo "legacy harness paths are still tracked by git" >&2
  exit 1
fi

echo "legacy harness cleanup passed"
