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

if printf '%s' '{"tool_input":{"command":"git push origin feature --force"}}' | .codex/hooks/pre-tool-use.sh 2>/dev/null; then
  echo "expected force push to be blocked" >&2
  exit 1
fi

if printf '%s' '{"tool_input":{"command":"rtk git reset --hard"}}' | .codex/hooks/pre-tool-use.sh 2>/dev/null; then
  echo "expected RTK-wrapped destructive git command to be blocked" >&2
  exit 1
fi

if printf '%s' '{"tool_input":{"command":"rtk git commit --no-verify -m test"}}' | .codex/hooks/pre-tool-use.sh 2>/dev/null; then
  echo "expected RTK-wrapped --no-verify commit to be blocked" >&2
  exit 1
fi

if printf '%s' '{"tool_input":{"command":"rtk proxy git push origin feature --force-with-lease"}}' | .codex/hooks/pre-tool-use.sh 2>/dev/null; then
  echo "expected RTK proxy force push to be blocked" >&2
  exit 1
fi

if command -v rtk >/dev/null 2>&1; then
  rtk_stderr="$(mktemp)"
  if printf '%s' '{"tool_input":{"command":"git status --short"}}' | .codex/hooks/pre-tool-use.sh 2>"$rtk_stderr"; then
    echo "expected raw read-heavy git status to be nudged through RTK" >&2
    rm -f "$rtk_stderr"
    exit 1
  fi
  if ! grep -q 'rtk git status --short' "$rtk_stderr"; then
    echo "expected RTK rewrite suggestion for raw git status" >&2
    rm -f "$rtk_stderr"
    exit 1
  fi
  rm -f "$rtk_stderr"
  if printf '%s' '{"tool_input":{"command":"rtk git status --short"}}' | .codex/hooks/pre-tool-use.sh >/dev/null; then
    :
  else
    echo "expected RTK-wrapped git status to be allowed" >&2
    exit 1
  fi
elif printf '%s' '{"tool_input":{"command":"git status --short"}}' | .codex/hooks/pre-tool-use.sh >/dev/null; then
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
