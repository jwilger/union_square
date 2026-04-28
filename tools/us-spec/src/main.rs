use serde::Deserialize;
use std::{collections::BTreeSet, env, fs, path::PathBuf, process};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BehaviorSpec {
    issue: u64,
    goal: String,
    examples: Vec<Example>,
    acceptance_criteria: Vec<String>,
    non_goals: Vec<String>,
    architecture_impacts: Vec<String>,
    test_trace_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Example {
    id: String,
    name: String,
    given: Vec<String>,
    when: Vec<String>,
    then: Vec<String>,
}

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
        .ok_or_else(|| "missing required --issue <number>".to_string())?
        .parse::<u64>()
        .map_err(|error| format!("invalid issue number: {error}"))?;
    let path = PathBuf::from(format!(".codex/specs/issue-{issue}.yaml"));
    let text = fs::read_to_string(&path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;

    validate_spec_text(&text, issue)?;
    println!("behavior spec is valid: {}", path.display());
    Ok(())
}

fn validate_spec_text(text: &str, expected_issue: u64) -> Result<(), String> {
    let spec: BehaviorSpec =
        serde_yaml::from_str(text).map_err(|error| format!("invalid YAML spec: {error}"))?;
    spec.validate(expected_issue)
}

impl BehaviorSpec {
    fn validate(&self, expected_issue: u64) -> Result<(), String> {
        let mut errors = Vec::new();

        if self.issue != expected_issue {
            errors.push(format!(
                "issue field {} does not match expected issue {}",
                self.issue, expected_issue
            ));
        }
        require_non_empty("goal", &self.goal, &mut errors);
        require_non_empty_list("examples", &self.examples, &mut errors);
        require_non_blank_list(
            "acceptance_criteria",
            &self.acceptance_criteria,
            &mut errors,
        );
        require_non_blank_list("non_goals", &self.non_goals, &mut errors);
        require_non_blank_list(
            "architecture_impacts",
            &self.architecture_impacts,
            &mut errors,
        );
        require_non_blank_list("test_trace_ids", &self.test_trace_ids, &mut errors);

        let mut example_ids = BTreeSet::new();
        for example in &self.examples {
            require_non_empty("examples[].id", &example.id, &mut errors);
            require_non_empty("examples[].name", &example.name, &mut errors);
            require_non_blank_list("examples[].given", &example.given, &mut errors);
            require_non_blank_list("examples[].when", &example.when, &mut errors);
            require_non_blank_list("examples[].then", &example.then, &mut errors);
            if !example.id.trim().is_empty() && !example_ids.insert(example.id.as_str()) {
                errors.push(format!("duplicate example id `{}`", example.id));
            }
        }

        for trace in &self.test_trace_ids {
            let (example_id, test_path) = match trace.split_once(':') {
                Some(parts) => parts,
                None => {
                    errors.push(format!(
                        "test_trace_ids entry `{trace}` must use example-id:test-path format"
                    ));
                    continue;
                }
            };
            if !example_ids.contains(example_id.trim()) {
                errors.push(format!(
                    "test_trace_ids entry `{trace}` references unknown example id"
                ));
            }
            if test_path.trim().is_empty() {
                errors.push(format!(
                    "test_trace_ids entry `{trace}` has empty test path"
                ));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join("\n"))
        }
    }
}

fn require_non_empty(field: &str, value: &str, errors: &mut Vec<String>) {
    if value.trim().is_empty() {
        errors.push(format!("{field} must not be empty"));
    }
}

fn require_non_empty_list<T>(field: &str, value: &[T], errors: &mut Vec<String>) {
    if value.is_empty() {
        errors.push(format!("{field} must not be empty"));
    }
}

fn require_non_blank_list(field: &str, value: &[String], errors: &mut Vec<String>) {
    if value.is_empty() {
        errors.push(format!("{field} must not be empty"));
        return;
    }
    for (index, entry) in value.iter().enumerate() {
        if entry.trim().is_empty() {
            errors.push(format!("{field}[{index}] must not be blank"));
        }
    }
}

fn flag_value(args: &[String], flag: &str) -> Option<String> {
    args.windows(2)
        .find(|pair| pair[0] == flag)
        .map(|pair| pair[1].clone())
}

#[cfg(test)]
mod tests {
    use super::validate_spec_text;

    #[test]
    fn valid_spec_fixture_passes_schema_and_traceability() {
        let fixture = include_str!("../fixtures/valid-spec.yaml");

        validate_spec_text(fixture, 215).expect("valid fixture should pass");
    }

    #[test]
    fn invalid_spec_fixture_reports_schema_and_traceability_errors() {
        let fixture = include_str!("../fixtures/invalid-spec.yaml");
        let error = validate_spec_text(fixture, 215).expect_err("invalid fixture should fail");

        assert!(error.contains("goal must not be empty"));
        assert!(error.contains("examples[].then must not be empty"));
        assert!(error.contains("references unknown example id"));
    }

    #[test]
    fn blank_list_entries_are_rejected() {
        let fixture = include_str!("../fixtures/blank-list-entry.yaml");
        let error = validate_spec_text(fixture, 215).expect_err("blank list entry should fail");

        assert!(error.contains("acceptance_criteria[0] must not be blank"));
        assert!(error.contains("examples[].given[0] must not be blank"));
    }
}
