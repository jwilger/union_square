//! Stream naming conventions for EventCore event sourcing
//!
//! This module provides factory functions for creating consistent stream IDs
//! following EventCore's stream-centric architecture. Each stream represents
//! a consistency boundary that can span multiple traditional aggregates.
//!
//! Stream naming patterns:
//! - `session:{id}` - All events for a session lifecycle
//! - `analysis:{id}` - Events for an analysis process
//! - `user:{id}:settings` - User-specific settings and preferences
//! - `extraction:{id}` - Test case extraction events

use crate::domain::{AnalysisId, ExtractionId, SessionId, UserId};
use eventcore::StreamId;

/// Creates a stream ID for session events
///
/// This stream contains all events related to a session lifecycle including:
/// - Session started/ended
/// - Requests/responses captured
/// - Errors encountered
pub fn session_stream(session_id: &SessionId) -> StreamId {
    StreamId::try_new(format!("session:{}", session_id.value()))
        .expect("Session stream ID should always be valid")
}

/// Creates a stream ID for analysis events
///
/// This stream tracks the analysis process including:
/// - Analysis created/started
/// - Progress updates
/// - Results generated
/// - Analysis completed
pub fn analysis_stream(analysis_id: &AnalysisId) -> StreamId {
    StreamId::try_new(format!("analysis:{analysis_id}"))
        .expect("Analysis stream ID should always be valid")
}

/// Creates a stream ID for user settings
///
/// This stream contains user-specific configuration:
/// - Preferences updated
/// - API keys configured
/// - Notification settings changed
pub fn user_settings_stream(user_id: &UserId) -> StreamId {
    StreamId::try_new(format!("user:{user_id}:settings"))
        .expect("User settings stream ID should always be valid")
}

/// Creates a stream ID for test case extraction
///
/// This stream tracks extraction process:
/// - Extraction initiated
/// - Test cases identified
/// - Extraction completed
pub fn extraction_stream(extraction_id: &ExtractionId) -> StreamId {
    StreamId::try_new(format!("extraction:{extraction_id}"))
        .expect("Extraction stream ID should always be valid")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stream_names_follow_conventions() {
        let session_id = SessionId::generate();
        let stream = session_stream(&session_id);
        assert!(stream.as_ref().starts_with("session:"));

        let analysis_id = AnalysisId::generate();
        let stream = analysis_stream(&analysis_id);
        assert!(stream.as_ref().starts_with("analysis:"));

        let user_id = UserId::generate();
        let stream = user_settings_stream(&user_id);
        assert!(stream.as_ref().starts_with("user:"));
        assert!(stream.as_ref().ends_with(":settings"));

        let extraction_id = ExtractionId::generate();
        let stream = extraction_stream(&extraction_id);
        assert!(stream.as_ref().starts_with("extraction:"));
    }
}
