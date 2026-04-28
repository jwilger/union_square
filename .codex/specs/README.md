# Behavior Specs

Behavior specs are issue-scoped BDD contracts validated by `just spec ISSUE=<number>`.

Create specs as `.codex/specs/issue-<number>.yaml` with this shape:

```yaml
issue: 123
goal: Describe the behavior outcome.
examples:
  - id: example-1
    name: user-visible behavior
    given:
      - precondition
    when:
      - action
    then:
      - observable result
acceptance_criteria:
  - criterion
non_goals:
  - explicitly out of scope
architecture_impacts:
  - none
test_trace_ids:
  - example-1:tests/some_acceptance.rs::test_name
```
