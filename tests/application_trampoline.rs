use async_trait::async_trait;
use union_square::{
    application::{
        run_trampoline, EffectInterpreter, SessionAnalysisEffect, SessionAnalysisObservation,
        SessionAnalysisResult, SessionAnalysisWorkflow, SessionEventCount, TrampolineError,
    },
    domain::{identifiers::AnalysisId, session::SessionId},
};

#[derive(Default)]
struct RecordingInterpreter {
    effects: Vec<SessionAnalysisEffect>,
}

#[async_trait]
impl EffectInterpreter<SessionAnalysisEffect> for RecordingInterpreter {
    type Error = &'static str;
    type Observation = SessionAnalysisObservation;

    async fn interpret(
        &mut self,
        effect: SessionAnalysisEffect,
    ) -> Result<Self::Observation, Self::Error> {
        self.effects.push(effect.clone());

        Ok(match effect {
            SessionAnalysisEffect::LoadSessionFacts { .. } => {
                SessionAnalysisObservation::SessionFactsLoaded {
                    event_count: SessionEventCount::try_new(3).unwrap(),
                }
            }
            SessionAnalysisEffect::RecordAnalysisRequested { .. } => {
                SessionAnalysisObservation::AnalysisRequestRecorded
            }
            SessionAnalysisEffect::EmitTelemetry { .. } => {
                SessionAnalysisObservation::TelemetryEmitted
            }
        })
    }
}

struct FailingInterpreter {
    effects: Vec<SessionAnalysisEffect>,
    event_count: SessionEventCount,
}

impl FailingInterpreter {
    fn new(event_count: SessionEventCount) -> Self {
        Self {
            effects: Vec::new(),
            event_count,
        }
    }
}

#[async_trait]
impl EffectInterpreter<SessionAnalysisEffect> for FailingInterpreter {
    type Error = &'static str;
    type Observation = SessionAnalysisObservation;

    async fn interpret(
        &mut self,
        effect: SessionAnalysisEffect,
    ) -> Result<Self::Observation, Self::Error> {
        self.effects.push(effect.clone());

        match effect {
            SessionAnalysisEffect::LoadSessionFacts { .. } => {
                Ok(SessionAnalysisObservation::SessionFactsLoaded {
                    event_count: self.event_count,
                })
            }
            SessionAnalysisEffect::RecordAnalysisRequested { .. } => Err("record failed"),
            SessionAnalysisEffect::EmitTelemetry { .. } => {
                Ok(SessionAnalysisObservation::TelemetryEmitted)
            }
        }
    }
}

struct SubstituteInterpreter {
    effects: Vec<SessionAnalysisEffect>,
    event_count: SessionEventCount,
}

impl SubstituteInterpreter {
    fn new(event_count: SessionEventCount) -> Self {
        Self {
            effects: Vec::new(),
            event_count,
        }
    }
}

#[async_trait]
impl EffectInterpreter<SessionAnalysisEffect> for SubstituteInterpreter {
    type Error = &'static str;
    type Observation = SessionAnalysisObservation;

    async fn interpret(
        &mut self,
        effect: SessionAnalysisEffect,
    ) -> Result<Self::Observation, Self::Error> {
        self.effects.push(effect.clone());

        Ok(match effect {
            SessionAnalysisEffect::LoadSessionFacts { .. } => {
                SessionAnalysisObservation::SessionFactsLoaded {
                    event_count: self.event_count,
                }
            }
            SessionAnalysisEffect::RecordAnalysisRequested { .. } => {
                SessionAnalysisObservation::AnalysisRequestRecorded
            }
            SessionAnalysisEffect::EmitTelemetry { .. } => {
                SessionAnalysisObservation::TelemetryEmitted
            }
        })
    }
}

#[tokio::test]
async fn session_analysis_workflow_completes_through_interpreted_effects() {
    let session_id = SessionId::generate();
    let analysis_id = AnalysisId::generate();
    let event_count = SessionEventCount::try_new(3).unwrap();
    let mut workflow = SessionAnalysisWorkflow::new(session_id.clone(), analysis_id.clone());
    let mut interpreter = RecordingInterpreter::default();

    let result = run_trampoline(&mut workflow, &mut interpreter).await;

    assert_eq!(
        result,
        Ok(SessionAnalysisResult {
            session_id: session_id.clone(),
            analysis_id: analysis_id.clone(),
            event_count,
        })
    );
    assert_eq!(
        interpreter.effects,
        vec![
            SessionAnalysisEffect::LoadSessionFacts {
                session_id: session_id.clone(),
            },
            SessionAnalysisEffect::RecordAnalysisRequested {
                session_id,
                analysis_id: analysis_id.clone(),
                event_count,
            },
            SessionAnalysisEffect::EmitTelemetry {
                analysis_id,
                event_count,
            },
        ]
    );
    assert!(!matches!(result, Err(TrampolineError::Interpreter(_))));
}

#[tokio::test]
async fn interpreter_failure_stops_the_trampoline() {
    let session_id = SessionId::generate();
    let analysis_id = AnalysisId::generate();
    let event_count = SessionEventCount::try_new(2).unwrap();
    let mut workflow = SessionAnalysisWorkflow::new(session_id.clone(), analysis_id.clone());
    let mut interpreter = FailingInterpreter::new(event_count);

    let result = run_trampoline(&mut workflow, &mut interpreter).await;

    assert_eq!(result, Err(TrampolineError::Interpreter("record failed")));
    assert_eq!(
        interpreter.effects,
        vec![
            SessionAnalysisEffect::LoadSessionFacts {
                session_id: session_id.clone(),
            },
            SessionAnalysisEffect::RecordAnalysisRequested {
                session_id,
                analysis_id,
                event_count,
            },
        ]
    );
}

#[tokio::test]
async fn session_analysis_workflow_runs_with_substitute_interpreter() {
    let session_id = SessionId::generate();
    let analysis_id = AnalysisId::generate();
    let event_count = SessionEventCount::try_new(7).unwrap();
    let mut workflow = SessionAnalysisWorkflow::new(session_id.clone(), analysis_id.clone());
    let mut interpreter = SubstituteInterpreter::new(event_count);

    let result = run_trampoline(&mut workflow, &mut interpreter).await;

    assert_eq!(
        result,
        Ok(SessionAnalysisResult {
            session_id: session_id.clone(),
            analysis_id: analysis_id.clone(),
            event_count,
        })
    );
    assert_eq!(
        interpreter.effects,
        vec![
            SessionAnalysisEffect::LoadSessionFacts {
                session_id: session_id.clone(),
            },
            SessionAnalysisEffect::RecordAnalysisRequested {
                session_id,
                analysis_id: analysis_id.clone(),
                event_count,
            },
            SessionAnalysisEffect::EmitTelemetry {
                analysis_id,
                event_count,
            },
        ]
    );
}

#[test]
fn session_event_count_enforces_positive_boundary() {
    assert!(SessionEventCount::try_new(1).is_ok());
    assert!(SessionEventCount::try_new(0).is_err());
}
