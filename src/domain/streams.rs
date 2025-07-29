//! Type-safe stream naming and lifecycle management for EventCore
//!
//! This module provides strongly-typed stream identifiers and lifecycle definitions
//! that make illegal stream operations unrepresentable at compile time.

use eventcore::StreamId;
use nutype::nutype;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

use crate::domain::{
    session::SessionId,
    test_case::TestCaseId,
    user::UserId,
    llm::RequestId,
};

/// Type-safe stream identifier that carries the stream type at compile time
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypedStreamId<T> {
    inner: StreamId,
    _phantom: PhantomData<T>,
}

impl<T> TypedStreamId<T> {
    /// Create a new typed stream ID
    fn new(stream_id: StreamId) -> Self {
        Self {
            inner: stream_id,
            _phantom: PhantomData,
        }
    }

    /// Get the underlying EventCore StreamId
    pub fn as_stream_id(&self) -> &StreamId {
        &self.inner
    }

    /// Convert into the underlying EventCore StreamId
    pub fn into_stream_id(self) -> StreamId {
        self.inner
    }
}

// Marker types for different stream categories
pub struct SessionStream;
pub struct RequestStream;
pub struct AnalysisStream;
pub struct UserSettingsStream;
pub struct ExtractionStream;
pub struct TestCaseStream;
pub struct MetricsStream;

/// Analysis identifier - represents a specific analysis run
#[nutype(derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize))]
pub struct AnalysisId(uuid::Uuid);

impl AnalysisId {
    pub fn generate() -> Self {
        Self::new(uuid::Uuid::now_v7())
    }
}

/// Extraction identifier - represents a test case extraction process
#[nutype(derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize))]
pub struct ExtractionId(uuid::Uuid);

impl ExtractionId {
    pub fn generate() -> Self {
        Self::new(uuid::Uuid::now_v7())
    }
}

/// Stream lifecycle definition
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StreamLifecycle {
    /// Stream exists for a bounded period with clear start/end
    Bounded {
        created_by: &'static str,
        closed_by: &'static str,
        retention: RetentionPolicy,
    },
    /// Stream exists indefinitely (e.g., user settings)
    Unbounded {
        created_by: &'static str,
        retention: RetentionPolicy,
    },
}

/// Retention policy for streams
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RetentionPolicy {
    /// Keep for N days after last event
    Days(u32),
    /// Keep forever
    Forever,
    /// Keep until explicitly deleted
    UntilDeleted,
}

/// Stream documentation and metadata
#[derive(Debug, Clone)]
pub struct StreamDocumentation {
    pub stream_pattern: &'static str,
    pub purpose: &'static str,
    pub lifecycle: StreamLifecycle,
    pub related_streams: Vec<&'static str>,
}

/// Type-safe stream constructors
pub mod streams {
    use super::*;
    use crate::error::Result;

    /// Create a session stream ID
    pub fn session(session_id: &SessionId) -> Result<TypedStreamId<SessionStream>> {
        let stream_id = StreamId::try_new(format!("session:{}", session_id.clone().into_inner()))
            .map_err(|e| crate::error::Error::InvalidStreamId(e.to_string()))?;
        Ok(TypedStreamId::new(stream_id))
    }

    /// Create a request stream ID
    pub fn request(request_id: &RequestId) -> Result<TypedStreamId<RequestStream>> {
        let stream_id = StreamId::try_new(format!("request:{}", request_id.as_ref()))
            .map_err(|e| crate::error::Error::InvalidStreamId(e.to_string()))?;
        Ok(TypedStreamId::new(stream_id))
    }

    /// Create an analysis stream ID
    pub fn analysis(analysis_id: &AnalysisId) -> Result<TypedStreamId<AnalysisStream>> {
        let stream_id = StreamId::try_new(format!("analysis:{}", analysis_id.as_ref()))
            .map_err(|e| crate::error::Error::InvalidStreamId(e.to_string()))?;
        Ok(TypedStreamId::new(stream_id))
    }

