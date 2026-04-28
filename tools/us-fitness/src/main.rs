use std::{env, fs, path::Path, process::Command};

const FORBIDDEN_DOMAIN_PATTERNS: &[&str] = &[
    "use axum",
    "use tower",
    "use sqlx",
    "use tokio",
    "use http",
    "use hyper",
    "use std::env",
    "crate::proxy",
    "crate::providers",
    "crate::infrastructure",
];

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.first().map(String::as_str) != Some("check") {
        return Err("usage: us-fitness check --repo <path>".to_string());
    }

    let changed = changed_files()?;
    let mut findings = Vec::new();
    for file in changed {
        if !file.starts_with("src/domain/") || !file.ends_with(".rs") {
            continue;
        }
        let path = Path::new(&file);
        let text = fs::read_to_string(path)
            .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
        for pattern in FORBIDDEN_DOMAIN_PATTERNS {
            if text.contains(pattern) {
                findings.push(format!(
                    "{file}: forbidden domain dependency pattern `{pattern}`"
                ));
            }
        }
    }

    if findings.is_empty() {
        println!("architecture fitness passed");
        Ok(())
    } else {
        Err(findings.join("\n"))
    }
}

fn changed_files() -> Result<Vec<String>, String> {
    let mut files = Vec::new();
    for args in [
        &["diff", "--name-only", "--diff-filter=ACMR"][..],
        &["diff", "--cached", "--name-only", "--diff-filter=ACMR"][..],
    ] {
        let output = Command::new("git")
            .args(args)
            .output()
            .map_err(|error| format!("failed to run git diff: {error}"))?;
        if !output.status.success() {
            return Err("git diff failed while collecting changed files".to_string());
        }
        let stdout = String::from_utf8(output.stdout).map_err(|error| error.to_string())?;
        for line in stdout.lines() {
            if !line.trim().is_empty() && !files.iter().any(|file| file == line) {
                files.push(line.to_string());
            }
        }
    }
    Ok(files)
}
