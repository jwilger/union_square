//! Application services and business logic orchestration
//!
//! This module contains application services that coordinate
//! domain logic and infrastructure components.

pub mod app;
pub mod session_analysis;
pub mod trampoline;

pub use app::Application;
pub use session_analysis::{
    SessionAnalysisEffect, SessionAnalysisObservation, SessionAnalysisResult,
    SessionAnalysisWorkflow, SessionEventCount,
};
pub use trampoline::{run_trampoline, EffectInterpreter, Step, StepWorkflow, TrampolineError};
