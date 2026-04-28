use std::{
    env, fs,
    path::{Component, Path},
    process,
};

const GREEN_OR_LATER_STATES: &[&str] = &[
    "green_observed",
    "test_adversary_passed",
    "fitness_passed",
    "refactor_reviewed",
    "expert_review_done",
    "commit_ready",
    "pr_ready",
];

const ASSERTION_MARKERS: &[&str] = &["assert!", "assert_eq!", "assert_ne!", "matches!"];

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.first().map(String::as_str) != Some("check") {
        return Err("usage: us-test-adversary check --issue <number>".to_string());
    }
    let issue = flag_value(&args, "--issue")
        .ok_or_else(|| "missing required --issue <number>".to_string())
        .and_then(|issue| validate_issue_id(&issue))?;

    run_adversary_check(Path::new("."), &issue)?;
    println!("targeted test adversary passed for issue {issue}");
    Ok(())
}

fn run_adversary_check(root: &Path, issue: &str) -> Result<(), String> {
    let ledger_path = root.join(format!(".codex/state/issue-{issue}.json"));
    let ledger = fs::read_to_string(&ledger_path)
        .map_err(|error| format!("failed to read {}: {error}", ledger_path.display()))?;
    let state = parse_json_string_field(&ledger, "state")
        .ok_or_else(|| format!("{} has no state field", ledger_path.display()))?;
    if !GREEN_OR_LATER_STATES.contains(&state.as_str()) {
        return Err(format!(
            "{} must be at least green_observed before adversary checks run",
            ledger_path.display()
        ));
    }

    let spec_path = root.join(format!(".codex/specs/issue-{issue}.yaml"));
    let spec = fs::read_to_string(&spec_path)
        .map_err(|error| format!("failed to read {}: {error}", spec_path.display()))?;
    let traces = extract_trace_entries(&spec)?;
    let mut findings = Vec::new();
    for trace in traces {
        if let Err(error) = validate_trace(root, &trace) {
            findings.push(error);
        }
    }

    if findings.is_empty() {
        Ok(())
    } else {
        Err(findings.join("\n"))
    }
}

fn extract_trace_entries(spec: &str) -> Result<Vec<String>, String> {
    let mut traces = Vec::new();
    let mut in_trace_ids = false;
    for line in spec.lines() {
        let trimmed = line.trim();
        if trimmed == "test_trace_ids:" {
            in_trace_ids = true;
            continue;
        }
        if in_trace_ids && trimmed.starts_with('#') {
            continue;
        }
        if in_trace_ids && !trimmed.starts_with('-') && !trimmed.is_empty() {
            break;
        }
        if in_trace_ids && trimmed.starts_with("- ") {
            traces.push(trimmed.trim_start_matches("- ").trim().to_string());
        }
    }

    if traces.is_empty() {
        Err("spec must include at least one test_trace_ids entry".to_string())
    } else {
        Ok(traces)
    }
}

fn validate_trace(root: &Path, trace: &str) -> Result<(), String> {
    let (_example_id, test_ref) = trace
        .split_once(':')
        .ok_or_else(|| format!("trace `{trace}` must use example-id:test-path format"))?;
    let test_path = test_ref
        .split_once("::")
        .map_or(test_ref, |(path, _test_name)| path);
    reject_escaping_path(test_path)?;
    let full_path = root.join(test_path);
    let test_text = fs::read_to_string(&full_path).map_err(|error| {
        format!(
            "failed to read traced test {}: {error}",
            full_path.display()
        )
    })?;

    if !test_text.contains("#[test]") && !test_text.contains("#[tokio::test]") {
        return Err(format!(
            "traced file `{test_path}` does not contain a Rust test"
        ));
    }
    if !ASSERTION_MARKERS
        .iter()
        .any(|marker| test_text.contains(marker))
    {
        return Err(format!(
            "traced test `{test_path}` has no assertion marker; weak tests are not accepted"
        ));
    }
    Ok(())
}

fn reject_escaping_path(test_path: &str) -> Result<(), String> {
    let path = Path::new(test_path);
    if path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(format!(
            "trace test path `{test_path}` must stay inside the repository"
        ));
    }
    Ok(())
}

fn parse_json_string_field(text: &str, field: &str) -> Option<String> {
    let needle = format!("\"{field}\"");
    let line = text.lines().find(|line| line.contains(&needle))?;
    let value = line.split_once(':')?.1.trim().trim_end_matches(',').trim();
    Some(value.trim_matches('"').to_string())
}

fn validate_issue_id(issue: &str) -> Result<String, String> {
    let parsed = issue
        .parse::<u64>()
        .map_err(|error| format!("invalid issue number `{issue}`: {error}"))?;
    Ok(parsed.to_string())
}

fn flag_value(args: &[String], flag: &str) -> Option<String> {
    args.windows(2)
        .find(|pair| pair[0] == flag)
        .map(|pair| pair[1].clone())
}

#[cfg(test)]
mod tests {
    use super::{extract_trace_entries, run_adversary_check, validate_issue_id, validate_trace};
    use std::path::Path;

    #[test]
    fn meaningful_test_fixture_passes() {
        run_adversary_check(Path::new("fixtures/strong"), "219")
            .expect("strong fixture should pass");
    }

    #[test]
    fn weak_test_fixture_fails() {
        let error = run_adversary_check(Path::new("fixtures/weak"), "219")
            .expect_err("weak fixture should fail");

        assert!(error.contains("weak tests are not accepted"));
    }

    #[test]
    fn issue_ids_must_be_numeric_before_paths_are_built() {
        let error = validate_issue_id("../../tmp/x").expect_err("path traversal is rejected");

        assert!(error.contains("invalid issue number"));
    }

    #[test]
    fn trace_parser_ignores_comments_inside_trace_list() {
        let spec =
            "test_trace_ids:\n  # comment\n  - first:tests/a.rs::one\n  - second:tests/b.rs::two\n";

        let traces = extract_trace_entries(spec).expect("comments should not stop parsing");

        assert_eq!(traces.len(), 2);
    }

    #[test]
    fn traced_test_paths_must_not_escape_repository() {
        let error = validate_trace(Path::new("."), "example:../outside.rs::test")
            .expect_err("path traversal is rejected");

        assert!(error.contains("must stay inside the repository"));
    }
}
