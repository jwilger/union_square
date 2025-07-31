//! Example EventCore commands demonstrating patterns for session analysis
//!
//! This module shows how to implement commands using EventCore's #[derive(Command)] macro
//! with proper stream annotations, multi-stream operations, and state management.

use crate::domain::events::DomainEvent;
use crate::domain::identifiers::{AnalysisId, ExtractionId};
use crate::domain::llm::{ModelVersion, RequestId};
use crate::domain::metrics::Timestamp;
use crate::domain::session::{ApplicationId, SessionId};
use crate::domain::test_case::{TestCaseId, TestCaseName};
use crate::domain::types::{LlmParameters, Prompt};
use crate::domain::user::UserId;
use async_trait::async_trait;
use eventcore::StreamId;
use eventcore::{
    emit, require, CommandLogic, CommandResult, ReadOptions, ReadStreams, StoredEvent,
    StreamResolver, StreamWrite,
};
use eventcore_macros::Command;
use nutype::nutype;
use serde::{Deserialize, Serialize};

// Additional value objects not in main domain yet
#[nutype(derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq))]
pub struct AnalysisReason(String);

// Example 1: Single-stream command - Starting a session
#[derive(Command, Clone, Debug, Serialize, Deserialize)]
pub struct StartSession {
    #[stream]
    session_stream: StreamId,

    // Command data
    user_id: UserId,
    application_id: ApplicationId,
}

// State for session commands
#[derive(Default, Debug)]
pub struct SessionState {
    started: bool,
    ended: bool,
    request_count: usize,
}

#[async_trait]
impl CommandLogic for StartSession {
    type State = SessionState;
    type Event = DomainEvent;

    fn apply(&self, state: &mut Self::State, event: &StoredEvent<Self::Event>) {
        match &event.payload {
            DomainEvent::SessionStarted { .. } => {
                state.started = true;
            }
            DomainEvent::SessionEnded { .. } => {
                state.ended = true;
            }
            DomainEvent::LlmRequestReceived { .. } => {
                state.request_count += 1;
            }
            _ => {}
        }
    }

    async fn handle(
        &self,
        read_streams: ReadStreams<Self::StreamSet>,
        state: Self::State,
        _stream_resolver: &mut StreamResolver,
    ) -> CommandResult<Vec<StreamWrite<Self::StreamSet, Self::Event>>> {
        let mut events = Vec::new();

        // Business rule: Cannot start an already started session
        require!(!state.started, "Session already started");

        // Generate session ID from stream ID
        let session_id = SessionId::generate();

        // Emit the session started event
        emit!(
            events,
            &read_streams,
            self.session_stream.clone(),
            DomainEvent::SessionStarted {
                session_id,
                user_id: self.user_id.clone(),
                application_id: self.application_id.clone(),
                started_at: Timestamp::now(),
            }
        );

        Ok(events)
    }
}

// Example 2: Multi-stream command - Starting analysis across session and analysis streams
// This demonstrates the key pattern from the GitHub issue
#[derive(Command, Clone, Debug, Serialize, Deserialize)]
pub struct StartSessionAnalysis {
    #[stream]
    session_stream: StreamId,
    #[stream]
    analysis_stream: StreamId,

    // Command data
    reason: AnalysisReason,
}

// Combined state from multiple streams
#[derive(Default, Debug)]
pub struct AnalysisCommandState {
    // From session stream
    session_started: bool,
    session_id: Option<SessionId>,
    request_count: usize,

    // From analysis stream
    analysis_started: bool,
    analysis_id: Option<AnalysisId>,
}

#[async_trait]
impl CommandLogic for StartSessionAnalysis {
    type State = AnalysisCommandState;
    type Event = DomainEvent;

    fn apply(&self, state: &mut Self::State, event: &StoredEvent<Self::Event>) {
        // Handle events from different streams by checking stream ID
        match &event.stream_id {
            stream_id if stream_id.as_ref() == self.session_stream.as_ref() => {
                // In a real implementation, you'd decode session events here
                // For this example, we'll track basic session state
                match &event.payload {
                    DomainEvent::SessionStarted { .. } => {
                        state.session_started = true;
                        state.session_id = Some(SessionId::generate());
                    }
                    DomainEvent::LlmRequestReceived { .. } => {
                        state.request_count += 1;
                    }
                    _ => {}
                }
            }
            stream_id if stream_id.as_ref() == self.analysis_stream.as_ref() => {
                // In real code, you'd handle analysis events here
                state.analysis_started = true;
                state.analysis_id = Some(AnalysisId::generate());
            }
            _ => {}
        }
    }

    async fn handle(
        &self,
        read_streams: ReadStreams<Self::StreamSet>,
        state: Self::State,
        _stream_resolver: &mut StreamResolver,
    ) -> CommandResult<Vec<StreamWrite<Self::StreamSet, Self::Event>>> {
        let mut events = Vec::new();

        // Business rules - check state from multiple streams
        require!(state.session_started, "Session not found");
        require!(state.request_count > 0, "No requests to analyze");
        require!(!state.analysis_started, "Analysis already started");

        // Generate IDs
        let _analysis_id = AnalysisId::generate();
        let session_id = state.session_id.unwrap_or_else(SessionId::generate);

        // Emit event to analysis stream
        emit!(
            events,
            &read_streams,
            self.analysis_stream.clone(),
            // In real code, this would be a proper analysis event
            DomainEvent::SessionEnded {
                session_id,
                ended_at: Timestamp::now(),
                final_status: crate::domain::session::SessionStatus::Completed,
            }
        );

        Ok(events)
    }
}

