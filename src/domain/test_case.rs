//! Test case and test run domain entities
//!
//! This module defines entities for managing test cases and their execution,
//! following the type-state pattern to ensure valid state transitions.

use crate::domain::types::{
    AssertionDescription, ErrorMessage, MetadataAssertions, Pattern, PromptTemplate, ResponseText,
    TestCaseDescription,
};
use chrono::{DateTime, Utc};
use nutype::nutype;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use uuid::Uuid;

/// Placeholder prompt template used in draft state before actual prompt is set
const PLACEHOLDER_PROMPT_TEMPLATE: &str = "PLACEHOLDER";

/// Unique identifier for a test case
#[nutype(derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, AsRef))]
pub struct TestCaseId(Uuid);

impl TestCaseId {
    pub fn generate() -> Self {
        Self::new(Uuid::now_v7())
    }
}

impl Default for TestCaseId {
    fn default() -> Self {
        Self::generate()
    }
}

/// Unique identifier for a test run
#[nutype(derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, AsRef))]
pub struct TestRunId(Uuid);

impl TestRunId {
    pub fn generate() -> Self {
        Self::new(Uuid::now_v7())
    }
}

impl Default for TestRunId {
    fn default() -> Self {
        Self::generate()
    }
}

/// Test case name with validation
#[nutype(
    validate(not_empty, len_char_max = 200),
    derive(
        Debug,
        Clone,
        PartialEq,
        Eq,
        Hash,
        Serialize,
        Deserialize,
        AsRef,
        Display
    )
)]
pub struct TestCaseName(String);

/// Test states for type-state pattern
#[derive(Clone)]
pub struct Draft;
#[derive(Clone)]
pub struct Ready;
#[derive(Clone)]
pub struct Running;
#[derive(Clone)]
pub struct Completed;

/// Test case represents a reusable test scenario
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TestCase<State> {
    pub id: TestCaseId,
    pub name: TestCaseName,
    pub description: TestCaseDescription,
    pub expected_behavior: ExpectedBehavior,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(skip)]
    _state: PhantomData<State>,
}

/// Expected behavior for a test case
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExpectedBehavior {
    pub prompt_template: PromptTemplate,
    pub expected_patterns: Vec<Pattern>,
    pub forbidden_patterns: Vec<Pattern>,
    pub metadata_assertions: MetadataAssertions,
}

/// Validation error for test cases
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ValidationError {
    #[error("Test case name cannot be empty")]
    EmptyName,
    #[error("Prompt template cannot be empty")]
    EmptyPromptTemplate,
    #[error("At least one expected pattern is required")]
    NoExpectedPatterns,
}

impl TestCase<Draft> {
    /// Create a new test case in draft state
    pub fn new(name: TestCaseName, description: TestCaseDescription) -> Self {
        let now = Utc::now();
        Self {
            id: TestCaseId::generate(),
            name,
            description,
            expected_behavior: ExpectedBehavior {
                // Use a placeholder prompt template - will be replaced before finalize
                prompt_template: PromptTemplate::try_new(PLACEHOLDER_PROMPT_TEMPLATE.to_string())
                    .unwrap(),
                expected_patterns: Vec::new(),
                forbidden_patterns: Vec::new(),
                metadata_assertions: MetadataAssertions::new(serde_json::Value::Object(
                    serde_json::Map::new(),
                )),
            },
            created_at: now,
            updated_at: now,
            _state: PhantomData,
        }
    }

    /// Update the expected behavior
    pub fn with_expected_behavior(mut self, behavior: ExpectedBehavior) -> Self {
        self.expected_behavior = behavior;
        self.updated_at = Utc::now();
        self
    }

    /// Finalize the test case, moving it to Ready state
    pub fn finalize(self) -> Result<TestCase<Ready>, ValidationError> {
        // Validate the test case - check if still placeholder
        if self.expected_behavior.prompt_template.as_ref() == PLACEHOLDER_PROMPT_TEMPLATE {
            return Err(ValidationError::EmptyPromptTemplate);
        }
        if self.expected_behavior.expected_patterns.is_empty() {
            return Err(ValidationError::NoExpectedPatterns);
        }

        Ok(TestCase {
            id: self.id,
            name: self.name,
            description: self.description,
            expected_behavior: self.expected_behavior,
            created_at: self.created_at,
            updated_at: Utc::now(),
            _state: PhantomData,
        })
    }
}

impl TestCase<Ready> {
    /// Execute the test case, moving it to Running state
    pub fn execute(self) -> TestCase<Running> {
        TestCase {
            id: self.id,
            name: self.name,
            description: self.description,
            expected_behavior: self.expected_behavior,
            created_at: self.created_at,
            updated_at: Utc::now(),
            _state: PhantomData,
        }
    }
}

