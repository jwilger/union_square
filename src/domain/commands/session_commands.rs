//! Type-safe session management commands using EventCore patterns
//!
//! These commands demonstrate EventCore's stream-centric approach where
//! commands dynamically define their consistency boundaries.

use async_trait::async_trait;
use eventcore::{
    emit, require, CommandLogic, CommandResult, ReadStreams, StoredEvent, StreamId, StreamResolver,
    StreamWrite,
};
use eventcore_macros::Command;
use serde::{Deserialize, Serialize};

use crate::domain::{
    events::DomainEvent,
    metrics::Timestamp,
    session::{ApplicationId, SessionId, SessionStatus},
    streams::{streams, AnalysisId, TypedStreamId, SessionStream, AnalysisStream},
    user::UserId,
    workflows::{SessionWorkflow, AnalysisWorkflow, Unconfigured, AnalysisResults},
    types::Tag,
};

/// Start a new session
///
/// This command creates a new session stream and emits the initial event.
/// It demonstrates single-stream operations in EventCore.
#[derive(Debug, Clone, Serialize, Deserialize, Command)]
pub struct StartSession {
    #[stream]
    pub session_stream: StreamId,
    pub session_id: SessionId,
    pub user_id: UserId,
    pub application_id: ApplicationId,
    pub timestamp: Timestamp,
}

impl StartSession {
    /// Create a new StartSession command
    pub fn new(
        session_id: SessionId,
        user_id: UserId,
        application_id: ApplicationId,
        timestamp: Timestamp,
    ) -> crate::error::Result<Self> {
        let session_stream = streams::session(&session_id)?.into_stream_id();
        
        Ok(Self {
            session_stream,
            session_id,
            user_id,
            application_id,
            timestamp,
        })
    }
}

#[async_trait]
impl CommandLogic for StartSession {
    type State = SessionWorkflow;
    type Event = DomainEvent;

    fn apply(&self, state: &mut Self::State, event: &StoredEvent<Self::Event>) {
        // Let the workflow handle state transitions
        let _ = state.apply_event(&event.payload);
    }

    async fn handle(
        &self,
        read_streams: ReadStreams<Self::StreamSet>,
        state: Self::State,
        _stream_resolver: &mut StreamResolver,
    ) -> CommandResult<Vec<StreamWrite<Self::StreamSet, Self::Event>>> {
        let mut events = Vec::new();

        // Ensure session hasn't already started
        require!(
            matches!(state, SessionWorkflow::NotStarted),
            "Session already started"
        );

        // Emit session started event
        emit!(
            events,
            &read_streams,
            self.session_stream.clone(),
            DomainEvent::SessionStarted {
                session_id: self.session_id.clone(),
                user_id: self.user_id.clone(),
                application_id: self.application_id.clone(),
                started_at: self.timestamp,
            }
        );

        Ok(events)
    }
}

/// End a session
///
/// This command demonstrates how to properly close a session stream
/// and emit the final event.
#[derive(Debug, Clone, Serialize, Deserialize, Command)]
pub struct EndSession {
    #[stream]
    pub session_stream: StreamId,
    pub session_id: SessionId,
    pub final_status: SessionStatus,
    pub timestamp: Timestamp,
}

impl EndSession {
    /// Create a new EndSession command
    pub fn new(
        session_id: SessionId,
        final_status: SessionStatus,
        timestamp: Timestamp,
    ) -> crate::error::Result<Self> {
        let session_stream = streams::session(&session_id)?.into_stream_id();
        
        Ok(Self {
            session_stream,
            session_id,
            final_status,
            timestamp,
        })
    }
}

#[async_trait]
impl CommandLogic for EndSession {
    type State = SessionWorkflow;
    type Event = DomainEvent;

    fn apply(&self, state: &mut Self::State, event: &StoredEvent<Self::Event>) {
        let _ = state.apply_event(&event.payload);
    }

    async fn handle(
        &self,
        read_streams: ReadStreams<Self::StreamSet>,
        state: Self::State,
        _stream_resolver: &mut StreamResolver,
    ) -> CommandResult<Vec<StreamWrite<Self::StreamSet, Self::Event>>> {
        let mut events = Vec::new();

        // Ensure session is active
        require!(
            matches!(state, SessionWorkflow::Active { .. }),
            "Cannot end inactive session"
        );

        // Emit session ended event
        emit!(
            events,
            &read_streams,
            self.session_stream.clone(),
            DomainEvent::SessionEnded {
                session_id: self.session_id.clone(),
                ended_at: self.timestamp,
                final_status: self.final_status.clone(),
            }
        );

        Ok(events)
    }
}

/// Tag a session
///
/// This command adds tags to a session for categorization and search.
/// It demonstrates adding metadata to an existing stream.
#[derive(Debug, Clone, Serialize, Deserialize, Command)]
pub struct TagSession {
    #[stream]
    pub session_stream: StreamId,
    pub session_id: SessionId,
    pub tag: Tag,
    pub timestamp: Timestamp,
}

