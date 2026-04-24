# Rule: Refactor Sequencing

When refactoring, follow a safe, incremental sequence.

## The Sequence

1. **Add characterization tests** — Capture current behavior before changing anything
2. **Make the change easy** — Refactor to a clean structure that supports the new behavior
3. **Make the easy change** — Implement the new behavior in the clean structure
4. **Verify** — All tests pass, including characterization tests

## Rules

- **Never refactor and change behavior in the same commit** — Separate commits for "restructure" and "change"
- **Green tests before and after** — Refactoring should not break tests
- **Commit after each safe step** — Small, atomic commits make rollback easy
- **Use the compiler** — Let the type system guide the refactoring

## Example

```bash
# Commit 1: Add characterization tests
git commit -m "test: add characterization tests for order processing"

# Commit 2: Extract pure function from handler
git commit -m "refactor: extract calculate_total from PlaceOrder handler"

# Commit 3: Change the calculation logic
git commit -m "feat: apply discount to orders over $100"
```

## Enforcement

- Code review by `refactoring-patterns-architect`
- PRs mixing refactor and feature changes are rejected
