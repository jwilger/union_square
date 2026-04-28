#!/usr/bin/env bash
set -euo pipefail

input="$(cat)"
prompt="$(printf '%s' "$input" | jq -r '.prompt // .user_prompt // .message // empty' 2>/dev/null || true)"

if [[ "$prompt" =~ --no-verify|bypass[[:space:]]+(tests|hooks|checks)|ignore[[:space:]]+(tests|hooks|checks)|show[[:space:]]+(secret|token|credential) ]]; then
  echo "Request conflicts with Union Square safety policy." >&2
  exit 1
fi
