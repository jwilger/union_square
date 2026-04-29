#!/usr/bin/env bash
set -euo pipefail

found=0
while IFS= read -r -d '' file; do
  if git show ":$file" | grep -n '<<<<<<< '; then
    found=1
  fi
done < <(git diff --cached --name-only -z --diff-filter=ACMR)

if [[ "$found" == "1" ]]; then
  echo "Merge conflict markers found in staged files." >&2
  exit 1
fi
