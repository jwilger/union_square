---
description: Run tests and analyze failures
agent: build
---

Run the full test suite with `cargo nextest run --workspace` (fallback to `cargo test --workspace`).

If failures exist:
1. Identify the failing test(s)
2. Analyze the root cause from the output
3. Suggest specific fixes
4. Do NOT modify code without explicit approval

Also report test coverage status and any warnings.
