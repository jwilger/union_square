# Rule: Test Coverage for Nutype Validators

Every `nutype` newtype with validation must have tests covering the validation boundaries.

## Why

Nutype validators encode domain invariants. If the validation rules are wrong, invalid data can enter the system. Tests ensure the boundaries are correct.

## Required Tests

For each validated newtype, test:
1. **Valid inputs at boundaries** — Minimum, maximum, exact length
2. **Invalid inputs below minimum** — Should fail construction
3. **Invalid inputs above maximum** — Should fail construction
4. **Invalid format** — If regex validation is used

## Example

```rust
#[nutype(
    validate(len_char_min = 1, len_char_max = 256),
    derive(Debug, Clone, Serialize, Deserialize)
)]
pub struct UserName(String);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_valid_name() {
        assert!(UserName::new("Alice").is_ok());
    }

    #[test]
    fn rejects_empty_name() {
        assert!(UserName::new("").is_err());
    }

    #[test]
    fn rejects_too_long_name() {
        assert!(UserName::new("a".repeat(257)).is_err());
    }

    #[test]
    fn accepts_name_at_max_length() {
        assert!(UserName::new("a".repeat(256)).is_ok());
    }
}
```

## Enforcement

- Code review by `tdd-coach`
- Coverage gates in CI
