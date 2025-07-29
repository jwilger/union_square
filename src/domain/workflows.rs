//! Type-safe domain workflows and state machines
//!
//! This module defines state machines that ensure illegal state transitions
//! are impossible at compile time, encoding business rules in the type system.

use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

use crate::domain::{
    events::DomainEvent,
    llm::{ModelVersion, RequestId, ResponseMetadata},
    metrics::Timestamp,
    session::{SessionId, SessionStatus},
    streams::{AnalysisId, ExtractionId},
    test_case::TestCaseId,
    types::{ErrorMessage, LlmParameters, Prompt, ResponseText},
};

/// Session workflow state machine
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionWorkflow {
    /// Session not yet started
    NotStarted,
    /// Session is active
    Active {
        session_id: SessionId,
        started_at: Timestamp,
        request_count: u32,
    },
    /// Session completed successfully
    Completed {
        session_id: SessionId,
        started_at: Timestamp,
        completed_at: Timestamp,
        request_count: u32,
    },
    /// Session failed
    Failed {
        session_id: SessionId,
        started_at: Timestamp,
        failed_at: Timestamp,
        error: ErrorMessage,
    },
}

impl SessionWorkflow {
    /// Start a new session
    pub fn start(session_id: SessionId, timestamp: Timestamp) -> Result<Self, WorkflowError> {
        Ok(Self::Active {
            session_id,
            started_at: timestamp,
            request_count: 0,
        })
    }

    /// Record a request in the session
    pub fn record_request(self) -> Result<Self, WorkflowError> {
        match self {
            Self::Active {
                session_id,
                started_at,
                request_count,
            } => Ok(Self::Active {
                session_id,
                started_at,
                request_count: request_count + 1,
            }),
            _ => Err(WorkflowError::InvalidTransition {
                from: self.state_name(),
                to: "Active with incremented request",
                reason: "Can only record requests in active sessions",
            }),
        }
    }

    /// Complete the session
    pub fn complete(self, timestamp: Timestamp) -> Result<Self, WorkflowError> {
        match self {
            Self::Active {
                session_id,
                started_at,
                request_count,
            } => Ok(Self::Completed {
                session_id,
                started_at,
                completed_at: timestamp,
                request_count,
            }),
            _ => Err(WorkflowError::InvalidTransition {
                from: self.state_name(),
                to: "Completed",
                reason: "Can only complete active sessions",
            }),
        }
    }

    /// Fail the session
    pub fn fail(self, timestamp: Timestamp, error: ErrorMessage) -> Result<Self, WorkflowError> {
        match self {
            Self::Active {
                session_id,
                started_at,
                ..
            } => Ok(Self::Failed {
                session_id,
                started_at,
                failed_at: timestamp,
                error,
            }),
            _ => Err(WorkflowError::InvalidTransition {
                from: self.state_name(),
                to: "Failed",
                reason: "Can only fail active sessions",
            }),
        }
    }

    fn state_name(&self) -> &'static str {
        match self {
            Self::NotStarted => "NotStarted",
            Self::Active { .. } => "Active",
            Self::Completed { .. } => "Completed",
            Self::Failed { .. } => "Failed",
        }
    }
}

/// Analysis workflow state machine
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnalysisWorkflow<State> {
    /// Analysis not yet started
    NotStarted(PhantomData<State>),
    /// Analysis configuration phase
    Configuring {
        analysis_id: AnalysisId,
        session_id: SessionId,
        _state: PhantomData<State>,
    },
    /// Analysis in progress
    InProgress {
        analysis_id: AnalysisId,
        session_id: SessionId,
        started_at: Timestamp,
        _state: PhantomData<State>,
    },
    /// Analysis completed
    Completed {
        analysis_id: AnalysisId,
        session_id: SessionId,
        started_at: Timestamp,
        completed_at: Timestamp,
        results: AnalysisResults,
        _state: PhantomData<State>,
    },
}

// Phantom types for compile-time state tracking
pub struct Unconfigured;
pub struct Configured;
pub struct Analyzed;

