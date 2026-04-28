# Rule: Review Feedback Protocol

When responding to PR review comments, follow this exact protocol.

## Process

1. **Read every comment** — Do not skip bot reviews or automated checks
2. **Classify the feedback**:
   - `action-required`: Must fix before merge
   - `clarification`: Ask question if unclear
   - `suggestion`: Consider and decide
   - `praise`: Acknowledge with thanks
3. **Address each piece of feedback** in order of severity
4. **Reply to threads** using GraphQL mutation
5. **Format**: "I've addressed this by [specific action]."
6. **Check for responses** and continue conversation until resolved

Do not manually request bot re-review after pushing fixes unless explicitly asked; review bots normally re-run automatically.

## Rules

- **Never dismiss feedback without responding** — Even if you disagree, explain your reasoning
- **Make atomic commits per feedback theme** — Don't lump unrelated fixes together
- **Update tests when fixing code** — Review feedback often reveals missing test cases
- **Verify with CI** — Ensure all checks pass after addressing feedback
- **Do not trigger bot re-review manually by default** — Check for automatic review results instead

## GraphQL Reply Format

```graphql
mutation {
  addPullRequestReviewThreadReply(
    input: {
      pullRequestReviewThreadId: "THREAD_ID"
      body: "I've addressed this by adding the missing validation check in `src/domain/order.rs`."
    }
  ) {
    reply {
      id
    }
  }
}
```

## Enforcement

- This rule is self-enforcing through the PR workflow
- Unaddressed review comments block merge
