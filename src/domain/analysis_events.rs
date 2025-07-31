//! Analysis-specific domain events
//!
//! This module defines events specific to the analysis and test case extraction
//! functionality, extending the core domain events.

use crate::domain::commands_test::AnalysisReason;
use crate::domain::identifiers::{AnalysisId, ExtractionId};
use crate::domain::llm::RequestId;
use crate::domain::metrics::Timestamp;
use crate::domain::session::SessionId;
use crate::domain::test_case::{TestCaseId, TestCaseName};
use crate::domain::types::{ErrorMessage, ResponseText};
use serde::{Deserialize, Serialize};

/// Events related to session analysis
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AnalysisEvent {
    /// Analysis has been started for a session
    AnalysisStarted {
        analysis_id: AnalysisId,
        session_id: SessionId,
        reason: AnalysisReason,
        started_at: Timestamp,
    },

    /// Analysis has been completed
    AnalysisCompleted {
        analysis_id: AnalysisId,
        test_case_count: usize,
        completed_at: Timestamp,
    },

    /// A test case has been identified for extraction
    TestCaseIdentified {
        analysis_id: AnalysisId,
        extraction_id: ExtractionId,
        request_id: RequestId,
        suggested_name: TestCaseName,
        /// Confidence score as a percentage (0-100)
        confidence_score: u8,
    },

    /// Analysis failed
    AnalysisFailed {
        analysis_id: AnalysisId,
        error: ErrorMessage,
        failed_at: Timestamp,
    },
}

/// Events related to test case extraction
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TestCaseExtractionEvent {
    /// Test case extraction has been initiated
    ExtractionStarted {
        extraction_id: ExtractionId,
        analysis_id: AnalysisId,
        request_id: RequestId,
        started_at: Timestamp,
    },

    /// Test case has been successfully extracted
    TestCaseExtracted {
        extraction_id: ExtractionId,
        test_case_id: TestCaseId,
        name: TestCaseName,
        request_content: ResponseText,
        expected_response: ResponseText,
        extracted_at: Timestamp,
    },

    /// Extraction failed
    ExtractionFailed {
        extraction_id: ExtractionId,
        error: ErrorMessage,
        failed_at: Timestamp,
    },
}

/// Combined event type for analysis domain
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AnalysisDomainEvent {
    Analysis(AnalysisEvent),
    Extraction(TestCaseExtractionEvent),
}
