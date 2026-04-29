use std::fs;

#[test]
fn complete_ci_contract_includes_actions_security_linting() -> Result<(), String> {
    let justfile = fs::read_to_string("Justfile")
        .map_err(|error| format!("Justfile should be readable: {error}"))?;
    let ci_workflow = fs::read_to_string(".github/workflows/ci.yml")
        .map_err(|error| format!("CI workflow should be readable: {error}"))?;

    let ci_full = just_target_dependencies(&justfile, "ci-full")?;
    for expected_target in [
        "ci-rust",
        "ci-security",
        "build",
        "build-release",
        "bench-quick",
    ] {
        assert!(
            ci_full.contains(&expected_target),
            "ci-full should include {expected_target}"
        );
    }

    let ci_rust = just_target_dependencies(&justfile, "ci-rust")?;
    for expected_target in [
        "fmt-check",
        "clippy",
        "clippy-tools",
        "check",
        "check-tools",
        "test-tools",
        "test-hooks",
        "test",
        "test-doc",
        "ast-grep-branch",
        "ast-grep-test",
        "fitness",
    ] {
        assert!(
            ci_rust.contains(&expected_target),
            "ci-rust should include {expected_target}"
        );
    }

    let ci_security = just_target_dependencies(&justfile, "ci-security")?;
    for expected_target in ["audit", "deny", "actions-security"] {
        assert!(
            ci_security.contains(&expected_target),
            "ci-security should include {expected_target}"
        );
    }

    let actions_security = just_target_body(&justfile, "actions-security")?;
    let security_job = workflow_job_body(&ci_workflow, "security")?;
    let test_job = workflow_job_body(&ci_workflow, "test")?;

    assert!(
        actions_security.contains("actionlint")
            && actions_security.contains("zizmor --min-severity high ."),
        "actions-security should run actionlint and high-severity zizmor"
    );
    assert!(
        security_job.contains("run: just ci-security"),
        "CI security job should invoke the shared ci-security target"
    );
    assert!(
        test_job.contains("run: just ci-rust"),
        "CI test job should invoke the shared ci-rust target"
    );
    assert!(
        !security_job.contains("run: just audit"),
        "CI security job should not call audit directly and skip the rest of the shared security contract"
    );

    Ok(())
}

fn just_target_dependencies<'a>(justfile: &'a str, target: &str) -> Result<Vec<&'a str>, String> {
    let prefix = format!("{target}:");
    let target_line = justfile
        .lines()
        .find(|line| line.starts_with(&prefix))
        .ok_or_else(|| format!("Justfile should define {target}"))?;

    Ok(target_line
        .split_once(':')
        .ok_or_else(|| format!("{target} line should contain a colon"))?
        .1
        .split_whitespace()
        .collect())
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

fn workflow_job_body<'a>(workflow: &'a str, job: &str) -> Result<&'a str, String> {
    let header = format!("  {job}:");
    block_from_line(
        workflow,
        |line| line == header,
        is_workflow_job_header,
        || format!("CI workflow should define the {job} job"),
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

fn is_workflow_job_header(line: &str) -> bool {
    line.starts_with("  ") && !line.starts_with("    ") && line.ends_with(':')
}
