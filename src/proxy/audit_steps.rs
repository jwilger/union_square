//! Pure step model for the audit path
//!
//! This module defines the effect-driven workflow for processing audit events.
//! All decisions are pure functions; IO is performed by the interpreter in
//! `audit_path.rs`.

use crate::domain::commands::audit_commands::RecordAuditEvent;
use crate::proxy::types::{AuditEvent, RequestId};
use std::time::Duration;

/// Log levels for audit path logging effects
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

/// Effects that the audit path can request from the interpreter
#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum AuditEffect {
    /// Deserialize ring buffer bytes into a proxy AuditEvent
    Deserialize { data: Vec<u8> },
    /// Convert a proxy AuditEvent into a domain RecordAuditEvent command
    ConvertToDomain { event: AuditEvent },
    /// Persist a domain command through EventCore
    Persist { command: RecordAuditEvent },
    /// Log a message at the given level
    Log { level: LogLevel, message: String },
    /// Sleep before the next iteration
    Sleep { duration: Duration },
}

/// Observations fed back into the pure core after an effect is performed
///
/// Some variants carry large domain values (e.g., deserialized events).  Boxing
/// would save enum space but adds allocation noise; the audit path is not on the
/// hot path, so we keep the flat representation for clarity.
#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum Observation {
    /// Result of reading the ring buffer
    RingBufferRead(Option<(RequestId, Vec<u8>)>),
    /// Result of deserializing bytes
    Deserialized(Result<AuditEvent, String>),
    /// Result of converting proxy event to domain command
    Converted(Result<RecordAuditEvent, String>),
    /// Result of persisting the command
    Persisted(Result<(), String>),
    /// Log effect completed
    LogComplete,
    /// Sleep effect completed
    SleepComplete,
    /// Shutdown was requested
    ShutdownRequested,
}

/// State tracked by the audit processor
#[derive(Debug, Clone, Default)]
pub struct ProcessorState {
    pub events_processed: u64,
    pub deserialization_failures: u64,
    pub conversion_failures: u64,
    pub persist_failures: u64,
}

/// The next action the interpreter should take
#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum Step {
    /// Perform the given effect and feed the result back
    Effect(AuditEffect),
    /// Continue to the next iteration
    Continue,
    /// Stop the processor
    Stop,
}

/// Pure function: given current state and observation, decide the next step.
pub fn step(state: ProcessorState, observation: Observation) -> (ProcessorState, Step) {
    use AuditEffect::*;
    use Observation::*;

    match observation {
        RingBufferRead(None) => (
            state,
            Step::Effect(Sleep {
                duration: Duration::from_millis(10),
            }),
        ),
        RingBufferRead(Some((_request_id, data))) => (state, Step::Effect(Deserialize { data })),
        Deserialized(Err(e)) => {
            let state = ProcessorState {
                deserialization_failures: state.deserialization_failures + 1,
                ..state
            };
            (
                state,
                Step::Effect(Log {
                    level: LogLevel::Warn,
                    message: format!("Failed to deserialize audit event: {e}"),
                }),
            )
        }
        Deserialized(Ok(event)) => (state, Step::Effect(ConvertToDomain { event })),
        Converted(Err(e)) => {
            let state = ProcessorState {
                conversion_failures: state.conversion_failures + 1,
                ..state
            };
            (
                state,
                Step::Effect(Log {
                    level: LogLevel::Warn,
                    message: format!("Failed to convert audit event: {e}"),
                }),
            )
        }
        Converted(Ok(command)) => (state, Step::Effect(Persist { command })),
        Persisted(Err(e)) => {
            let state = ProcessorState {
                persist_failures: state.persist_failures + 1,
                ..state
            };
            (
                state,
                Step::Effect(Log {
                    level: LogLevel::Error,
                    message: format!("Failed to persist audit event: {e}"),
                }),
            )
        }
        Persisted(Ok(())) => {
            let state = ProcessorState {
                events_processed: state.events_processed + 1,
                ..state
            };
            (state, Step::Continue)
        }
        LogComplete | SleepComplete => (state, Step::Continue),
        ShutdownRequested => (state, Step::Stop),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_ring_buffer_sleeps() {
        let state = ProcessorState::default();
        let (_, step) = step(state, Observation::RingBufferRead(None));
        assert!(
            matches!(
                &step,
                Step::Effect(AuditEffect::Sleep { duration })
                    if duration.as_millis() == 10
            ),
            "Expected Sleep effect with 10ms, got {step:?}"
        );
    }

    #[test]
    fn ring_buffer_data_triggers_deserialize() {
        let state = ProcessorState::default();
        let req_id = RequestId::new();
        let data = b"test".to_vec();
        let (_, step) = step(
            state,
            Observation::RingBufferRead(Some((req_id, data.clone()))),
        );
        assert!(
            matches!(
                &step,
                Step::Effect(AuditEffect::Deserialize { data: d })
                    if *d == data
            ),
            "Expected Deserialize effect, got {step:?}"
        );
    }

    #[test]
    fn deserialization_failure_logs_and_counts() {
        let state = ProcessorState::default();
        let (new_state, step) = step(
            state,
            Observation::Deserialized(Err("bad json".to_string())),
        );
        assert_eq!(new_state.deserialization_failures, 1);
        assert!(
            matches!(
                &step,
                Step::Effect(AuditEffect::Log {
                    level: LogLevel::Warn,
                    message,
                }) if message.contains("bad json")
            ),
            "Expected Warn log for bad json, got {step:?}"
        );
    }

    #[test]
    fn successful_persist_increments_counter() {
        let state = ProcessorState::default();
        let (new_state, step) = step(state, Observation::Persisted(Ok(())));
        assert_eq!(new_state.events_processed, 1);
        assert!(
            matches!(step, Step::Continue),
            "Expected Continue, got {step:?}"
        );
    }

    #[test]
    fn persist_failure_logs_and_counts() {
        let state = ProcessorState::default();
        let (new_state, step) = step(state, Observation::Persisted(Err("db down".to_string())));
        assert_eq!(new_state.persist_failures, 1);
        assert!(
            matches!(
                &step,
                Step::Effect(AuditEffect::Log {
                    level: LogLevel::Error,
                    message,
                }) if message.contains("db down")
            ),
            "Expected Error log for db down, got {step:?}"
        );
    }

    #[test]
    fn shutdown_requested_stops() {
        let state = ProcessorState::default();
        let (_, step) = step(state, Observation::ShutdownRequested);
        assert!(matches!(step, Step::Stop), "Expected Stop, got {step:?}");
    }

    #[test]
    fn log_complete_continues() {
        let state = ProcessorState::default();
        let (_, step) = step(state, Observation::LogComplete);
        assert!(
            matches!(step, Step::Continue),
            "Expected Continue, got {step:?}"
        );
    }
}
