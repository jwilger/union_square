//! EventCore stream naming conventions.
//!
//! These helpers centralize stream naming at the boundary between domain
//! commands and EventCore infrastructure. They return `Result` because
//! `StreamId` validation is fallible and production code must not panic.

use crate::domain::identifiers::{AnalysisId, ExtractionId};
use crate::domain::session::SessionId;
use crate::domain::user::UserId;
use eventcore::StreamId;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum StreamNameError {
    #[error("invalid stream id `{stream_id}`: {reason}")]
    Invalid { stream_id: String, reason: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetentionPolicy {
    Days(u16),
    Indefinite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamLifecycle {
    Bounded {
        created_by: &'static str,
        closed_by: &'static str,
        retention: RetentionPolicy,
    },
    Ongoing {
        created_by: &'static str,
        retention: RetentionPolicy,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StreamDocumentation {
    pub stream_pattern: &'static str,
    pub purpose: &'static str,
    pub lifecycle: StreamLifecycle,
    pub related_streams: &'static [&'static str],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionWithAnalysesStreams {
    pub session_stream: StreamId,
    pub analysis_streams: Vec<StreamId>,
}

impl SessionWithAnalysesStreams {
    pub fn all_streams(&self) -> impl Iterator<Item = &StreamId> {
        std::iter::once(&self.session_stream).chain(self.analysis_streams.iter())
    }
}

pub const STREAM_DOCUMENTATION: &[StreamDocumentation] = &[
    StreamDocumentation {
        stream_pattern: "session:{session_id}",
        purpose: "Tracks durable facts for one LLM session.",
        lifecycle: StreamLifecycle::Bounded {
            created_by: "RecordAuditEvent",
            closed_by: "RecordAuditEvent(SessionEnded)",
            retention: RetentionPolicy::Days(90),
        },
        related_streams: &["analysis:{analysis_id}", "user:{user_id}:settings"],
    },
    StreamDocumentation {
        stream_pattern: "analysis:{analysis_id}",
        purpose: "Tracks analysis workflow decisions and outcomes.",
        lifecycle: StreamLifecycle::Bounded {
            created_by: "StartSessionAnalysis",
            closed_by: "CompleteSessionAnalysis",
            retention: RetentionPolicy::Days(365),
        },
        related_streams: &["session:{session_id}", "extraction:{extraction_id}"],
    },
    StreamDocumentation {
        stream_pattern: "user:{user_id}:settings",
        purpose: "Tracks settings that affect a user's Union Square experience.",
        lifecycle: StreamLifecycle::Ongoing {
            created_by: "UpdateUserSettings",
            retention: RetentionPolicy::Indefinite,
        },
        related_streams: &["session:{session_id}"],
    },
    StreamDocumentation {
        stream_pattern: "extraction:{extraction_id}",
        purpose: "Tracks test-case extraction workflow decisions and outcomes.",
        lifecycle: StreamLifecycle::Bounded {
            created_by: "StartTestCaseExtraction",
            closed_by: "CompleteTestCaseExtraction",
            retention: RetentionPolicy::Days(365),
        },
        related_streams: &["analysis:{analysis_id}", "session:{session_id}"],
    },
];

pub fn session_stream(session_id: &SessionId) -> Result<StreamId, StreamNameError> {
    stream_id(format!("session:{}", session_id.as_ref()))
}

pub fn analysis_stream(analysis_id: &AnalysisId) -> Result<StreamId, StreamNameError> {
    stream_id(format!("analysis:{analysis_id}"))
}

pub fn user_settings_stream(user_id: &UserId) -> Result<StreamId, StreamNameError> {
    stream_id(format!("user:{}:settings", user_id.as_ref()))
}

pub fn extraction_stream(extraction_id: &ExtractionId) -> Result<StreamId, StreamNameError> {
    stream_id(format!("extraction:{extraction_id}"))
}

pub fn session_with_analyses_streams(
    session_id: &SessionId,
    analysis_ids: &[AnalysisId],
) -> Result<SessionWithAnalysesStreams, StreamNameError> {
    let session_stream = session_stream(session_id)?;
    let analysis_streams = analysis_ids
        .iter()
        .map(analysis_stream)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(SessionWithAnalysesStreams {
        session_stream,
        analysis_streams,
    })
}

fn stream_id(raw: String) -> Result<StreamId, StreamNameError> {
    StreamId::try_new(raw.clone()).map_err(|error| StreamNameError::Invalid {
        stream_id: raw,
        reason: error.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::commands::metrics_commands::{RecordApplicationFScore, RecordModelFScore};
    use crate::domain::commands::version_commands::RecordVersionChange;
    use crate::domain::identifiers::{AnalysisId, ExtractionId};
    use crate::domain::llm::{LlmProvider, ModelVersion};
    use crate::domain::metrics::{Precision, Recall, SampleCount};
    use crate::domain::session::{ApplicationId, SessionId};
    use crate::domain::test_data;
    use crate::domain::types::{ChangeReason, ModelId};
    use crate::domain::user::UserId;
    use eventcore::CommandStreams;

    #[test]
    fn stream_factories_use_canonical_names() {
        let session_id = SessionId::generate();
        let analysis_id = AnalysisId::generate();
        let user_id = UserId::generate();
        let extraction_id = ExtractionId::generate();

        assert_eq!(
            session_stream(&session_id).unwrap().as_ref(),
            format!("session:{}", session_id.as_ref())
        );
        assert_eq!(
            analysis_stream(&analysis_id).unwrap().as_ref(),
            format!("analysis:{analysis_id}")
        );
        assert_eq!(
            user_settings_stream(&user_id).unwrap().as_ref(),
            format!("user:{}:settings", user_id.as_ref())
        );
        assert_eq!(
            extraction_stream(&extraction_id).unwrap().as_ref(),
            format!("extraction:{extraction_id}")
        );
    }

    #[test]
    fn stream_lifecycle_documentation_covers_core_patterns() {
        let patterns: Vec<&str> = STREAM_DOCUMENTATION
            .iter()
            .map(|doc| doc.stream_pattern)
            .collect();

        assert!(patterns.contains(&"session:{session_id}"));
        assert!(patterns.contains(&"analysis:{analysis_id}"));
        assert!(patterns.contains(&"user:{user_id}:settings"));
        assert!(patterns.contains(&"extraction:{extraction_id}"));
    }

    #[test]
    fn session_with_analyses_query_plan_includes_session_and_analysis_streams() {
        let session_id = SessionId::generate();
        let analysis_ids = [AnalysisId::generate(), AnalysisId::generate()];

        let plan = session_with_analyses_streams(&session_id, &analysis_ids).unwrap();

        assert_eq!(
            plan.session_stream.as_ref(),
            format!("session:{}", session_id.as_ref())
        );
        assert_eq!(plan.analysis_streams.len(), 2);
        assert_eq!(
            plan.all_streams()
                .map(|stream| stream.as_ref())
                .collect::<Vec<_>>(),
            vec![
                plan.session_stream.as_ref(),
                plan.analysis_streams[0].as_ref(),
                plan.analysis_streams[1].as_ref(),
            ]
        );
    }

    #[test]
    fn existing_multi_stream_commands_declare_consistency_boundaries() {
        let session_id = SessionId::generate();
        let from_version = model_version("claude-3-haiku");
        let to_version = model_version("claude-3-5-haiku");
        let version_change = RecordVersionChange::new(
            session_id.clone(),
            from_version,
            to_version,
            Some(ChangeReason::try_new("provider migration".to_string()).unwrap()),
        );

        assert_eq!(version_change.stream_declarations().len(), 2);

        let model_score = RecordModelFScore::new(
            session_id.clone(),
            model_version("gpt-4o-mini"),
            Precision::try_new(0.9).unwrap(),
            Recall::try_new(0.8).unwrap(),
            SampleCount::try_new(test_data::numeric::BATCH_SIZE_100 as u64).unwrap(),
        );
        assert_eq!(model_score.stream_declarations().len(), 1);

        let application_score = RecordApplicationFScore::new(
            session_id,
            ApplicationId::try_new("union-square".to_string()).unwrap(),
            model_version("gpt-4o-mini"),
            Precision::try_new(0.9).unwrap(),
            Recall::try_new(0.8).unwrap(),
            SampleCount::try_new(test_data::numeric::BATCH_SIZE_100 as u64).unwrap(),
        );
        assert_eq!(application_score.stream_declarations().len(), 2);
    }

    fn model_version(model_id: &str) -> ModelVersion {
        ModelVersion {
            provider: LlmProvider::OpenAI,
            model_id: ModelId::try_new(model_id.to_string()).unwrap(),
        }
    }
}