impl TestCase<Running> {
    /// Complete the test case execution
    pub fn complete(self, result: TestResult) -> (TestCase<Completed>, TestRun) {
        let test_case = TestCase {
            id: self.id,
            name: self.name,
            description: self.description,
            expected_behavior: self.expected_behavior,
            created_at: self.created_at,
            updated_at: Utc::now(),
            _state: PhantomData,
        };

        let test_run = TestRun {
            id: TestRunId::generate(),
            test_case_id: test_case.id.clone(),
            session_id: result.session_id,
            started_at: result.started_at,
            completed_at: Utc::now(),
            status: result.status,
            actual_response: result.actual_response,
            assertions_passed: result.assertions_passed,
            assertions_failed: result.assertions_failed,
            error_message: result.error_message,
        };

        (test_case, test_run)
    }
}

/// Result of executing a test case
#[derive(Debug, Clone, PartialEq)]
pub struct TestResult {
    pub session_id: crate::domain::SessionId,
    pub started_at: DateTime<Utc>,
    pub status: TestRunStatus,
    pub actual_response: ResponseText,
    pub assertions_passed: Vec<AssertionDescription>,
    pub assertions_failed: Vec<AssertionDescription>,
    pub error_message: Option<ErrorMessage>,
}

/// Test run represents a single execution of a test case
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TestRun {
    pub id: TestRunId,
    pub test_case_id: TestCaseId,
    pub session_id: crate::domain::SessionId,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
    pub status: TestRunStatus,
    pub actual_response: ResponseText,
    pub assertions_passed: Vec<AssertionDescription>,
    pub assertions_failed: Vec<AssertionDescription>,
    pub error_message: Option<ErrorMessage>,
}

/// Status of a test run
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TestRunStatus {
    Passed,
    Failed,
    Error,
    Skipped,
}

impl TestRun {
    /// Check if the test run passed
    pub fn is_passed(&self) -> bool {
        matches!(self.status, TestRunStatus::Passed)
    }

    /// Get the duration of the test run
    pub fn duration(&self) -> chrono::Duration {
        self.completed_at - self.started_at
    }

