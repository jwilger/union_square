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
    fn session_stream_creates_correct_format() {
        // Arrange
        let session_id = SessionId::generate();

        // Act
        let stream_id = session_stream(&session_id);

        // Assert
        assert_eq!(
            stream_id.as_ref(),
            format!("session:{}", session_id.value()),
            "Session stream ID should follow 'session:{{id}}' format"
        );
    }

    #[test]
    fn analysis_stream_creates_correct_format() {
        // Arrange
        let analysis_id = AnalysisId::generate();

        // Act
        let stream_id = analysis_stream(&analysis_id);

        // Assert
        assert_eq!(
            stream_id.as_ref(),
            format!("analysis:{analysis_id}"),
            "Analysis stream ID should follow 'analysis:{{id}}' format"
        );
    }

    #[test]
    fn user_settings_stream_creates_correct_format() {
        // Arrange
        let user_id = UserId::generate();

        // Act
        let stream_id = user_settings_stream(&user_id);

        // Assert
        assert_eq!(
            stream_id.as_ref(),
            format!("user:{user_id}:settings"),
            "User settings stream ID should follow 'user:{{id}}:settings' format"
        );
    }

    #[test]
    fn extraction_stream_creates_correct_format() {
        // Arrange
        let extraction_id = ExtractionId::generate();

        // Act
        let stream_id = extraction_stream(&extraction_id);

        // Assert
        assert_eq!(
            stream_id.as_ref(),
            format!("extraction:{extraction_id}"),
            "Extraction stream ID should follow 'extraction:{{id}}' format"
        );
    }

    #[test]
    fn stream_ids_are_unique_for_different_entities() {
        // Arrange
        let session_id = SessionId::generate();
        let analysis_id = AnalysisId::generate();

        // Act
        let session_stream_id = session_stream(&session_id);
        let analysis_stream_id = analysis_stream(&analysis_id);

        // Assert
        assert_ne!(
            session_stream_id.as_ref(),
            analysis_stream_id.as_ref(),
            "Different entity types should produce different stream IDs"
        );
    }

    #[test]
    fn stream_ids_are_consistent_for_same_entity() {
        // Arrange
        let session_id = SessionId::generate();

        // Act
        let stream1 = session_stream(&session_id);
        let stream2 = session_stream(&session_id);

        // Assert
        assert_eq!(
            stream1.as_ref(),
            stream2.as_ref(),
            "Same entity ID should always produce the same stream ID"
        );
    }

    #[test]
    fn stream_factory_functions_return_valid_eventcore_stream_ids() {
        // Test that our factory functions always produce valid EventCore StreamIds
        // This ensures we're properly handling the Result from StreamId::try_new

        let session_id = SessionId::generate();
        let analysis_id = AnalysisId::generate();
        let user_id = UserId::generate();
        let extraction_id = ExtractionId::generate();

        // These should not panic - they should always produce valid StreamIds
        let session_stream_id = session_stream(&session_id);
        let analysis_stream_id = analysis_stream(&analysis_id);
        let user_stream_id = user_settings_stream(&user_id);
        let extraction_stream_id = extraction_stream(&extraction_id);

        // Verify they can be used with EventCore APIs
        // (this is really just checking they're the right type)
        assert_eq!(session_stream_id, session_stream_id.clone());
        assert_eq!(analysis_stream_id, analysis_stream_id.clone());
        assert_eq!(user_stream_id, user_stream_id.clone());
        assert_eq!(extraction_stream_id, extraction_stream_id.clone());
    }

    #[test]
    fn stream_ids_are_deterministic() {
        // Test that we haven't implemented any stream ID functionality
        // that we don't have yet - this should fail if we add features
        // like stream versioning or namespacing

        // For now, this test documents that we DON'T have:
        // - Stream versioning (e.g., "session:v2:id")
        // - Environment namespacing (e.g., "prod:session:id")
        // - Tenant isolation (e.g., "tenant1:session:id")

        let session_id = SessionId::generate();
        let stream = session_stream(&session_id);

        // Currently, streams are simple and don't include versioning
        assert!(
            !stream.as_ref().contains(":v"),
            "Stream IDs should not contain version markers (not implemented yet)"
        );

        // Currently, streams don't include environment prefixes
        assert!(
            !stream.as_ref().starts_with("prod:")
                && !stream.as_ref().starts_with("dev:")
                && !stream.as_ref().starts_with("test:"),
            "Stream IDs should not contain environment prefixes (not implemented yet)"
        );
    }

    #[test]
    fn stream_ids_cannot_be_empty() {
        // Test that our factory functions never create empty stream IDs
        // even if given unusual inputs

        let session_id = SessionId::generate();
        let stream = session_stream(&session_id);

        assert!(
            !stream.as_ref().is_empty(),
            "Stream IDs should never be empty"
        );
        assert!(
            stream.as_ref().len() > 8, // "session:" prefix is 8 chars
            "Session stream should have more than just the prefix"
        );
    }

    #[test]
    fn stream_parsing_from_string_not_yet_implemented() {
        // This test documents functionality we haven't implemented yet:
        // parsing a StreamId back to its components
        // This should fail because we don't have this feature

        let session_id = SessionId::generate();
        let stream = session_stream(&session_id);

        // We don't have a way to parse stream IDs back to their components yet
        // This is a feature we might need for debugging or analytics
        // For now, this test documents that this is NOT implemented

        // This would be nice to have:
        // let (stream_type, entity_id) = parse_stream_id(&stream)?;
        // assert_eq!(stream_type, "session");
        // assert_eq!(entity_id, session_id.to_string());

        // Since we don't have this, we can only do string manipulation
        let stream_str = stream.as_ref();
        assert!(stream_str.starts_with("session:"));

        // Document that we can't get the session ID back out
        // (this is the missing functionality)
        let id_part = stream_str.strip_prefix("session:").unwrap();
        // We can get the string, but can't convert back to SessionId
        // because SessionId doesn't have a from_string method
        // Use value() to get the actual UUID for stream comparison
        assert_eq!(id_part, session_id.value());
    }

    #[test]
    fn stream_ids_follow_naming_convention() {
        // Test the specific naming patterns we expect
        let session_id = SessionId::generate();
        let analysis_id = AnalysisId::generate();
        let user_id = UserId::generate();
        let extraction_id = ExtractionId::generate();

        let session_stream_id = session_stream(&session_id);
        let analysis_stream_id = analysis_stream(&analysis_id);
        let user_stream_id = user_settings_stream(&user_id);
        let extraction_stream_id = extraction_stream(&extraction_id);

        // Verify prefixes
        assert!(
            session_stream_id.as_ref().starts_with("session:"),
            "Session streams must start with 'session:' prefix"
        );
        assert!(
            analysis_stream_id.as_ref().starts_with("analysis:"),
            "Analysis streams must start with 'analysis:' prefix"
        );
        assert!(
            user_stream_id.as_ref().starts_with("user:")
                && user_stream_id.as_ref().ends_with(":settings"),
            "User settings streams must follow 'user:{{id}}:settings' pattern"
        );
        assert!(
            extraction_stream_id.as_ref().starts_with("extraction:"),
            "Extraction streams must start with 'extraction:' prefix"
        );

        // Verify no double colons or invalid characters
        for stream in [
            &session_stream_id,
            &analysis_stream_id,
            &user_stream_id,
            &extraction_stream_id,
        ] {
            assert!(
                !stream.as_ref().contains("::"),
                "Stream IDs should not contain double colons"
            );
            assert!(
                stream
                    .as_ref()
                    .chars()
                    .all(|c| c.is_ascii() && (c.is_alphanumeric() || c == ':' || c == '-')),
                "Stream IDs should only contain ASCII alphanumeric characters, colons, and hyphens"
            );
        }
    }
}
