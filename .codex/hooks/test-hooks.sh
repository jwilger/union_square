#!/usr/bin/env bash
set -euo pipefail

if [[ -e .codex/state || -e .codex/specs/issue-217.yaml ]]; then
  echo "refusing to run hook smoke tests with existing issue 217 state/spec" >&2
  exit 1
fi

cleanup() {
  rm -rf .codex/state
  rm -f .codex/specs/issue-217.yaml
}
trap cleanup EXIT

cat > .codex/specs/issue-217.yaml <<'SPEC'
issue: 217
goal: Hooks enforce issue workflow state before irreversible actions.
examples:
  - id: rejects-pr-before-ready
    name: PR command before ready state is rejected
    given:
      - an active issue ledger is not pr_ready
    when:
      - the agent attempts to create a PR
    then:
      - the hook rejects the command
acceptance_criteria:
  - hooks reject incomplete workflow states
non_goals:
  - replacing CI gates
architecture_impacts:
  - none
test_trace_ids:
  - rejects-pr-before-ready:.codex/hooks/test-hooks.sh
SPEC

cargo run --manifest-path tools/us-agent/Cargo.toml -- start-issue 217 >/dev/null

if printf '%s' '{"tool_input":{"command":"gh pr create --fill"}}' | .codex/hooks/pre-tool-use.sh 2>/dev/null; then
  echo "expected PR hook to reject incomplete workflow state" >&2
  exit 1
fi

for action in \
  record-branch \
  record-spec \
  record-test-list \
  record-red \
  record-green \
  record-test-adversary \
  record-fitness \
  record-refactor \
  record-review \
  ready-to-commit \
  ready-to-pr
do
  cargo run --manifest-path tools/us-agent/Cargo.toml -- "$action" 217 >/dev/null
done

printf '%s' '{"tool_input":{"command":"gh pr create --fill"}}' | .codex/hooks/pre-tool-use.sh >/dev/null

rm -f .codex/specs/issue-217.yaml
if printf '%s' '{"tool_input":{"command":"git commit -m test"}}' | .codex/hooks/pre-tool-use.sh 2>/dev/null; then
  echo "expected commit hook to reject missing active behavior spec" >&2
  exit 1
fi

echo "hook workflow smoke tests passed"