    /// Get a summary of the test run
    pub fn summary(&self) -> String {
        format!(
            "Test run {} for test case {}: {} ({} passed, {} failed) in {}ms",
            self.id.as_ref(),
            self.test_case_id.as_ref(),
            match self.status {
                TestRunStatus::Passed => "PASSED",
                TestRunStatus::Failed => "FAILED",
                TestRunStatus::Error => "ERROR",
                TestRunStatus::Skipped => "SKIPPED",
            },
            self.assertions_passed.len(),
            self.assertions_failed.len(),
            self.duration().num_milliseconds()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_test_case_id_generation() {
        let id1 = TestCaseId::generate();
        let id2 = TestCaseId::generate();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_test_run_id_generation() {
        let id1 = TestRunId::generate();
        let id2 = TestRunId::generate();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_test_case_name_validation() {
        assert!(TestCaseName::try_new("Valid Test Name".to_string()).is_ok());
        assert!(TestCaseName::try_new("".to_string()).is_err());
        assert!(TestCaseName::try_new("a".repeat(201)).is_err());
    }

    #[test]
    fn test_test_case_state_transitions() {
        // Create draft
        let name = TestCaseName::try_new("Test LLM Response".to_string()).unwrap();
        let description = TestCaseDescription::try_new("Test description".to_string()).unwrap();
        let draft = TestCase::<Draft>::new(name, description);

        // Update expected behavior
        let behavior = ExpectedBehavior {
            prompt_template: PromptTemplate::try_new("Hello, {name}!".to_string()).unwrap(),
            expected_patterns: vec![Pattern::try_new("greeting".to_string()).unwrap()],
            forbidden_patterns: vec![Pattern::try_new("error".to_string()).unwrap()],
            metadata_assertions: MetadataAssertions::new(serde_json::json!({"min_tokens": 10})),
        };
        let draft = draft.with_expected_behavior(behavior);

        // Finalize to ready
        let ready = draft.finalize().unwrap();
        assert_eq!(ready.name.as_ref(), "Test LLM Response");

        // Execute
        let running = ready.execute();

        // Complete
        let result = TestResult {
            session_id: crate::domain::SessionId::generate(),
            started_at: Utc::now(),
            status: TestRunStatus::Passed,
            actual_response: ResponseText::try_new("Hello! Nice to meet you.".to_string()).unwrap(),
            assertions_passed: vec![
                AssertionDescription::try_new("Contains greeting".to_string()).unwrap(),
            ],
            assertions_failed: vec![],
            error_message: None,
        };
        let (_completed, test_run) = running.complete(result);

        assert_eq!(test_run.status, TestRunStatus::Passed);
        assert!(test_run.is_passed());
        assert_eq!(test_run.assertions_passed.len(), 1);
        assert_eq!(test_run.assertions_failed.len(), 0);
    }

    #[test]
    fn test_validation_errors() {
        let name = TestCaseName::try_new("Test".to_string()).unwrap();
        let description = TestCaseDescription::try_new("Description".to_string()).unwrap();
        let draft = TestCase::<Draft>::new(name, description);

        // Empty prompt template
        let result = draft.clone().finalize();
        assert!(matches!(result, Err(ValidationError::EmptyPromptTemplate)));

        // No expected patterns
        let behavior = ExpectedBehavior {
            prompt_template: PromptTemplate::try_new("Hello".to_string()).unwrap(),
            expected_patterns: vec![],
            forbidden_patterns: vec![],
            metadata_assertions: MetadataAssertions::new(serde_json::Value::Null),
        };
        let draft = draft.with_expected_behavior(behavior);
        let result = draft.finalize();
        assert!(matches!(result, Err(ValidationError::NoExpectedPatterns)));
    }

    #[test]
    fn test_test_run_duration() {
        let started = Utc::now();
        let completed = started + chrono::Duration::seconds(2);

        let test_run = TestRun {
            id: TestRunId::generate(),
            test_case_id: TestCaseId::generate(),
            session_id: crate::domain::SessionId::generate(),
            started_at: started,
            completed_at: completed,
            status: TestRunStatus::Passed,
            actual_response: ResponseText::try_new("Response".to_string()).unwrap(),
            assertions_passed: vec![],
            assertions_failed: vec![],
            error_message: None,
        };

        assert_eq!(test_run.duration().num_seconds(), 2);
    }

    // Property-based tests
    proptest! {
        #[test]
        fn prop_test_case_id_uniqueness(n in 1..100usize) {
            let mut ids = std::collections::HashSet::new();
            for _ in 0..n {
                let id = TestCaseId::generate();
                assert!(ids.insert(id));
            }
        }

        #[test]
        fn prop_test_case_name_roundtrip(s in ".{1,200}") {
            if let Ok(name) = TestCaseName::try_new(s.clone()) {
                assert_eq!(name.as_ref(), &s);

                let json = serde_json::to_string(&name).unwrap();
                let deserialized: TestCaseName = serde_json::from_str(&json).unwrap();
                assert_eq!(name, deserialized);
            }
        }

        #[test]
        fn prop_test_run_serialization(
            response in ".{0,10000}",
            passed_count in 0..10usize,
            failed_count in 0..10usize,
            error_msg in prop::option::of(".{1,1000}"),
            status_choice in 0..4u8
        ) {
            let status = match status_choice {
                0 => TestRunStatus::Passed,
                1 => TestRunStatus::Failed,
                2 => TestRunStatus::Error,
                _ => TestRunStatus::Skipped,
            };

            let test_run = TestRun {
                id: TestRunId::generate(),
                test_case_id: TestCaseId::generate(),
                session_id: crate::domain::SessionId::generate(),
                started_at: Utc::now(),
                completed_at: Utc::now(),
                status,
                actual_response: ResponseText::try_new(response).unwrap(),
                assertions_passed: (0..passed_count).map(|i| AssertionDescription::try_new(format!("Passed {i}")).unwrap()).collect(),
                assertions_failed: (0..failed_count).map(|i| AssertionDescription::try_new(format!("Failed {i}")).unwrap()).collect(),
                error_message: error_msg.and_then(|s| ErrorMessage::try_new(s).ok()),
            };

            let json = serde_json::to_string(&test_run).unwrap();
            let deserialized: TestRun = serde_json::from_str(&json).unwrap();
            assert_eq!(test_run, deserialized);
        }

        #[test]
        fn prop_expected_behavior_validation(
            prompt in ".{0,1000}",
            expected_count in 0..10usize,
            forbidden_count in 0..10usize
        ) {
            let name = TestCaseName::try_new("Test".to_string()).unwrap();
            let description = TestCaseDescription::try_new("Description".to_string()).unwrap();
            let draft = TestCase::<Draft>::new(name, description);

            let prompt_template = if prompt.is_empty() {
                // Keep placeholder for empty prompts
                PromptTemplate::try_new(PLACEHOLDER_PROMPT_TEMPLATE.to_string()).unwrap()
            } else {
                PromptTemplate::try_new(prompt.clone()).unwrap()
            };

            let behavior = ExpectedBehavior {
                prompt_template,
                expected_patterns: (0..expected_count).map(|i| Pattern::try_new(format!("Pattern {i}")).unwrap()).collect(),
                forbidden_patterns: (0..forbidden_count).map(|i| Pattern::try_new(format!("Forbidden {i}")).unwrap()).collect(),
                metadata_assertions: MetadataAssertions::new(serde_json::json!({})),
            };

            let draft = draft.with_expected_behavior(behavior);
            let result = draft.finalize();

            if prompt.is_empty() || prompt == PLACEHOLDER_PROMPT_TEMPLATE {
                assert!(matches!(result, Err(ValidationError::EmptyPromptTemplate)));
            } else if expected_count == 0 {
                assert!(matches!(result, Err(ValidationError::NoExpectedPatterns)));
            } else {
                assert!(result.is_ok());
            }
        }
    }
}