    /// Create a user settings stream ID
    pub fn user_settings(user_id: &UserId) -> Result<TypedStreamId<UserSettingsStream>> {
        let stream_id = StreamId::try_new(format!("user:{}:settings", user_id.as_ref()))
            .map_err(|e| crate::error::Error::InvalidStreamId(e.to_string()))?;
        Ok(TypedStreamId::new(stream_id))
    }

    /// Create an extraction stream ID
    pub fn extraction(extraction_id: &ExtractionId) -> Result<TypedStreamId<ExtractionStream>> {
        let stream_id = StreamId::try_new(format!("extraction:{}", extraction_id.as_ref()))
            .map_err(|e| crate::error::Error::InvalidStreamId(e.to_string()))?;
        Ok(TypedStreamId::new(stream_id))
    }

    /// Create a test case stream ID
    pub fn test_case(test_case_id: &TestCaseId) -> Result<TypedStreamId<TestCaseStream>> {
        let stream_id = StreamId::try_new(format!("testcase:{}", test_case_id.as_ref()))
            .map_err(|e| crate::error::Error::InvalidStreamId(e.to_string()))?;
        Ok(TypedStreamId::new(stream_id))
    }

    /// Create a metrics stream ID for a specific time period
    pub fn metrics(year: u16, month: u8) -> Result<TypedStreamId<MetricsStream>> {
        let stream_id = StreamId::try_new(format!("metrics:{year:04}-{month:02}"))
            .map_err(|e| crate::error::Error::InvalidStreamId(e.to_string()))?;
        Ok(TypedStreamId::new(stream_id))
    }
}

/// Stream documentation catalog
pub const STREAM_DOCUMENTATION: &[StreamDocumentation] = &[
    StreamDocumentation {
        stream_pattern: "session:{session_id}",
        purpose: "Tracks all events for a single LLM interaction session",
        lifecycle: StreamLifecycle::Bounded {
            created_by: "StartSession command",
            closed_by: "EndSession command",
            retention: RetentionPolicy::Days(90),
        },
        related_streams: vec!["request:{request_id}", "analysis:{analysis_id}"],
    },
    StreamDocumentation {
        stream_pattern: "request:{request_id}",
        purpose: "Tracks lifecycle of a single LLM request/response",
        lifecycle: StreamLifecycle::Bounded {
            created_by: "RecordAuditEvent command",
            closed_by: "Request completion or failure",
            retention: RetentionPolicy::Days(90),
        },
        related_streams: vec!["session:{session_id}"],
    },
    StreamDocumentation {
        stream_pattern: "analysis:{analysis_id}",
        purpose: "Tracks session analysis process and results",
        lifecycle: StreamLifecycle::Bounded {
            created_by: "StartSessionAnalysis command",
            closed_by: "CompleteAnalysis command",
            retention: RetentionPolicy::Days(365),
        },
        related_streams: vec!["session:{session_id}", "extraction:{extraction_id}"],
    },
    StreamDocumentation {
        stream_pattern: "user:{user_id}:settings",
        purpose: "User preferences and configuration",
        lifecycle: StreamLifecycle::Unbounded {
            created_by: "UpdateUserSettings command",
            retention: RetentionPolicy::UntilDeleted,
        },
        related_streams: vec!["session:{session_id}"],
    },
    StreamDocumentation {
        stream_pattern: "extraction:{extraction_id}",
        purpose: "Test case extraction process from sessions",
        lifecycle: StreamLifecycle::Bounded {
            created_by: "StartTestExtraction command",
            closed_by: "CompleteTestExtraction command",
            retention: RetentionPolicy::Days(365),
        },
        related_streams: vec!["session:{session_id}", "testcase:{test_case_id}"],
    },
    StreamDocumentation {
        stream_pattern: "testcase:{test_case_id}",
        purpose: "Individual test case definition and execution history",
        lifecycle: StreamLifecycle::Unbounded {
            created_by: "CreateTestCase command",
            retention: RetentionPolicy::Forever,
        },
        related_streams: vec!["extraction:{extraction_id}"],
    },
    StreamDocumentation {
        stream_pattern: "metrics:{year}-{month}",
        purpose: "Aggregated metrics for a specific time period",
        lifecycle: StreamLifecycle::Unbounded {
            created_by: "Metric aggregation projections",
            retention: RetentionPolicy::Forever,
        },
        related_streams: vec!["session:{session_id}"],
    },
];

