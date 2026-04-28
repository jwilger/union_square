use std::{env, fs, path::PathBuf, process};

const REQUIRED_KEYS: &[&str] = &[
    "issue:",
    "goal:",
    "examples:",
    "given:",
    "when:",
    "then:",
    "acceptance_criteria:",
    "non_goals:",
    "architecture_impacts:",
    "test_trace_ids:",
];

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.first().map(String::as_str) != Some("check") {
        return Err("usage: us-spec check --issue <number>".to_string());
    }

    let issue = flag_value(&args, "--issue")
        .ok_or_else(|| "missing required --issue <number>".to_string())?;
    let path = PathBuf::from(format!(".codex/specs/issue-{issue}.yaml"));
    let text = fs::read_to_string(&path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;

    let mut missing = Vec::new();
    for key in REQUIRED_KEYS {
        if !text.lines().any(|line| line.trim_start().starts_with(key)) {
            missing.push(*key);
        }
    }

    if !missing.is_empty() {
        return Err(format!(
            "{} is missing required keys: {}",
            path.display(),
            missing.join(", ")
        ));
    }

    println!("behavior spec is valid: {}", path.display());
    Ok(())
}

fn flag_value(args: &[String], flag: &str) -> Option<String> {
    args.windows(2)
        .find(|pair| pair[0] == flag)
        .map(|pair| pair[1].clone())
}