// Example 3: Command with retry strategy
#[derive(Command, Clone, Debug, Serialize, Deserialize)]
pub struct CaptureRequest {
    #[stream]
    session_stream: StreamId,

    // Command data
    request_id: RequestId,
    model_version: ModelVersion,
    prompt: Prompt,
    parameters: LlmParameters,
}

#[async_trait]
impl CommandLogic for CaptureRequest {
    type State = SessionState;
    type Event = DomainEvent;

    fn apply(&self, state: &mut Self::State, event: &StoredEvent<Self::Event>) {
        match &event.payload {
            DomainEvent::SessionStarted { .. } => {
                state.started = true;
            }
            DomainEvent::SessionEnded { .. } => {
                state.ended = true;
            }
            DomainEvent::LlmRequestReceived { .. } => {
                state.request_count += 1;
            }
            _ => {}
        }
    }

    async fn handle(
        &self,
        read_streams: ReadStreams<Self::StreamSet>,
        state: Self::State,
        _stream_resolver: &mut StreamResolver,
    ) -> CommandResult<Vec<StreamWrite<Self::StreamSet, Self::Event>>> {
        let mut events = Vec::new();

        // Business rules
        require!(state.started, "Session not started");
        require!(!state.ended, "Session already ended");

        // StreamResolver doesn't have a resolve method in the current EventCore version
        // In real code, you might use it to dynamically determine which streams to write to

        let session_id = SessionId::generate();

        // Emit the request received event
        emit!(
            events,
            &read_streams,
            self.session_stream.clone(),
            DomainEvent::LlmRequestReceived {
                request_id: self.request_id.clone(),
                session_id,
                model_version: self.model_version.clone(),
                prompt: self.prompt.clone(),
                parameters: self.parameters.clone(),
                received_at: Timestamp::now(),
            }
        );

        Ok(events)
    }
}

// Example 4: Three-stream command for test case extraction
#[derive(Command, Clone, Debug, Serialize, Deserialize)]
pub struct ExtractTestCase {
    #[stream]
    session_stream: StreamId,
    #[stream]
    analysis_stream: StreamId,
    #[stream]
    test_case_stream: StreamId,

    // Command data
    request_id: RequestId,
    test_name: TestCaseName,
}

// State combining information from all three streams
#[derive(Default, Debug)]
pub struct ExtractTestCaseState {
    // Track state from each stream
    session_exists: bool,
    analysis_active: bool,
    test_case_already_extracted: bool,
}

#[async_trait]
impl CommandLogic for ExtractTestCase {
    type State = ExtractTestCaseState;
    type Event = DomainEvent;

    fn apply(&self, state: &mut Self::State, event: &StoredEvent<Self::Event>) {
        // Handle events from different streams
        match &event.stream_id {
            stream_id if stream_id.as_ref() == self.session_stream.as_ref() => {
                state.session_exists = true;
            }
            stream_id if stream_id.as_ref() == self.analysis_stream.as_ref() => {
                // In real code, you'd handle analysis events here
                state.analysis_active = true;
            }
            stream_id if stream_id.as_ref() == self.test_case_stream.as_ref() => {
                // In real code, you'd handle test case events here
                state.test_case_already_extracted = false;
            }
            _ => {}
        }
    }