/// Analysis results
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalysisResults {
    pub pattern_matches: Vec<PatternMatch>,
    pub metrics: AnalysisMetrics,
    pub recommendations: Vec<Recommendation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatternMatch {
    pub pattern_id: String,
    pub confidence: f32,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalysisMetrics {
    pub total_requests: u32,
    pub successful_requests: u32,
    pub average_response_time_ms: u64,
    pub error_rate: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recommendation {
    pub category: RecommendationCategory,
    pub description: String,
    pub priority: Priority,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecommendationCategory {
    Performance,
    CostOptimization,
    ModelSelection,
    PromptEngineering,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

impl AnalysisWorkflow<Unconfigured> {
    /// Create a new analysis
    pub fn new() -> Self {
        Self::NotStarted(PhantomData)
    }

    /// Start configuring the analysis
    pub fn configure(
        self,
        analysis_id: AnalysisId,
        session_id: SessionId,
    ) -> AnalysisWorkflow<Configured> {
        AnalysisWorkflow::Configuring {
            analysis_id,
            session_id,
            _state: PhantomData,
        }
    }
}

impl AnalysisWorkflow<Configured> {
    /// Start the analysis
    pub fn start(self, timestamp: Timestamp) -> Result<AnalysisWorkflow<Analyzed>, WorkflowError> {
        match self {
            AnalysisWorkflow::Configuring {
                analysis_id,
                session_id,
                ..
            } => Ok(AnalysisWorkflow::InProgress {
                analysis_id,
                session_id,
                started_at: timestamp,
                _state: PhantomData,
            }),
            _ => Err(WorkflowError::InvalidTransition {
                from: "Unknown",
                to: "InProgress",
                reason: "Can only start from Configuring state",
            }),
        }
    }
}

impl AnalysisWorkflow<Analyzed> {
    /// Complete the analysis with results
    pub fn complete(
        self,
        timestamp: Timestamp,
        results: AnalysisResults,
    ) -> Result<Self, WorkflowError> {
        match self {
            AnalysisWorkflow::InProgress {
                analysis_id,
                session_id,
                started_at,
                ..
            } => Ok(AnalysisWorkflow::Completed {
                analysis_id,
                session_id,
                started_at,
                completed_at: timestamp,
                results,
                _state: PhantomData,
            }),
            _ => Err(WorkflowError::InvalidTransition {
                from: "Unknown",
                to: "Completed",
                reason: "Can only complete from InProgress state",
            }),
        }
    }
}

/// Test extraction workflow with compile-time guarantees
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtractionWorkflow<Stage> {
    extraction_id: ExtractionId,
    session_id: SessionId,
    _stage: PhantomData<Stage>,
}

// Phantom types for extraction stages
pub struct SelectingInteractions;
pub struct ValidatingPatterns;
pub struct GeneratingTests;
pub struct ReviewingTests;
pub struct ExtractionComplete;

impl ExtractionWorkflow<SelectingInteractions> {
    /// Start a new extraction workflow
    pub fn new(extraction_id: ExtractionId, session_id: SessionId) -> Self {
        Self {
            extraction_id,
            session_id,
            _stage: PhantomData,
        }
    }

    /// Select interactions for test extraction
    pub fn select_interactions(
        self,
        interaction_ids: Vec<RequestId>,
    ) -> Result<ExtractionWorkflow<ValidatingPatterns>, WorkflowError> {
        if interaction_ids.is_empty() {
            return Err(WorkflowError::ValidationError(
                "Must select at least one interaction".to_string(),
            ));
        }
        Ok(ExtractionWorkflow {
            extraction_id: self.extraction_id,
            session_id: self.session_id,
            _stage: PhantomData,
        })
    }
}

impl ExtractionWorkflow<ValidatingPatterns> {
    /// Validate patterns and proceed to test generation
    pub fn validate_patterns(
        self,
        validation_results: ValidationResults,
    ) -> Result<ExtractionWorkflow<GeneratingTests>, WorkflowError> {
        if !validation_results.is_valid {
            return Err(WorkflowError::ValidationError(format!(
                "Pattern validation failed: {}",
                validation_results.errors.join(", ")
            )));
        }
        Ok(ExtractionWorkflow {
            extraction_id: self.extraction_id,
            session_id: self.session_id,
            _stage: PhantomData,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationResults {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl ExtractionWorkflow<GeneratingTests> {
    /// Generate test cases
    pub fn generate_tests(
        self,
        generated_tests: Vec<TestCaseId>,
    ) -> Result<ExtractionWorkflow<ReviewingTests>, WorkflowError> {
        if generated_tests.is_empty() {
            return Err(WorkflowError::ValidationError(
                "No tests were generated".to_string(),
            ));
        }
        Ok(ExtractionWorkflow {
            extraction_id: self.extraction_id,
            session_id: self.session_id,
            _stage: PhantomData,
        })
    }
}

impl ExtractionWorkflow<ReviewingTests> {
    /// Complete the extraction after review
    pub fn complete_extraction(
        self,
        approved_tests: Vec<TestCaseId>,
    ) -> Result<ExtractionWorkflow<ExtractionComplete>, WorkflowError> {
        Ok(ExtractionWorkflow {
            extraction_id: self.extraction_id,
            session_id: self.session_id,
            _stage: PhantomData,
        })
    }
}

/// Workflow errors
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum WorkflowError {
    #[error("Invalid state transition from {from} to {to}: {reason}")]
    InvalidTransition {
        from: &'static str,
        to: &'static str,
        reason: &'static str,
    },

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Precondition not met: {0}")]
    PreconditionFailed(String),
}

/// Event-driven state transitions
pub trait EventDrivenWorkflow {
    fn apply_event(&mut self, event: &DomainEvent) -> Result<(), WorkflowError>;
}

impl EventDrivenWorkflow for SessionWorkflow {
    fn apply_event(&mut self, event: &DomainEvent) -> Result<(), WorkflowError> {
        match event {
            DomainEvent::SessionStarted {
                session_id,
                started_at,
                ..
            } => {
                *self = Self::start(session_id.clone(), *started_at)?;
                Ok(())
            }
            DomainEvent::LlmRequestReceived { .. } => {
                *self = self.clone().record_request()?;
                Ok(())
            }
            DomainEvent::SessionEnded {
                ended_at,
                final_status,
                ..
            } => match final_status {
                SessionStatus::Completed => {
                    *self = self.clone().complete(*ended_at)?;
                    Ok(())
                }
                SessionStatus::Failed => {
                    let error = ErrorMessage::try_new("Session failed".to_string())
                        .expect("Valid error message");
                    *self = self.clone().fail(*ended_at, error)?;
                    Ok(())
                }
                _ => Ok(()),
            },
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_workflow_transitions() {
        let session_id = SessionId::generate();
        let timestamp = Timestamp::now();

        // Start session
        let workflow = SessionWorkflow::start(session_id.clone(), timestamp).unwrap();
        assert!(matches!(workflow, SessionWorkflow::Active { .. }));

        // Record requests
        let workflow = workflow.record_request().unwrap();
        let workflow = workflow.record_request().unwrap();
        match &workflow {
            SessionWorkflow::Active { request_count, .. } => assert_eq!(*request_count, 2),
            _ => panic!("Expected Active state"),
        }

        // Complete session
        let workflow = workflow.complete(timestamp).unwrap();
        assert!(matches!(workflow, SessionWorkflow::Completed { .. }));

        // Cannot record request after completion
        assert!(workflow.record_request().is_err());
    }

    #[test]
    fn test_analysis_workflow_type_safety() {
        let analysis_id = AnalysisId::generate();
        let session_id = SessionId::generate();
        let timestamp = Timestamp::now();

        // Create workflow
        let workflow: AnalysisWorkflow<Unconfigured> = AnalysisWorkflow::new();

        // Configure (changes type)
        let workflow: AnalysisWorkflow<Configured> =
            workflow.configure(analysis_id, session_id);

        // Start analysis (changes type)
        let workflow: AnalysisWorkflow<Analyzed> = workflow.start(timestamp).unwrap();

        // Complete with results
        let results = AnalysisResults {
            pattern_matches: vec![],
            metrics: AnalysisMetrics {
                total_requests: 10,
                successful_requests: 9,
                average_response_time_ms: 250,
                error_rate: 0.1,
            },
            recommendations: vec![],
        };

        let _completed = workflow.complete(timestamp, results).unwrap();
    }

    #[test]
    fn test_extraction_workflow_stages() {
        let extraction_id = ExtractionId::generate();
        let session_id = SessionId::generate();

        // Create workflow
        let workflow = ExtractionWorkflow::new(extraction_id, session_id);

        // Cannot proceed without selecting interactions
        assert!(workflow
            .clone()
            .select_interactions(vec![])
            .is_err());

        // Select interactions
        let request_id = RequestId::new();
        let workflow = workflow
            .select_interactions(vec![request_id])
            .unwrap();

        // Validate patterns
        let validation = ValidationResults {
            is_valid: true,
            errors: vec![],
            warnings: vec![],
        };
        let workflow = workflow.validate_patterns(validation).unwrap();

        // Generate tests
        let test_id = TestCaseId::generate();
        let workflow = workflow.generate_tests(vec![test_id.clone()]).unwrap();

        // Complete extraction
        let _completed = workflow.complete_extraction(vec![test_id]).unwrap();
    }
}