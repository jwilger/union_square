# Rule: Merge Main Into Feature Branch, Never Rebase

When a feature branch falls behind `main`, always merge `main` into the feature branch. Never rebase the feature branch onto `main`.

## Why

- **Preserves history**: Merge commits document when and why the branch was updated
- **Avoids force pushes**: Rebase requires force-pushing rewritten history, which is destructive and dangerous in shared branches
- **Safer for collaboration**: Other contributors may have checked out the branch; rebasing breaks their local copies
- **Easier to review**: PR reviewers can see the merge commit and understand the timeline

## Correct Workflow

```bash
# When your feature branch is behind main
git fetch origin
git checkout feat/your-feature-branch
git merge origin/main
# Resolve any conflicts, then commit the merge
git push origin feat/your-feature-branch
```

## Forbidden

```bash
# NEVER do this
git rebase origin/main
git push --force origin feat/your-feature-branch
```

## Enforcement

- This rule is self-enforcing through code review
- Force pushes to feature branches should trigger alerts
