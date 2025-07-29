# TDD Checklist

Use this checklist for every new feature or bug fix to ensure proper TDD practices.

## Before Starting

- [ ] I understand the requirement/bug clearly
- [ ] I have identified the behavior I want to test
- [ ] I know what "done" looks like

## RED Phase - Write Failing Test First

- [ ] I wrote a test BEFORE writing any production code
- [ ] The test has a descriptive name explaining the behavior
- [ ] The test follows Arrange-Act-Assert or Given-When-Then pattern
- [ ] I ran the test and it FAILS
- [ ] The test fails for the RIGHT reason (not compilation error)
- [ ] I committed the failing test with `test:` prefix

## GREEN Phase - Make Test Pass

- [ ] I wrote the MINIMUM code to make the test pass
- [ ] I didn't add any extra functionality
- [ ] I didn't worry about perfect code yet
- [ ] All tests are now GREEN
- [ ] I committed with `feat:` or `fix:` prefix

## REFACTOR Phase - Improve Design

- [ ] All tests are still GREEN before refactoring
- [ ] I looked for duplication to remove
- [ ] I improved naming for clarity
- [ ] I extracted methods/functions where appropriate
- [ ] I ran tests after EACH refactoring step
- [ ] All tests are still GREEN after refactoring
- [ ] I committed each refactoring with `refactor:` prefix

## Code Review Readiness

- [ ] My commit history shows clear RED-GREEN-REFACTOR cycle
- [ ] Each test tests ONE specific behavior
- [ ] Tests are independent and can run in any order
- [ ] Tests document the behavior of the code
- [ ] No test implementation details are exposed
- [ ] Edge cases are covered with tests

## Common TDD Smells to Avoid

- [ ] I did NOT write production code without a failing test
- [ ] I did NOT write more than one failing test at a time
- [ ] I did NOT refactor while tests were red
- [ ] I did NOT skip the refactoring step
- [ ] I did NOT test private/internal implementation

## Example Commit Sequence

Good TDD commit history looks like:
```
test(auth): add failing test for expired token rejection
feat(auth): implement token expiration check
refactor(auth): extract token validation to separate function
test(auth): add failing test for malformed token handling
feat(auth): handle malformed tokens gracefully
```

## Quick Commands

```bash
# Run specific test
cargo test test_function_name

# Run tests in watch mode
cargo watch -x test

# Run tests with output
cargo test -- --show-output

# Run only unit tests (fast)
cargo test --lib

# Run with coverage
cargo tarpaulin
```

Remember: The goal is not just to have tests, but to use tests to drive better design!
