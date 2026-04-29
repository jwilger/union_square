//! Non-hot-path session analysis workflow expressed as pure steps.

use crate::{
    application::trampoline::{Step, StepWorkflow},
    domain::{identifiers::AnalysisId, session::SessionId},
};
use nutype::nutype;

#[nutype(validate(greater = 0), derive(Debug, Clone, Copy, PartialEq, Eq, Hash))]
pub struct SessionEventCount(u64);

/// Effects requested by session-analysis orchestration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionAnalysisEffect {
    LoadSessionFacts {
        session_id: SessionId,
    },
    RecordAnalysisRequested {
        session_id: SessionId,
        analysis_id: AnalysisId,
        event_count: SessionEventCount,
    },
    EmitTelemetry {
        analysis_id: AnalysisId,
        event_count: SessionEventCount,
    },
}

/// Observations returned by a session-analysis interpreter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionAnalysisObservation {
    SessionFactsLoaded { event_count: SessionEventCount },
    AnalysisRequestRecorded,
    TelemetryEmitted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionAnalysisResult {
    pub session_id: SessionId,
    pub analysis_id: AnalysisId,
    pub event_count: SessionEventCount,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SessionAnalysisError {
    #[error("session analysis workflow received an unexpected observation")]
    UnexpectedObservation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SessionAnalysisState {
    Ready,
    LoadingFacts,
    RecordingRequest { event_count: SessionEventCount },
    EmittingTelemetry { event_count: SessionEventCount },
    Complete,
}

/// Session analysis is non-hot-path orchestration and can use the trampoline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionAnalysisWorkflow {
    session_id: SessionId,
    analysis_id: AnalysisId,
    state: SessionAnalysisState,
}

impl SessionAnalysisWorkflow {
    pub fn new(session_id: SessionId, analysis_id: AnalysisId) -> Self {
        Self {
            session_id,
            analysis_id,
            state: SessionAnalysisState::Ready,
        }
    }
}

impl StepWorkflow for SessionAnalysisWorkflow {
    type Effect = SessionAnalysisEffect;
    type Error = SessionAnalysisError;
    type Observation = SessionAnalysisObservation;
    type Output = SessionAnalysisResult;

    fn next_step(
        &mut self,
        observation: Option<Self::Observation>,
    ) -> Step<Self::Effect, Self::Output, Self::Error> {
        match (self.state.clone(), observation) {
            (SessionAnalysisState::Ready, None) => {
                self.state = SessionAnalysisState::LoadingFacts;
                Step::Effect(SessionAnalysisEffect::LoadSessionFacts {
                    session_id: self.session_id.clone(),
                })
            }
            (
                SessionAnalysisState::LoadingFacts,
                Some(SessionAnalysisObservation::SessionFactsLoaded { event_count }),
            ) => {
                self.state = SessionAnalysisState::RecordingRequest { event_count };
                Step::Effect(SessionAnalysisEffect::RecordAnalysisRequested {
                    session_id: self.session_id.clone(),
                    analysis_id: self.analysis_id.clone(),
                    event_count,
                })
            }
            (
                SessionAnalysisState::RecordingRequest { event_count },
                Some(SessionAnalysisObservation::AnalysisRequestRecorded),
            ) => {
                self.state = SessionAnalysisState::EmittingTelemetry { event_count };
                Step::Effect(SessionAnalysisEffect::EmitTelemetry {
                    analysis_id: self.analysis_id.clone(),
                    event_count,
                })
            }
            (
                SessionAnalysisState::EmittingTelemetry { event_count },
                Some(SessionAnalysisObservation::TelemetryEmitted),
            ) => {
                self.state = SessionAnalysisState::Complete;
                Step::Complete(SessionAnalysisResult {
                    session_id: self.session_id.clone(),
                    analysis_id: self.analysis_id.clone(),
                    event_count,
                })
            }
            _ => Step::Failed(SessionAnalysisError::UnexpectedObservation),
        }
    }
}