impl TagSession {
    /// Create a new TagSession command
    pub fn new(
        session_id: SessionId,
        tag: Tag,
        timestamp: Timestamp,
    ) -> crate::error::Result<Self> {
        let session_stream = streams::session(&session_id)?.into_stream_id();
        
        Ok(Self {
            session_stream,
            session_id,
            tag,
            timestamp,
        })
    }
}

#[async_trait]
impl CommandLogic for TagSession {
    type State = SessionWorkflow;
    type Event = DomainEvent;

    fn apply(&self, state: &mut Self::State, event: &StoredEvent<Self::Event>) {
        let _ = state.apply_event(&event.payload);
    }

    async fn handle(
        &self,
        read_streams: ReadStreams<Self::StreamSet>,
        state: Self::State,
        _stream_resolver: &mut StreamResolver,
    ) -> CommandResult<Vec<StreamWrite<Self::StreamSet, Self::Event>>> {
        let mut events = Vec::new();

        // Can only tag active or completed sessions
        require!(
            matches!(
                state,
                SessionWorkflow::Active { .. } | SessionWorkflow::Completed { .. }
            ),
            "Cannot tag failed or non-existent session"
        );

        // Emit tagged event
        emit!(
            events,
            &read_streams,
            self.session_stream.clone(),
            DomainEvent::SessionTagged {
                session_id: self.session_id.clone(),
                tag: self.tag.clone(),
                tagged_at: self.timestamp,
            }
        );

        Ok(events)
    }
}

/// Start session analysis
///
/// This command demonstrates EventCore's multi-stream atomic operations.
/// It creates a new analysis stream while also updating the session stream,
/// ensuring both changes happen atomically.
#[derive(Debug, Clone, Serialize, Deserialize, Command)]
pub struct StartSessionAnalysis {
    #[stream]
    pub session_stream: StreamId,
    #[stream]
    pub analysis_stream: StreamId,
    pub session_id: SessionId,
    pub analysis_id: AnalysisId,
    pub analysis_config: AnalysisConfig,
    pub timestamp: Timestamp,
}

/// Analysis configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisConfig {
    pub patterns_to_detect: Vec<String>,
    pub metrics_to_calculate: Vec<MetricType>,
    pub include_recommendations: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MetricType {
    ResponseTime,
    TokenUsage,
    CostAnalysis,
    ErrorRate,
}

impl StartSessionAnalysis {
    /// Create a new StartSessionAnalysis command
    pub fn new(
        session_id: SessionId,
        analysis_id: AnalysisId,
        analysis_config: AnalysisConfig,
        timestamp: Timestamp,
    ) -> crate::error::Result<Self> {
        let session_stream = streams::session(&session_id)?.into_stream_id();
        let analysis_stream = streams::analysis(&analysis_id)?.into_stream_id();
        
        Ok(Self {
            session_stream,
            analysis_stream,
            session_id,
            analysis_id,
            analysis_config,
            timestamp,
        })
    }
}

/// State for session analysis command
#[derive(Debug, Default)]
pub struct SessionAnalysisState {
    pub session_workflow: SessionWorkflow,
    pub analysis_exists: bool,
}

#[async_trait]
impl CommandLogic for StartSessionAnalysis {
    type State = SessionAnalysisState;
    type Event = DomainEvent;

