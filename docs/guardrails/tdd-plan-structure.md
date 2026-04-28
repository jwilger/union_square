# Rule: TDD Plan Structure

When planning implementation with TDD, create a structured test list before writing code.

## Plan Format

Before implementing a feature, create a todo list or test list:

```markdown
## Test List for Session Recording

### Acceptance Tests
- [ ] Proxy forwards request to Bedrock and returns response
- [ ] Session aggregates all interactions within a time window
- [ ] Extracted test case matches expected format

### Unit Tests
- [ ] SessionId rejects empty strings
- [ ] SessionId rejects strings > 256 chars
- [ ] RequestForwarded event contains all metadata
- [ ] ResponseReceived event links to correct request

### Property Tests
- [ ] Any valid session produces at least one event
- [ ] Session total requests equals count of RequestForwarded events
```

## Rules

1. **Write the test list BEFORE implementation** — Not as you go
2. **Check off tests as they pass** — Visual progress tracking
3. **Add discovered tests** — When you find edge cases during implementation, add them to the list
4. **Never skip a failing test** — If a test is too hard to make pass, ask for help

## Relationship to Todo Lists

The test list is part of the standard todo structure:
1. Write failing tests first (includes test list creation)
2. Implementation tasks
3. "Make a commit"
4. "Push changes and update PR"

## Enforcement

- Todo list structure enforces test-first workflow
- Code review by `tdd-coach`
