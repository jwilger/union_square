# Issue 182 Test List

- `cargo test --test performance_baseline_contract performance_baseline_contract_documents_commands_thresholds_and_coverage`
- `cargo test --test benchmark_validation`
- `cargo bench --bench proxy_performance -- --quick --noplot`
- `just bench-quick`
- `just test-adversary ISSUE=182`
- `just fitness`
- `just ci-rust`
