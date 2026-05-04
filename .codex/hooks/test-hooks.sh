#!/usr/bin/env bash
set -euo pipefail

backup_dir="$(mktemp -d)"
repo_root="$(pwd)"
had_state=0
had_issue_217_spec=0

restore_workflow_state() {
  local exit_code=$?

  rm -rf .codex/state
  rm -f .codex/specs/issue-217.yaml

  if [[ "$had_state" == "1" ]]; then
    mkdir -p .codex
    mv "$backup_dir/state" .codex/state
  fi

  if [[ "$had_issue_217_spec" == "1" ]]; then
    mkdir -p .codex/specs
    mv "$backup_dir/issue-217.yaml" .codex/specs/issue-217.yaml
  fi

  rm -rf "$backup_dir"
  exit "$exit_code"
}
trap restore_workflow_state EXIT

if [[ -e .codex/state ]]; then
  mv .codex/state "$backup_dir/state"
  had_state=1
fi

if [[ -e .codex/specs/issue-217.yaml ]]; then
  mv .codex/specs/issue-217.yaml "$backup_dir/issue-217.yaml"
  had_issue_217_spec=1
fi

mkdir -p .codex/specs

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

for draft_pr_command in \
  "gh pr create --draft --fill" \
  "gh pr create --fill --draft"
do
  draft_pr_output="$(
    printf '{"tool_input":{"command":"%s"}}' "$draft_pr_command" \
      | .codex/hooks/pre-tool-use.sh 2>&1
  )" && {
    echo "expected PR hook to reject draft PR creation after pr_ready: $draft_pr_command" >&2
    exit 1
  }
  if ! grep -qi "ready-for-review" <<<"$draft_pr_output"; then
    echo "expected draft PR rejection to explain ready-for-review default: $draft_pr_command" >&2
    exit 1
  fi
done

conflict_repo="$backup_dir/conflict-repo"
mkdir "$conflict_repo"
git -C "$conflict_repo" init -q
git -C "$conflict_repo" config user.email codex@example.invalid
git -C "$conflict_repo" config user.name Codex
{
  printf '%s\n' '<<<<<<< HEAD'
  printf '%s\n' 'left'
  printf '%s\n' '======='
  printf '%s\n' 'right'
  printf '%s\n' '>>>>>>> branch'
} > "$conflict_repo/conflict.txt"
git -C "$conflict_repo" add conflict.txt
staged_conflict_blob_before="$(git -C "$conflict_repo" rev-parse :conflict.txt)"
conflict_output="$(
  cd "$conflict_repo" && "$repo_root/tools/check-merge-conflict-markers.sh" 2>&1
)" && {
  echo "expected merge conflict marker check to fail on staged markers" >&2
  exit 1
}
if ! grep -qi "merge conflict" <<<"$conflict_output"; then
  echo "expected merge conflict marker check to report the failure" >&2
  exit 1
fi
staged_conflict_blob_after="$(git -C "$conflict_repo" rev-parse :conflict.txt)"
if [[ "$staged_conflict_blob_before" != "$staged_conflict_blob_after" ]]; then
  echo "expected merge conflict marker check to leave the staged blob unchanged" >&2
  exit 1
fi

rm -f .codex/specs/issue-217.yaml
if printf '%s' '{"tool_input":{"command":"git commit -m test"}}' | .codex/hooks/pre-tool-use.sh 2>/dev/null; then
  echo "expected commit hook to reject missing active behavior spec" >&2
  exit 1
fi

echo "hook workflow smoke tests passed"
