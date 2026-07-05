#!/usr/bin/env bash
set -euo pipefail

pr_number="${DISPATCH_PR_NUMBER:-${EVENT_PR_NUMBER:-}}"
if [[ -z "$pr_number" ]]; then
  echo "No pull request number was available for this event." >&2
  exit 1
fi
if [[ -z "${TRUNK_API_TOKEN:-}" ]]; then
  echo "TRUNK_API_TOKEN was not loaded from 1Password." >&2
  exit 1
fi

pr_json="$(gh pr view "$pr_number" -R "$REPOSITORY_OWNER/$REPOSITORY_NAME" --json number,state,isDraft,baseRefName,headRepositoryOwner,url)"
state="$(jq -r '.state' <<<"$pr_json")"
is_draft="$(jq -r '.isDraft' <<<"$pr_json")"
base_ref="$(jq -r '.baseRefName' <<<"$pr_json")"
head_owner="$(jq -r '.headRepositoryOwner.login' <<<"$pr_json")"
pr_url="$(jq -r '.url' <<<"$pr_json")"
default_branch="$(gh api "repos/$REPOSITORY_OWNER/$REPOSITORY_NAME" --jq '.default_branch')"

if [[ "$state" != "OPEN" ]]; then
  echo "Skipping $pr_url because it is $state."
  exit 0
fi
if [[ "$is_draft" == "true" ]]; then
  echo "Skipping $pr_url because it is a draft."
  exit 0
fi
if [[ "$base_ref" != "$default_branch" ]]; then
  echo "Skipping $pr_url because its base branch ($base_ref) is not the default branch."
  exit 0
fi
if [[ "$EVENT_NAME" != "workflow_dispatch" && "$head_owner" != "$REPOSITORY_OWNER" ]]; then
  echo "Skipping $pr_url because automatic submission is limited to same-owner branches."
  exit 0
fi

payload="$(
  jq -nc \
    --arg owner "$REPOSITORY_OWNER" \
    --arg name "$REPOSITORY_NAME" \
    --arg targetBranch "$base_ref" \
    --argjson prNumber "$pr_number" \
    '{
      repo: {
        host: "github.com",
        owner: $owner,
        name: $name
      },
      targetBranch: $targetBranch,
      pr: {
        number: $prNumber
      }
    }'
)"

response_file="$(mktemp)"
status="$(
  curl --silent --show-error \
    --max-time 30 \
    --connect-timeout 10 \
    --output "$response_file" \
    --write-out "%{http_code}" \
    --request POST \
    --url https://api.trunk.io/v1/submitPullRequest \
    --header "Content-Type: application/json" \
    --header "x-api-token: $TRUNK_API_TOKEN" \
    --data "$payload"
)"

if [[ "$status" =~ ^2[0-9][0-9]$ ]]; then
  echo "Submitted $pr_url to the Trunk merge queue."
  cat "$response_file"
  exit 0
fi

if [[ "$status" == "409" ]]; then
  echo "$pr_url was already submitted to the Trunk merge queue."
  cat "$response_file"
  exit 0
fi

echo "Trunk submit failed with HTTP $status." >&2
cat "$response_file" >&2
exit 1
