use std::fs;

#[test]
fn performance_baseline_contract_documents_commands_thresholds_and_coverage() -> Result<(), String>
{
    let justfile = fs::read_to_string("Justfile")
        .map_err(|error| format!("Justfile should be readable: {error}"))?;
    let baselines = fs::read_to_string("docs/performance/baselines.md")
        .map_err(|error| format!("performance baselines should be documented: {error}"))?;

    let bench_quick = just_target_body(&justfile, "bench-quick")?;
    assert!(
        bench_quick.contains("cargo test --test benchmark_validation")
            && bench_quick.contains("cargo bench --bench proxy_performance -- --quick --noplot"),
        "bench-quick should run deterministic validation plus quick Criterion checks"
    );

    let bench_local = just_target_body(&justfile, "bench-local")?;
    assert!(
        bench_local.contains("cargo bench --bench proxy_performance -- --noplot")
            && bench_local.contains("cargo bench --bench memory_profiling")
            && bench_local.contains("cargo test --test load_testing --release")
            && bench_local.contains("test_500_rps_sustained_load")
            && bench_local.contains("test_2000_rps_burst_load")
            && bench_local.contains("test_1000_concurrent_users")
            && bench_local.contains("--ignored")
            && bench_local.contains("--nocapture")
            && bench_local.contains("--test-threads=1"),
        "bench-local should run full proxy benchmark, memory profiling, and ignored release-mode load tests"
    );

    for required in [
        "just bench-quick",
        "just bench-local",
        "Ring-buffer write latency",
        "Ring-buffer read throughput",
        "Ring-buffer overflow behavior",
        "Hot-path proxy overhead",
        "Representative audit handoff cost",
        "CI validation",
        "Local benchmark",
        "Regression threshold",
    ] {
        assert!(
            baselines.contains(required),
            "performance baseline documentation should mention {required}"
        );
    }

    Ok(())
}

fn just_target_body<'a>(justfile: &'a str, target: &str) -> Result<&'a str, String> {
    let prefix = format!("{target}:");
    block_from_line(
        justfile,
        |line| line.starts_with(&prefix),
        is_just_target_header,
        || format!("Justfile should define {target}"),
    )
}

fn block_from_line(
    text: &str,
    is_start: impl Fn(&str) -> bool,
    is_next_block: impl Fn(&str) -> bool,
    missing_message: impl FnOnce() -> String,
) -> Result<&str, String> {
    let mut start = None;
    let mut offset = 0;

    for line in text.split_inclusive('\n') {
        let trimmed_newline = line.trim_end_matches(['\r', '\n']);
        if let Some(start_offset) = start {
            if offset != start_offset && is_next_block(trimmed_newline) {
                return Ok(&text[start_offset..offset]);
            }
        } else if is_start(trimmed_newline) {
            start = Some(offset);
        }

        offset += line.len();
    }

    start
        .map(|start_offset| &text[start_offset..])
        .ok_or_else(missing_message)
}

fn is_just_target_header(line: &str) -> bool {
    !line.is_empty() && !line.starts_with(' ') && !line.starts_with('\t') && line.contains(':')
}
