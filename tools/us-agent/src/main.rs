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
        Some("start-issue") => write_state(required_arg(&args, 1)?, "issue_selected", "started"),
        Some("record-branch") => transition(required_arg(&args, 1)?, "branch_created"),
        Some("record-spec") => transition(required_arg(&args, 1)?, "behavior_spec_written"),
        Some("record-test-list") => transition(required_arg(&args, 1)?, "test_list_written"),
        Some("record-red") => transition(required_arg(&args, 1)?, "red_test_observed"),
        Some("record-green") => transition(required_arg(&args, 1)?, "green_observed"),
        Some("record-test-adversary") => {
            transition(required_arg(&args, 1)?, "test_adversary_passed")
        }
        Some("record-fitness") => transition(required_arg(&args, 1)?, "fitness_passed"),
        Some("record-refactor") => transition(required_arg(&args, 1)?, "refactor_reviewed"),
        Some("record-review") => transition(required_arg(&args, 1)?, "expert_review_done"),
        Some("ready-to-commit") => transition(required_arg(&args, 1)?, "commit_ready"),
        Some("ready-to-pr") => transition(required_arg(&args, 1)?, "pr_ready"),
        Some("status") => status(args.get(1).map(String::as_str)),
        Some("require") => require_state(required_arg(&args, 1)?),
        Some("export-pr-summary") => export_summary(required_arg(&args, 1)?),
        _ => Err(usage()),
    }
}

fn transition(issue: &str, next: &str) -> Result<(), String> {
    let current = read_state(issue)?;
    let current_index = state_index(&current)?;
    let next_index = state_index(next)?;
    if next_index != current_index + 1 {
        return Err(format!(
            "invalid transition for issue {issue}: {current} -> {next}"
        ));
    }
    write_state(issue, next, "transition")
}

fn status(issue: Option<&str>) -> Result<(), String> {
    let issue = match issue {
        Some(issue) => issue.to_string(),
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
    println!("## Codex Workflow Evidence\n\n- Issue: #{issue}\n- State: `{state}`\n- Spec: `.codex/specs/issue-{issue}.yaml`\n- Ledger: local `.codex/state/issue-{issue}.json`");
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
    parse_json_string_field(&text, "state")
        .ok_or_else(|| format!("ledger for issue {issue} has no state field"))
}

fn active_issue() -> Result<(String, String), String> {
    let dir = Path::new(".codex/state");
    let entries =
        fs::read_dir(dir).map_err(|error| format!("no active us-agent ledger: {error}"))?;
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
            return Ok((issue, state));
        }
    }
    Err("no active us-agent ledger found".to_string())
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

fn required_arg(args: &[String], index: usize) -> Result<&str, String> {
    args.get(index).map(String::as_str).ok_or_else(usage)
}

fn usage() -> String {
    "usage: us-agent start-issue|record-spec|record-test-list|record-red|record-green|record-test-adversary|record-fitness|record-refactor|record-review|ready-to-commit|ready-to-pr|status|require|export-pr-summary <issue-or-state>".to_string()
}
