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

    let changed = changed_files(repo_path)?;
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
        if contains_forbidden_pattern(text, pattern) {
            findings.push(format!(
                "{file}: forbidden domain dependency pattern `{pattern}`"
            ));
        }
    }
    findings
}

fn contains_forbidden_pattern(text: &str, pattern: &str) -> bool {
    text.lines().any(|line| {
        if let Some(import) = pattern.strip_prefix("use ") {
            normalized_use_path(line).is_some_and(|path| import_matches(&path, import))
        } else if let Some(module) = pattern.strip_prefix("crate::") {
            contains_crate_module(line, module)
        } else {
            line.contains(pattern)
        }
    })
}

fn normalized_use_path(line: &str) -> Option<String> {
    let mut line = line.trim();
    if let Some(rest) = line.strip_prefix("pub ") {
        line = rest.trim_start();
    } else if line.starts_with("pub(") {
        let (_visibility, rest) = line.split_once(") ")?;
        line = rest.trim_start();
    }
    let import = line
        .strip_prefix("use ")?
        .trim()
        .trim_end_matches(';')
        .trim();
    let import = import
        .split_once(" as ")
        .map_or(import, |(path, _alias)| path)
        .trim();
    Some(import.to_string())
}

fn import_matches(path: &str, forbidden: &str) -> bool {
    path == forbidden
        || path.starts_with(&format!("{forbidden}::"))
        || path == format!("{forbidden}::*")
}

fn contains_crate_module(line: &str, module: &str) -> bool {
    if normalized_use_path(line)
        .is_some_and(|path| import_matches(&path, &format!("crate::{module}")))
    {
        return true;
    }

    for marker in [
        format!("crate::{module}::"),
        format!("crate::{module}("),
        format!("crate::{module};"),
        format!("crate::{module},"),
        format!("crate::{{{module}"),
        format!("crate::{{{module},"),
        format!("crate::{{{module} "),
        format!("crate::{{{module}::"),
        format!("crate::{{{module} as "),
        format!("crate::{{{module}}}"),
    ] {
        if line.contains(&marker) {
            return true;
        }
    }
    false
}

fn changed_files(repo_path: &Path) -> Result<Vec<String>, String> {
    if let Ok(files) = env::var("US_FITNESS_CHANGED_FILES") {
        return Ok(unique_non_empty_lines(&files));
    }

    if let Ok(base_ref) = env::var("US_FITNESS_BASE_REF")
        .or_else(|_| env::var("GITHUB_BASE_REF").map(|base_ref| format!("origin/{base_ref}")))
    {
        let compare_ref = format!("{base_ref}...HEAD");
        return git_changed_files(
            repo_path,
            &["diff", "--name-only", "--diff-filter=ACMR", &compare_ref],
        );
    }

    let mut files = Vec::new();
    for args in [
        &["diff", "--name-only", "--diff-filter=ACMR"][..],
        &["diff", "--cached", "--name-only", "--diff-filter=ACMR"][..],
    ] {
        files.extend(git_changed_files(repo_path, args)?);
        files.sort();
        files.dedup();
    }
    Ok(files)
}

fn git_changed_files(repo_path: &Path, args: &[&str]) -> Result<Vec<String>, String> {
    let output = Command::new("git")
        .current_dir(repo_path)
        .args(args)
        .output()
        .map_err(|error| format!("failed to run git diff: {error}"))?;
    if !output.status.success() {
        return Err("git diff failed while collecting changed files".to_string());
    }
    let stdout = String::from_utf8(output.stdout).map_err(|error| error.to_string())?;
    Ok(unique_non_empty_lines(&stdout))
}

fn unique_non_empty_lines(text: &str) -> Vec<String> {
    let mut files = Vec::new();
    for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
        if !files.iter().any(|file| file == line) {
            files.push(line.to_string());
        }
    }
    files
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

    #[test]
    fn dependency_patterns_do_not_match_prefix_collisions() {
        let text = "use http_body::Body;\nfn f() { crate::proxy_utils::x(); }";

        assert!(domain_dependency_findings("src/domain/clean.rs", text).is_empty());
    }

    #[test]
    fn dependency_patterns_match_common_rust_use_forms() {
        for text in [
            "pub use axum::Router;",
            "use axum as web;",
            "use axum::prelude::*;",
            "use crate::proxy as p;",
        ] {
            assert!(
                !domain_dependency_findings("src/domain/violating.rs", text).is_empty(),
                "expected `{text}` to be rejected"
            );
        }
    }
}