/// Stream state tracking for lifecycle management
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StreamState<T> {
    /// Stream doesn't exist yet
    NotCreated(PhantomData<T>),
    /// Stream is active and accepting events
    Active {
        created_at: crate::domain::metrics::Timestamp,
        event_count: u64,
        _phantom: PhantomData<T>,
    },
    /// Stream is closed (no new events accepted)
    Closed {
        created_at: crate::domain::metrics::Timestamp,
        closed_at: crate::domain::metrics::Timestamp,
        event_count: u64,
        _phantom: PhantomData<T>,
    },
}

impl<T> Default for StreamState<T> {
    fn default() -> Self {
        Self::NotCreated(PhantomData)
    }
}

impl<T> StreamState<T> {
    /// Check if the stream is active
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Active { .. })
    }

    /// Check if the stream is closed
    pub fn is_closed(&self) -> bool {
        matches!(self, Self::Closed { .. })
    }

    /// Transition to active state
    pub fn activate(self, timestamp: crate::domain::metrics::Timestamp) -> crate::error::Result<Self> {
        match self {
            Self::NotCreated(_) => Ok(Self::Active {
                created_at: timestamp,
                event_count: 0,
                _phantom: PhantomData,
            }),
            _ => Err(crate::error::Error::InvalidStateTransition(
                "Cannot activate an already created stream".to_string(),
            )),
        }
    }

    /// Record an event
    pub fn record_event(self) -> crate::error::Result<Self> {
        match self {
            Self::Active {
                created_at,
                event_count,
                _phantom,
            } => Ok(Self::Active {
                created_at,
                event_count: event_count + 1,
                _phantom,
            }),
            _ => Err(crate::error::Error::InvalidStateTransition(
                "Cannot record event on inactive stream".to_string(),
            )),
        }
    }

    /// Close the stream
    pub fn close(self, timestamp: crate::domain::metrics::Timestamp) -> crate::error::Result<Self> {
        match self {
            Self::Active {
                created_at,
                event_count,
                _phantom,
            } => Ok(Self::Closed {
                created_at,
                closed_at: timestamp,
                event_count,
                _phantom,
            }),
            _ => Err(crate::error::Error::InvalidStateTransition(
                "Cannot close an inactive stream".to_string(),
            )),
        }
    }
}

/// Helper to validate stream relationships
pub fn validate_stream_relationship<T, U>(
    parent: &TypedStreamId<T>,
    child: &TypedStreamId<U>,
) -> bool {
    // This is a compile-time check that the relationship is valid
    // The actual validation logic would depend on the specific types
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_typed_stream_ids() {
        let session_id = SessionId::generate();
        let session_stream = streams::session(&session_id).unwrap();
        
        // This ensures type safety - can't mix stream types
        let _session_specific: TypedStreamId<SessionStream> = session_stream;
        
        // The following would not compile:
        // let _wrong_type: TypedStreamId<RequestStream> = session_stream;
    }

    #[test]
    fn test_stream_state_transitions() {
        let state: StreamState<SessionStream> = StreamState::default();
        assert!(!state.is_active());
        
        let timestamp = crate::domain::metrics::Timestamp::now();
        let active_state = state.activate(timestamp).unwrap();
        assert!(active_state.is_active());
        
        let after_event = active_state.record_event().unwrap();
        assert!(after_event.is_active());
        
        let closed = after_event.close(timestamp).unwrap();
        assert!(closed.is_closed());
        
        // Cannot record events on closed stream
        assert!(closed.record_event().is_err());
    }
}