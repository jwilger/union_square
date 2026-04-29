//! Reusable step-trampoline primitives for application orchestration.
//!
//! Workflows describe effects as data. Interpreters perform IO in the
//! imperative shell and feed observations back into the workflow.

use async_trait::async_trait;

/// A workflow step either requests an effect, completes, or fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step<Effect, Output, Error> {
    Effect(Effect),
    Complete(Output),
    Failed(Error),
}

/// Pure workflow state machine driven by observations from an interpreter.
pub trait StepWorkflow {
    type Effect;
    type Error;
    type Observation;
    type Output;

    fn next_step(
        &mut self,
        observation: Option<Self::Observation>,
    ) -> Step<Self::Effect, Self::Output, Self::Error>;
}

/// Interpreter owned by the imperative shell.
#[async_trait]
pub trait EffectInterpreter<Effect> {
    type Error;
    type Observation;

    async fn interpret(&mut self, effect: Effect) -> Result<Self::Observation, Self::Error>;
}

/// Error returned by the trampoline when workflow or interpreter execution fails.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TrampolineError<WorkflowError, InterpreterError> {
    #[error("workflow failed: {0}")]
    Workflow(WorkflowError),
    #[error("interpreter failed: {0}")]
    Interpreter(InterpreterError),
}

/// Run a workflow by interpreting each requested effect until it completes.
pub async fn run_trampoline<Workflow, Interpreter>(
    workflow: &mut Workflow,
    interpreter: &mut Interpreter,
) -> Result<Workflow::Output, TrampolineError<Workflow::Error, Interpreter::Error>>
where
    Workflow: StepWorkflow,
    Interpreter: EffectInterpreter<Workflow::Effect, Observation = Workflow::Observation>,
{
    let mut observation = None;

    loop {
        match workflow.next_step(observation.take()) {
            Step::Effect(effect) => {
                observation = Some(
                    interpreter
                        .interpret(effect)
                        .await
                        .map_err(TrampolineError::Interpreter)?,
                );
            }
            Step::Complete(output) => return Ok(output),
            Step::Failed(error) => return Err(TrampolineError::Workflow(error)),
        }
    }
}