    async fn handle(
        &self,
        read_streams: ReadStreams<Self::StreamSet>,
        state: Self::State,
        _stream_resolver: &mut StreamResolver,
    ) -> CommandResult<Vec<StreamWrite<Self::StreamSet, Self::Event>>> {
        let mut events = Vec::new();

        // Complex business rules checking state from multiple streams
        require!(state.session_exists, "Session not found");
        require!(state.analysis_active, "No active analysis");
        require!(
            !state.test_case_already_extracted,
            "Test case already extracted for this request"
        );

        // Generate IDs
        let _extraction_id = ExtractionId::generate();
        let _test_case_id = TestCaseId::generate();
        let _analysis_id = AnalysisId::generate();

        // Emit events to multiple streams atomically

        // 1. Record in analysis stream that a test case was identified
        emit!(
            events,
            &read_streams,
            self.analysis_stream.clone(),
            // In real code, this would be a proper test case event
            DomainEvent::SessionEnded {
                session_id: SessionId::generate(),
                ended_at: Timestamp::now(),
                final_status: crate::domain::session::SessionStatus::Completed,
            }
        );

        // 2. Create the test case extraction in its own stream
        emit!(
            events,
            &read_streams,
            self.test_case_stream.clone(),
            // In real code, this would be a proper extraction event
            DomainEvent::SessionEnded {
                session_id: SessionId::generate(),
                ended_at: Timestamp::now(),
                final_status: crate::domain::session::SessionStatus::Completed,
            }
        );

        Ok(events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eventcore::{CommandExecutor, EventStore, ExecutionOptions};
    use eventcore_memory::InMemoryEventStore;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_start_session_command() {
        // Create in-memory event store
        let event_store = InMemoryEventStore::new();
        let executor = Arc::new(CommandExecutor::new(event_store));

        // Create stream ID
        let session_id = SessionId::generate();
        let session_stream = crate::domain::streams::session_stream(&session_id);

        // Create command
        let command = StartSession {
            session_stream: session_stream.clone(),
            user_id: UserId::generate(),
            application_id: ApplicationId::try_new("test-app".to_string()).unwrap(),
        };

        // Execute command
        let result = executor.execute(command, ExecutionOptions::default()).await;
        assert!(result.is_ok());

        // Verify events were written
        let events = executor
            .event_store()
            .read_streams(&[session_stream.clone()], &ReadOptions::default())
            .await
            .unwrap();

        assert_eq!(events.events.len(), 1);
        match &events.events[0].payload {
            DomainEvent::SessionStarted { application_id, .. } => {
                assert_eq!(application_id.as_ref(), "test-app");
            }
            _ => panic!("Expected SessionStarted event"),
        }
    }

    #[tokio::test]
    async fn test_cannot_start_session_twice() {
        let event_store = InMemoryEventStore::new();
        let executor = Arc::new(CommandExecutor::new(event_store));
        let session_id = SessionId::generate();
        let session_stream = crate::domain::streams::session_stream(&session_id);

        let command = StartSession {
            session_stream: session_stream.clone(),
            user_id: UserId::generate(),
            application_id: ApplicationId::try_new("test-app".to_string()).unwrap(),
        };

        // First execution should succeed
        assert!(executor
            .execute(command.clone(), ExecutionOptions::default())
            .await
            .is_ok());

        // Second execution should fail
        let result = executor.execute(command, ExecutionOptions::default()).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Session already started"));
    }

    #[tokio::test]
    async fn test_multi_stream_analysis_command() {
        let event_store = InMemoryEventStore::new();
        let executor = Arc::new(CommandExecutor::new(event_store));

        // First, start a session
        let session_id = SessionId::generate();
        let session_stream = crate::domain::streams::session_stream(&session_id);
        let start_session = StartSession {
            session_stream: session_stream.clone(),
            user_id: UserId::generate(),
            application_id: ApplicationId::try_new("test-app".to_string()).unwrap(),
        };
        executor
            .execute(start_session, ExecutionOptions::default())
            .await
            .unwrap();

        // Capture a request to have something to analyze
        let capture_request = CaptureRequest {
            session_stream: session_stream.clone(),
            request_id: RequestId::generate(),
            model_version: ModelVersion {
                provider: crate::domain::llm::LlmProvider::OpenAI,
                model_id: crate::domain::types::ModelId::try_new("gpt-4-0613".to_string()).unwrap(),
            },
            prompt: Prompt::try_new("Hello, world!".to_string()).unwrap(),
            parameters: LlmParameters::new(serde_json::json!({})),
        };
        executor
            .execute(capture_request, ExecutionOptions::default())
            .await
            .unwrap();

        // Now start analysis
        let analysis_id = AnalysisId::generate();
        let analysis_stream = crate::domain::streams::analysis_stream(&analysis_id);
        let start_analysis = StartSessionAnalysis {
            session_stream: session_stream.clone(),
            analysis_stream: analysis_stream.clone(),
            reason: AnalysisReason::new("Test analysis".to_string()),
        };

        let result = executor
            .execute(start_analysis, ExecutionOptions::default())
            .await;
        assert!(result.is_ok());

        // Verify event was written to analysis stream
        let events = executor
            .event_store()
            .read_streams(&[analysis_stream.clone()], &ReadOptions::default())
            .await
            .unwrap();

        // For this example, we just verify an event was written
        assert_eq!(events.events.len(), 1);
    }

    #[tokio::test]
    async fn test_three_stream_extraction_command() {
        let event_store = InMemoryEventStore::new();
        let executor = Arc::new(CommandExecutor::new(event_store));

        // Setup: Create session, start analysis
        let session_id = SessionId::generate();
        let session_stream = crate::domain::streams::session_stream(&session_id);
        let analysis_id = AnalysisId::generate();
        let analysis_stream = crate::domain::streams::analysis_stream(&analysis_id);
        let _test_case_id = TestCaseId::generate();
        let test_case_stream = crate::domain::streams::extraction_stream(&ExtractionId::generate());

        // Pre-populate some events to set up state
        // (In a real test, you'd use proper commands to set up the state)

        // Extract test case across three streams
        let extract_command = ExtractTestCase {
            session_stream,
            analysis_stream: analysis_stream.clone(),
            test_case_stream: test_case_stream.clone(),
            request_id: RequestId::generate(),
            test_name: TestCaseName::try_new("Test Case 1".to_string()).unwrap(),
        };

        // This would fail without proper setup, demonstrating the state validation
        let result = executor
            .execute(extract_command, ExecutionOptions::default())
            .await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Session not found"));
    }
}
