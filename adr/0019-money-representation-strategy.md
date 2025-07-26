# 0019. Money Representation Strategy

Date: 2024-01-26
Status: Accepted

## Context

The Union Square proxy needs to track costs associated with LLM API calls for audit and analysis purposes. This requires:
1. Storing prices per thousand tokens (often fractional cents, e.g., $0.003)
2. Calculating actual costs based on token usage
3. Serializing/deserializing monetary values for storage and API responses
4. Ensuring proper rounding for billing purposes

We evaluated several approaches:
- Using floating-point numbers (rejected due to precision issues with money)
- Using `rust_decimal::Decimal` throughout
- Using dedicated money crates like `rusty-money`, `steel-cent`, or `currencies`
- Building our own money type

## Decision Drivers

- **Precision**: Must accurately represent fractional cents for pricing
- **Type Safety**: Prevent mixing monetary values with regular numbers
- **Currency Support**: Should handle currency information (initially USD only)
- **Serialization**: Must support serde for JSON API responses
- **Rounding Rules**: Must support standard financial rounding (ceiling for costs)
- **Performance**: Should not significantly impact response times

## Considered Options

### Option 1: Decimal Everywhere
Use `rust_decimal::Decimal` for both prices and costs.

**Pros:**
- Simple, single type for all monetary values
- Arbitrary precision
- Good serde support

**Cons:**
- No currency information
- No type distinction between prices and money
- Easy to accidentally mix with non-monetary decimals

### Option 2: rusty-money Throughout
Use `rusty-money` crate for all monetary values.

**Pros:**
- Dedicated money type with currency support
- Type safety
- Rich API for money operations

**Cons:**
- No built-in serde support (deal breaker)
- Cannot represent fractional cents well

### Option 3: Hybrid Approach
Use `Decimal` for prices (per-thousand-tokens) and a money crate for final costs.

**Pros:**
- Appropriate types for each use case
- Type safety for actual money values
- Can represent fractional cent prices

**Cons:**
- Two different types to manage
- Potential confusion about when to use which

## Decision Outcome

We chose **Option 3: Hybrid Approach** using:
- `rust_decimal::Decimal` for price-per-thousand-tokens
- `currencies::Amount<USD>` for final cost calculations

### Implementation Details

```rust
/// Price per thousand tokens (can be fractional cents)
#[nutype(
    validate(predicate = |price| *price >= Decimal::ZERO),
    derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, AsRef)
)]
pub struct PricePerThousandTokens(Decimal);

/// Calculate cost with ceiling rounding
pub fn calculate_cost(
    &self,
    input_tokens: InputTokens,
    output_tokens: OutputTokens,
) -> Amount<USD> {
    // Calculate in Decimal for precision
    let total_cost_decimal = /* calculation */;

    // Convert to cents with ceiling rounding
    let total_cents = (total_cost_decimal * Decimal::from(100)).ceil();
    let cents_u64 = total_cents.try_into().unwrap_or(0);

    Amount::<USD>::from_raw(cents_u64)
}
```

### Rounding Strategy

All costs are rounded UP to the next penny (ceiling rounding), which is standard practice for usage-based billing systems. This ensures:
- Providers are never under-compensated
- Consistent with industry practices
- Simple and predictable for users

## Consequences

### Positive
- Type safety prevents mixing prices with costs
- Currency information is preserved in cost values
- Proper financial rounding is enforced
- Clear distinction between pricing models and actual charges
- Good serialization support for API responses

### Negative
- Breaking API change: `ProviderMetadata.cost_estimate` type changed
- Developers must understand when to use each type
- Additional dependency on `currencies` crate
- Conversion logic needed between Decimal and Amount

### Future Considerations
- Easy to extend to other currencies when needed
- Could add convenience methods for common conversions
- May want to create specialized types for different pricing models

## Links

- [currencies crate documentation](https://docs.rs/currencies/)
- [rust_decimal documentation](https://docs.rs/rust_decimal/)
- PR #136 - Initial implementation