    fn apply(&self, state: &mut Self::State, event: &StoredEvent<Self::Event>) {
        match &event.payload {
            DomainEvent::SessionStarted { .. } | DomainEvent::SessionEnded { .. } => {
                let _ = state.session_workflow.apply_event(&event.payload);
            }
            // Track if analysis already exists
            DomainEvent::AnalysisStarted { .. } => {
                state.analysis_exists = true;
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

        // Business rule: Can only analyze completed sessions
        require!(
            matches!(state.session_workflow, SessionWorkflow::Completed { .. }),
            "Cannot analyze incomplete session"
        );

        // Business rule: Cannot start duplicate analysis
        require!(!state.analysis_exists, "Analysis already started for this session");

        // Emit event to session stream
        emit!(
            events,
            &read_streams,
            self.session_stream.clone(),
            DomainEvent::AnalysisStarted {
                session_id: self.session_id.clone(),
                analysis_id: self.analysis_id.clone(),
            }
        );

        // Emit event to analysis stream
        emit!(
            events,
            &read_streams,
            self.analysis_stream.clone(),
            DomainEvent::AnalysisCreated {
                analysis_id: self.analysis_id.clone(),
                session_id: self.session_id.clone(),
                config: serde_json::to_value(&self.analysis_config)
                    .expect("Analysis config should serialize"),
                created_at: self.timestamp,
            }
        );

        Ok(events)
    }
}

/// Complete analysis with results
///
/// This command demonstrates updating multiple streams with analysis results.
#[derive(Debug, Clone, Serialize, Deserialize, Command)]
pub struct CompleteAnalysis {
    #[stream]
    pub session_stream: StreamId,
    #[stream]
    pub analysis_stream: StreamId,
    pub analysis_id: AnalysisId,
    pub results: AnalysisResults,
    pub timestamp: Timestamp,
}

/// State for completing analysis
#[derive(Debug, Default)]
pub struct CompleteAnalysisState {
    pub analysis_in_progress: bool,
    pub session_id: Option<SessionId>,
}

#[async_trait]
impl CommandLogic for CompleteAnalysis {
    type State = CompleteAnalysisState;
    type Event = DomainEvent;

    fn apply(&self, state: &mut Self::State, event: &StoredEvent<Self::Event>) {
        match &event.payload {
            DomainEvent::AnalysisCreated { session_id, .. } => {
                state.analysis_in_progress = true;
                state.session_id = Some(session_id.clone());
            }
            DomainEvent::AnalysisCompleted { .. } => {
                state.analysis_in_progress = false;
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

        // Ensure analysis is in progress
        require!(
            state.analysis_in_progress,
            "Analysis not in progress"
        );

        let session_id = state.session_id
            .ok_or_else(|| eventcore::CommandError::Validation(
                "Session ID not found in state".to_string()
            ))?;

        // Emit completion event to analysis stream
        emit!(
            events,
            &read_streams,
            self.analysis_stream.clone(),
            DomainEvent::AnalysisCompleted {
                analysis_id: self.analysis_id.clone(),
                results: serde_json::to_value(&self.results)
                    .expect("Analysis results should serialize"),
                completed_at: self.timestamp,
            }
        );

        // Emit notification to session stream
        emit!(
            events,
            &read_streams,
            self.session_stream.clone(),
            DomainEvent::SessionAnalysisCompleted {
                session_id,
                analysis_id: self.analysis_id.clone(),
                summary: create_analysis_summary(&self.results),
            }
        );

        Ok(events)
    }
}

fn create_analysis_summary(results: &AnalysisResults) -> String {
    format!(
        "Analysis completed: {} patterns found, {} recommendations",
        results.pattern_matches.len(),
        results.recommendations.len()
    )
}

// Note: We need to add these new event variants to DomainEvent
// - AnalysisStarted
// - AnalysisCreated
// - AnalysisCompleted
// - SessionAnalysisCompleted

#[cfg(test)]
mod tests {
    use super::*;
    use eventcore::{CommandExecutor, EventStore, ExecutionOptions};
    use eventcore_memory::InMemoryEventStore;

    #[tokio::test]
    async fn test_session_lifecycle() {
        let event_store = InMemoryEventStore::new();
        let executor = CommandExecutor::new(event_store);

        let session_id = SessionId::generate();
        let user_id = UserId::generate();
        let app_id = ApplicationId::try_new("test-app".to_string()).unwrap();
        let timestamp = Timestamp::now();

        // Start session
        let start_cmd = StartSession::new(
            session_id.clone(),
            user_id,
            app_id,
            timestamp,
        ).unwrap();

        let result = executor.execute(start_cmd, ExecutionOptions::default()).await;
        assert!(result.is_ok());

        // End session
        let end_cmd = EndSession::new(
            session_id.clone(),
            SessionStatus::Completed,
            timestamp,
        ).unwrap();

        let result = executor.execute(end_cmd, ExecutionOptions::default()).await;
        assert!(result.is_ok());

        // Cannot start same session again
        let duplicate_start = StartSession::new(
            session_id.clone(),
            user_id,
            app_id,
            timestamp,
        ).unwrap();

        let result = executor.execute(duplicate_start, ExecutionOptions::default()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_multi_stream_analysis() {
        let event_store = InMemoryEventStore::new();
        let executor = CommandExecutor::new(event_store.clone());

        let session_id = SessionId::generate();
        let user_id = UserId::generate();
        let app_id = ApplicationId::try_new("test-app".to_string()).unwrap();
        let analysis_id = AnalysisId::generate();
        let timestamp = Timestamp::now();

        // Start and complete session first
        let start_cmd = StartSession::new(
            session_id.clone(),
            user_id,
            app_id,
            timestamp,
        ).unwrap();
        executor.execute(start_cmd, ExecutionOptions::default()).await.unwrap();

        let end_cmd = EndSession::new(
            session_id.clone(),
            SessionStatus::Completed,
            timestamp,
        ).unwrap();
        executor.execute(end_cmd, ExecutionOptions::default()).await.unwrap();

        // Start analysis
        let analysis_config = AnalysisConfig {
            patterns_to_detect: vec!["error_patterns".to_string()],
            metrics_to_calculate: vec![MetricType::ResponseTime],
            include_recommendations: true,
        };

        let analysis_cmd = StartSessionAnalysis::new(
            session_id.clone(),
            analysis_id.clone(),
            analysis_config,
            timestamp,
        ).unwrap();

        let result = executor.execute(analysis_cmd, ExecutionOptions::default()).await;
        assert!(result.is_ok());

        // Verify events were written to both streams
        let session_stream = streams::session(&session_id).unwrap().into_stream_id();
        let analysis_stream = streams::analysis(&analysis_id).unwrap().into_stream_id();

        let stream_data = event_store
            .read_streams(&[session_stream, analysis_stream], &Default::default())
            .await
            .unwrap();

        // Should have events in both streams
        assert!(!stream_data.events.is_empty());
    }
}