use std::{
    env, fs,
    path::{Path, PathBuf},
    process,
    time::{SystemTime, UNIX_EPOCH},
};

const STATES: &[&str] = &[
    "issue_selected",
    "branch_created",
    "behavior_spec_written",
    "test_list_written",
    "red_test_observed",
    "green_observed",
    "test_adversary_passed",
    "fitness_passed",
    "refactor_reviewed",
    "expert_review_done",
    "commit_ready",
    "pr_ready",
];

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    match args.first().map(String::as_str) {
        Some("start-issue") => write_state(
            &required_issue_arg(&args, 1)?,
            "issue_selected",
            "IssueSelected",
        ),
        Some("record-branch") => transition(
            &required_issue_arg(&args, 1)?,
            "branch_created",
            "BranchCreated",
        ),
        Some("record-spec") => transition(
            &required_issue_arg(&args, 1)?,
            "behavior_spec_written",
            "BehaviorSpecWritten",
        ),
        Some("record-test-list") => transition(
            &required_issue_arg(&args, 1)?,
            "test_list_written",
            "TestListWritten",
        ),
        Some("record-red") => transition(
            &required_issue_arg(&args, 1)?,
            "red_test_observed",
            "RedTestObserved",
        ),
        Some("record-green") => transition(
            &required_issue_arg(&args, 1)?,
            "green_observed",
            "GreenObserved",
        ),
        Some("record-test-adversary") => transition(
            &required_issue_arg(&args, 1)?,
            "test_adversary_passed",
            "TestAdversaryPassed",
        ),
        Some("record-fitness") => transition(
            &required_issue_arg(&args, 1)?,
            "fitness_passed",
            "FitnessPassed",
        ),
        Some("record-refactor") => transition(
            &required_issue_arg(&args, 1)?,
            "refactor_reviewed",
            "RefactorReviewed",
        ),
        Some("record-review") => transition(
            &required_issue_arg(&args, 1)?,
            "expert_review_done",
            "ExpertReviewCompleted",
        ),
        Some("ready-to-commit") => transition(
            &required_issue_arg(&args, 1)?,
            "commit_ready",
            "CommitReady",
        ),
        Some("ready-to-pr") => transition(&required_issue_arg(&args, 1)?, "pr_ready", "PrReady"),
        Some("status") => status(args.get(1).map(String::as_str)),
        Some("require") => require_state(required_arg(&args, 1)?),
        Some("export-pr-summary") => export_summary(&required_issue_arg(&args, 1)?),
        _ => Err(usage()),
    }
}

fn transition(issue: &str, next: &str, event: &str) -> Result<(), String> {
    let current = read_state(issue)?;
    validate_transition(issue, &current, next)?;
    write_state(issue, next, event)
}

fn status(issue: Option<&str>) -> Result<(), String> {
    let issue = match issue {
        Some(issue) => validate_issue_id(issue)?,
        None => active_issue()?.0,
    };
    println!(
        "{}",
        fs::read_to_string(ledger_path(&issue)).map_err(|error| error.to_string())?
    );
    Ok(())
}

fn require_state(required: &str) -> Result<(), String> {
    let (issue, state) = active_issue()?;
    if state_index(&state)? < state_index(required)? {
        return Err(format!(
            "active issue {issue} is in state {state}; required at least {required}"
        ));
    }
    Ok(())
}

fn export_summary(issue: &str) -> Result<(), String> {
    let state = read_state(issue)?;
    println!("{}", export_summary_text(issue, &state));
    Ok(())
}

fn write_state(issue: &str, state: &str, event: &str) -> Result<(), String> {
    fs::create_dir_all(".codex/state").map_err(|error| error.to_string())?;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_secs();
    let body = format!(
        "{{\n  \"issue\": \"{issue}\",\n  \"state\": \"{state}\",\n  \"last_event\": \"{event}\",\n  \"updated_at_unix\": {timestamp}\n}}\n"
    );
    fs::write(ledger_path(issue), body).map_err(|error| error.to_string())?;
    println!("issue {issue} state: {state}");
    Ok(())
}

fn read_state(issue: &str) -> Result<String, String> {
    let text = fs::read_to_string(ledger_path(issue))
        .map_err(|error| format!("failed to read ledger for issue {issue}: {error}"))?;
    read_state_from_text(issue, &text)
}

fn read_state_from_text(issue: &str, text: &str) -> Result<String, String> {
    parse_json_string_field(text, "state")
        .ok_or_else(|| format!("ledger for issue {issue} has no state field"))
}

