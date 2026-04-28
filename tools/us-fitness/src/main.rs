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
    let repo = flag_value(&args, "--repo").unwrap_or_else(|| ".".to_string());
    let repo_path = Path::new(&repo);

    let changed = changed_files()?;
    let mut findings = Vec::new();
    for file in changed {
        if !file.starts_with("src/domain/") || !file.ends_with(".rs") {
            continue;
        }
        let path = repo_path.join(&file);
        let text = fs::read_to_string(path).map_err(|error| {
            format!(
                "failed to read {}: {error}",
                repo_path.join(&file).display()
            )
        })?;
        findings.extend(domain_dependency_findings(&file, &text));
    }

    if findings.is_empty() {
        println!("architecture fitness passed");
        Ok(())
    } else {
        Err(findings.join("\n"))
    }
}

fn domain_dependency_findings(file: &str, text: &str) -> Vec<String> {
    let mut findings = Vec::new();
    for pattern in FORBIDDEN_DOMAIN_PATTERNS {
        if text.contains(pattern) {
            findings.push(format!(
                "{file}: forbidden domain dependency pattern `{pattern}`"
            ));
        }
    }
    findings
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

fn flag_value(args: &[String], flag: &str) -> Option<String> {
    args.windows(2)
        .find(|pair| pair[0] == flag)
        .map(|pair| pair[1].clone())
}

#[cfg(test)]
mod tests {
    use super::domain_dependency_findings;

    #[test]
    fn clean_domain_fixture_passes() {
        let text = include_str!("../fixtures/clean-domain.rs");

        assert!(domain_dependency_findings("src/domain/clean.rs", text).is_empty());
    }

    #[test]
    fn violation_fixture_reports_forbidden_dependency() {
        let text = include_str!("../fixtures/violating-domain.rs");
        let findings = domain_dependency_findings("src/domain/violating.rs", text);

        assert!(findings
            .iter()
            .any(|finding| finding.contains("forbidden domain dependency pattern `use axum`")));
    }
}
