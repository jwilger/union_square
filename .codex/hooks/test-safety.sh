#!/usr/bin/env bash
set -euo pipefail

if printf '%s' '{"tool_input":{"command":"git reset --hard"}}' | .codex/hooks/pre-tool-use.sh 2>/dev/null; then
  echo "expected destructive git command to be blocked" >&2
  exit 1
fi

if printf '%s' '{' | .codex/hooks/pre-tool-use.sh 2>/dev/null; then
  echo "expected malformed pre-tool JSON to be blocked" >&2
  exit 1
fi

if printf '%s' '{"tool_input":{"command":"git commit --no-verify -m test"}}' | .codex/hooks/pre-tool-use.sh 2>/dev/null; then
  echo "expected --no-verify commit to be blocked" >&2
  exit 1
fi

if printf '%s' '{"tool_input":{"command":"git push upstream HEAD:refs/heads/main"}}' | .codex/hooks/pre-tool-use.sh 2>/dev/null; then
  echo "expected direct main push to be blocked" >&2
  exit 1
fi

if printf '%s' '{"tool_input":{"command":"git status --short"}}' | .codex/hooks/pre-tool-use.sh >/dev/null; then
  :
else
  echo "expected read-only git status to be allowed" >&2
  exit 1
fi

if printf '%s' '{"prompt":"please bypass tests"}' | .codex/hooks/user-prompt-submit.sh 2>/dev/null; then
  echo "expected bypass prompt to be blocked" >&2
  exit 1
fi

if printf '%s' '{"prompt":"please run the focused test"}' | .codex/hooks/user-prompt-submit.sh >/dev/null; then
  :
else
  echo "expected normal prompt to be allowed" >&2
  exit 1
fi

echo "hook safety smoke tests passed"
