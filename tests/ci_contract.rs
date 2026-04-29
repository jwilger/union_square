use std::fs;

#[test]
fn complete_ci_contract_includes_actions_security_linting() {
    let justfile = fs::read_to_string("Justfile").expect("Justfile should be readable");
    let ci_workflow =
        fs::read_to_string(".github/workflows/ci.yml").expect("CI workflow should be readable");

    let ci_full = just_target_dependencies(&justfile, "ci-full");
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

    let ci_rust = just_target_dependencies(&justfile, "ci-rust");
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

    let ci_security = just_target_dependencies(&justfile, "ci-security");
    for expected_target in ["audit", "deny", "actions-security"] {
        assert!(
            ci_security.contains(&expected_target),
            "ci-security should include {expected_target}"
        );
    }

    assert!(
        justfile.contains("actionlint") && justfile.contains("zizmor --min-severity high ."),
        "actions-security should run actionlint and high-severity zizmor"
    );
    assert!(
        ci_workflow.contains("run: just ci-security"),
        "CI security job should invoke the shared ci-security target"
    );
    assert!(
        !ci_workflow.contains("run: just audit"),
        "CI security job should not call audit directly and skip the rest of the shared security contract"
    );
}

fn just_target_dependencies<'a>(justfile: &'a str, target: &str) -> Vec<&'a str> {
    let prefix = format!("{target}:");
    justfile
        .lines()
        .find(|line| line.starts_with(&prefix))
        .unwrap_or_else(|| panic!("Justfile should define {target}"))
        .split_once(':')
        .expect("target line should contain a colon")
        .1
        .split_whitespace()
        .collect()
}
