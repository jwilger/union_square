# Rule: GitHub Actions Security

All third-party GitHub Actions MUST be pinned to an immutable commit SHA.

## Why

Mutable tags (`@v2`, `@stable`, `@master`) can be retagged to point to malicious code without changing the workflow file. Pinning to a commit SHA ensures the exact code that was reviewed is what runs.

## Pattern

### Allowed

```yaml
uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683
uses: dtolnay/rust-toolchain@29eef336d9b2848a0b548edc03f92a220660cdb8
```

### Forbidden

```yaml
uses: actions/checkout@v4
uses: dtolnay/rust-toolchain@stable
uses: Swatinem/rust-cache@v2
uses: some-org/some-action@master
```

## Exceptions

GitHub-owned actions (`actions/*`) MAY use version tags since GitHub provides strong integrity guarantees for these. However, commit SHA pins are still preferred.

## Enforcement

- Use `zizmor` (security linter for GitHub Actions) in CI to detect unpinned third-party actions
- Code review by `security-reviewer`
