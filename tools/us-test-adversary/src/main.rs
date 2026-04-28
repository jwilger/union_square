use std::{env, fs, path::PathBuf, process};

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
        .ok_or_else(|| "missing required --issue <number>".to_string())?;
    let ledger = PathBuf::from(format!(".codex/state/issue-{issue}.json"));
    let text = fs::read_to_string(&ledger)
        .map_err(|error| format!("failed to read {}: {error}", ledger.display()))?;

    if !(text.contains("\"green_observed\"")
        || text.contains("\"test_adversary_passed\"")
        || text.contains("\"fitness_passed\"")
        || text.contains("\"refactor_reviewed\"")
        || text.contains("\"expert_review_done\"")
        || text.contains("\"commit_ready\"")
        || text.contains("\"pr_ready\""))
    {
        return Err(format!(
            "{} must be at least green_observed before adversary checks run",
            ledger.display()
        ));
    }

    println!("targeted test adversary scaffold passed for issue {issue}");
    println!("red reversion and mutation execution are intentionally scoped for follow-up implementation");
    Ok(())
}

fn flag_value(args: &[String], flag: &str) -> Option<String> {
    args.windows(2)
        .find(|pair| pair[0] == flag)
        .map(|pair| pair[1].clone())
}