fn validate_transition(issue: &str, current: &str, next: &str) -> Result<(), String> {
    let current_index = state_index(current)?;
    let next_index = state_index(next)?;
    if next_index != current_index + 1 {
        return Err(format!(
            "invalid transition for issue {issue}: {current} -> {next}"
        ));
    }
    Ok(())
}

fn export_summary_text(issue: &str, state: &str) -> String {
    format!(
        "## Codex Workflow Evidence\n\n- Issue: #{issue}\n- State: `{state}`\n- Spec: `.codex/specs/issue-{issue}.yaml`\n- Ledger: local `.codex/state/issue-{issue}.json`"
    )
}

fn active_issue() -> Result<(String, String), String> {
    let dir = Path::new(".codex/state");
    let entries =
        fs::read_dir(dir).map_err(|error| format!("no active us-agent ledger: {error}"))?;
    let mut candidates = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let text = fs::read_to_string(&path).map_err(|error| error.to_string())?;
        if let (Some(issue), Some(state)) = (
            parse_json_string_field(&text, "issue"),
            parse_json_string_field(&text, "state"),
        ) {
            let issue = validate_issue_id(&issue)?;
            let updated_at = parse_json_u64_field(&text, "updated_at_unix").unwrap_or(0);
            candidates.push((updated_at, issue, state));
        }
    }
    candidates
        .into_iter()
        .max_by_key(|(updated_at, _, _)| *updated_at)
        .map(|(_, issue, state)| (issue, state))
        .ok_or_else(|| "no active us-agent ledger found".to_string())
}

fn ledger_path(issue: &str) -> PathBuf {
    PathBuf::from(format!(".codex/state/issue-{issue}.json"))
}

fn state_index(state: &str) -> Result<usize, String> {
    STATES
        .iter()
        .position(|candidate| *candidate == state)
        .ok_or_else(|| format!("unknown state: {state}"))
}

fn parse_json_string_field(text: &str, field: &str) -> Option<String> {
    let needle = format!("\"{field}\"");
    let line = text.lines().find(|line| line.contains(&needle))?;
    let value = line.split_once(':')?.1.trim().trim_end_matches(',').trim();
    Some(value.trim_matches('"').to_string())
}

fn parse_json_u64_field(text: &str, field: &str) -> Option<u64> {
    let needle = format!("\"{field}\"");
    let line = text.lines().find(|line| line.contains(&needle))?;
    line.split_once(':')?
        .1
        .trim()
        .trim_end_matches(',')
        .parse()
        .ok()
}

fn required_arg(args: &[String], index: usize) -> Result<&str, String> {
    args.get(index).map(String::as_str).ok_or_else(usage)
}

fn required_issue_arg(args: &[String], index: usize) -> Result<String, String> {
    validate_issue_id(required_arg(args, index)?)
}

fn validate_issue_id(issue: &str) -> Result<String, String> {
    let parsed = issue
        .parse::<u64>()
        .map_err(|error| format!("invalid issue number `{issue}`: {error}"))?;
    Ok(parsed.to_string())
}

fn usage() -> String {
    "usage: us-agent start-issue|record-branch|record-spec|record-test-list|record-red|record-green|record-test-adversary|record-fitness|record-refactor|record-review|ready-to-commit|ready-to-pr|status|require|export-pr-summary <issue-or-state>".to_string()
}

#[cfg(test)]
mod tests {
    use super::{
        export_summary_text, read_state_from_text, validate_issue_id, validate_transition,
    };

    #[test]
    fn fixture_ledger_allows_next_transition() {
        let text = include_str!("../fixtures/ledger-red.json");
        let state = read_state_from_text("216", text).expect("fixture should have state");

        validate_transition("216", &state, "green_observed")
            .expect("red should transition to green");
    }

    #[test]
    fn fixture_ledger_rejects_skipped_transition() {
        let text = include_str!("../fixtures/ledger-red.json");
        let state = read_state_from_text("216", text).expect("fixture should have state");
        let error =
            validate_transition("216", &state, "fitness_passed").expect_err("cannot skip states");

        assert!(error.contains("red_test_observed -> fitness_passed"));
    }

    #[test]
    fn export_pr_summary_includes_issue_state_spec_and_ledger() {
        let summary = export_summary_text("216", "pr_ready");

        assert!(summary.contains("Issue: #216"));
        assert!(summary.contains("State: `pr_ready`"));
        assert!(summary.contains(".codex/specs/issue-216.yaml"));
        assert!(summary.contains(".codex/state/issue-216.json"));
    }

    #[test]
    fn issue_ids_must_be_numeric_before_paths_are_built() {
        let error = validate_issue_id("../../tmp/x").expect_err("path traversal is rejected");

        assert!(error.contains("invalid issue number"));
    }
}
