//! Type-safe session tracking with compile-time guarantees
//!
//! This module provides session tracking types that make illegal states
//! unrepresentable, ensuring sessions can only transition through valid states.

use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

use crate::domain::{
    llm::{ModelVersion, RequestId},
    metrics::{Timestamp, Duration},
    session::{SessionId, ApplicationId},
    types::{ErrorMessage, Tag},
    user::UserId,
};

/// Session tracking with compile-time state guarantees
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionTracker<State> {
    pub session_id: SessionId,
    pub user_id: UserId,
    pub application_id: ApplicationId,
    pub created_at: Timestamp,
    pub state_data: StateData<State>,
    _state: PhantomData<State>,
}

/// State-specific data that can only exist in certain states
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StateData<State> {
    /// Initial state - no additional data
    Initial(PhantomData<State>),
    /// Active state - tracks requests and metadata
    Active {
        started_at: Timestamp,
        requests: Vec<TrackedRequest>,
        tags: Vec<Tag>,
        _state: PhantomData<State>,
    },
    /// Completed state - includes final metrics
    Completed {
        started_at: Timestamp,
        completed_at: Timestamp,
        final_metrics: SessionMetrics,
        _state: PhantomData<State>,
    },
    /// Failed state - includes error information
    Failed {
        started_at: Timestamp,
        failed_at: Timestamp,
        error: ErrorMessage,
        partial_metrics: Option<SessionMetrics>,
        _state: PhantomData<State>,
    },
}

// Phantom types for state tracking
pub struct Created;
pub struct Active;
pub struct Completed;
pub struct Failed;

/// Tracked request within a session
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrackedRequest {
    pub request_id: RequestId,
    pub model_version: ModelVersion,
    pub received_at: Timestamp,
    pub status: RequestStatus,
    pub response_time: Option<Duration>,
}

/// Request status within a session
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RequestStatus {
    Received,
    Processing,
    Completed { response_size_bytes: u64 },
    Failed { error: ErrorMessage },
    Cancelled,
}

/// Session metrics calculated at completion
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionMetrics {
    pub total_requests: u32,
    pub successful_requests: u32,
    pub failed_requests: u32,
    pub cancelled_requests: u32,
    pub total_duration: Duration,
    pub average_response_time: Duration,
    pub total_tokens_used: Option<u32>,
    pub total_cost_usd: Option<f64>,
    pub models_used: Vec<ModelVersion>,
}

impl SessionTracker<Created> {
    /// Create a new session tracker
    pub fn new(
        session_id: SessionId,
        user_id: UserId,
        application_id: ApplicationId,
        created_at: Timestamp,
    ) -> Self {
        Self {
            session_id,
            user_id,
            application_id,
            created_at,
            state_data: StateData::Initial(PhantomData),
            _state: PhantomData,
        }
    }

    /// Activate the session - only possible from Created state
    pub fn activate(self, started_at: Timestamp) -> SessionTracker<Active> {
        SessionTracker {
            session_id: self.session_id,
            user_id: self.user_id,
            application_id: self.application_id,
            created_at: self.created_at,
            state_data: StateData::Active {
                started_at,
                requests: Vec::new(),
                tags: Vec::new(),
                _state: PhantomData,
            },
            _state: PhantomData,
        }
    }
}

impl SessionTracker<Active> {
    /// Add a request to the active session
    pub fn add_request(mut self, request: TrackedRequest) -> Result<Self, SessionError> {
        if let StateData::Active { ref mut requests, .. } = self.state_data {
            // Prevent duplicate request IDs
            if requests.iter().any(|r| r.request_id == request.request_id) {
                return Err(SessionError::DuplicateRequest(request.request_id));
            }
            requests.push(request);
            Ok(self)
        } else {
            unreachable!("StateData should always be Active for SessionTracker<Active>")
        }
    }

    /// Add a tag to the session
    pub fn add_tag(mut self, tag: Tag) -> Result<Self, SessionError> {
        if let StateData::Active { ref mut tags, .. } = self.state_data {
            // Prevent duplicate tags
            if !tags.contains(&tag) {
                tags.push(tag);
            }
            Ok(self)
        } else {
            unreachable!("StateData should always be Active for SessionTracker<Active>")
        }
    }

    /// Update a request status
    pub fn update_request_status(
        mut self,
        request_id: RequestId,
        new_status: RequestStatus,
        response_time: Option<Duration>,
    ) -> Result<Self, SessionError> {
        if let StateData::Active { ref mut requests, .. } = self.state_data {
            if let Some(request) = requests.iter_mut().find(|r| r.request_id == request_id) {
                request.status = new_status;
                if let Some(duration) = response_time {
                    request.response_time = Some(duration);
                }
                Ok(self)
            } else {
                Err(SessionError::RequestNotFound(request_id))
            }
        } else {
            unreachable!("StateData should always be Active for SessionTracker<Active>")
        }
    }

    /// Complete the session - calculates final metrics
    pub fn complete(self, completed_at: Timestamp) -> Result<SessionTracker<Completed>, SessionError> {
        if let StateData::Active { started_at, requests, .. } = self.state_data {
            let metrics = calculate_session_metrics(&requests, started_at, completed_at)?;
            
            Ok(SessionTracker {
                session_id: self.session_id,
                user_id: self.user_id,
                application_id: self.application_id,
                created_at: self.created_at,
                state_data: StateData::Completed {
                    started_at,
                    completed_at,
                    final_metrics: metrics,
                    _state: PhantomData,
                },
                _state: PhantomData,
            })
        } else {
            unreachable!("StateData should always be Active for SessionTracker<Active>")
        }
    }

    /// Fail the session - preserves partial state
    pub fn fail(
        self,
        failed_at: Timestamp,
        error: ErrorMessage,
    ) -> Result<SessionTracker<Failed>, SessionError> {
        if let StateData::Active { started_at, requests, .. } = self.state_data {
            // Calculate partial metrics if possible
            let partial_metrics = if !requests.is_empty() {
                calculate_session_metrics(&requests, started_at, failed_at).ok()
            } else {
                None
            };

            Ok(SessionTracker {
                session_id: self.session_id,
                user_id: self.user_id,
                application_id: self.application_id,
                created_at: self.created_at,
                state_data: StateData::Failed {
                    started_at,
                    failed_at,
                    error,
                    partial_metrics,
                    _state: PhantomData,
                },
                _state: PhantomData,
            })
        } else {
            unreachable!("StateData should always be Active for SessionTracker<Active>")
        }
    }

    /// Get current request count
    pub fn request_count(&self) -> u32 {
        if let StateData::Active { requests, .. } = &self.state_data {
            requests.len() as u32
        } else {
            0
        }
    }

    /// Check if session has any failed requests
    pub fn has_failed_requests(&self) -> bool {
        if let StateData::Active { requests, .. } = &self.state_data {
            requests.iter().any(|r| matches!(r.status, RequestStatus::Failed { .. }))
        } else {
            false
        }
    }

    /// Get unique models used in this session
    pub fn models_used(&self) -> Vec<ModelVersion> {
        if let StateData::Active { requests, .. } = &self.state_data {
            let mut models: Vec<_> = requests
                .iter()
                .map(|r| r.model_version.clone())
                .collect();
            models.sort_by(|a, b| {
                a.provider.as_str().cmp(&b.provider.as_str())
                    .then_with(|| a.model_id.as_ref().cmp(b.model_id.as_ref()))
            });
            models.dedup();
            models
        } else {
            Vec::new()
        }
    }
}

impl SessionTracker<Completed> {
    /// Get the final metrics (only available in completed state)
    pub fn final_metrics(&self) -> &SessionMetrics {
        if let StateData::Completed { final_metrics, .. } = &self.state_data {
            final_metrics
        } else {
            unreachable!("StateData should always be Completed for SessionTracker<Completed>")
        }
    }

    /// Get session duration
    pub fn duration(&self) -> Duration {
        if let StateData::Completed { started_at, completed_at, .. } = &self.state_data {
            Duration::between(*started_at, *completed_at)
        } else {
            unreachable!("StateData should always be Completed for SessionTracker<Completed>")
        }
    }

    /// Check if session was successful (no failed requests)
    pub fn was_successful(&self) -> bool {
        self.final_metrics().failed_requests == 0
    }
}

impl SessionTracker<Failed> {
    /// Get the error that caused the failure
    pub fn error(&self) -> &ErrorMessage {
        if let StateData::Failed { error, .. } = &self.state_data {
            error
        } else {
            unreachable!("StateData should always be Failed for SessionTracker<Failed>")
        }
    }

    /// Get partial metrics if available
    pub fn partial_metrics(&self) -> Option<&SessionMetrics> {
        if let StateData::Failed { partial_metrics, .. } = &self.state_data {
            partial_metrics.as_ref()
        } else {
            unreachable!("StateData should always be Failed for SessionTracker<Failed>")
        }
    }
}

/// Session tracking errors
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SessionError {
    #[error("Duplicate request ID: {0:?}")]
    DuplicateRequest(RequestId),

    #[error("Request not found: {0:?}")]
    RequestNotFound(RequestId),

    #[error("Invalid state transition: {0}")]
    InvalidTransition(String),

    #[error("Metrics calculation failed: {0}")]
    MetricsCalculationFailed(String),
}

/// Calculate session metrics from requests
fn calculate_session_metrics(
    requests: &[TrackedRequest],
    started_at: Timestamp,
    ended_at: Timestamp,
) -> Result<SessionMetrics, SessionError> {
    let total_requests = requests.len() as u32;
    let successful_requests = requests
        .iter()
        .filter(|r| matches!(r.status, RequestStatus::Completed { .. }))
        .count() as u32;
    let failed_requests = requests
        .iter()
        .filter(|r| matches!(r.status, RequestStatus::Failed { .. }))
        .count() as u32;
    let cancelled_requests = requests
        .iter()
        .filter(|r| matches!(r.status, RequestStatus::Cancelled))
        .count() as u32;

    let total_duration = Duration::between(started_at, ended_at);

    // Calculate average response time from completed requests
    let response_times: Vec<_> = requests
        .iter()
        .filter_map(|r| r.response_time)
        .collect();

    let average_response_time = if response_times.is_empty() {
        Duration::zero()
    } else {
        let total_ms: u64 = response_times.iter().map(|d| d.as_millis()).sum();
        Duration::from_millis(total_ms / response_times.len() as u64)
    };

    // Get unique models used
    let mut models_used: Vec<_> = requests
        .iter()
        .map(|r| r.model_version.clone())
        .collect();
    models_used.sort_by(|a, b| {
        a.provider.as_str().cmp(&b.provider.as_str())
            .then_with(|| a.model_id.as_ref().cmp(b.model_id.as_ref()))
    });
    models_used.dedup();

    Ok(SessionMetrics {
        total_requests,
        successful_requests,
        failed_requests,
        cancelled_requests,
        total_duration,
        average_response_time,
        total_tokens_used: None, // Would be calculated from actual response data
        total_cost_usd: None,    // Would be calculated from pricing data
        models_used,
    })
}

/// Session collection for managing multiple sessions
#[derive(Debug, Clone)]
pub struct SessionCollection<State> {
    sessions: Vec<SessionTracker<State>>,
}

impl<State> SessionCollection<State> {
    /// Create a new empty collection
    pub fn new() -> Self {
        Self {
            sessions: Vec::new(),
        }
    }

    /// Add a session to the collection
    pub fn add_session(&mut self, session: SessionTracker<State>) {
        self.sessions.push(session);
    }

    /// Find a session by ID
    pub fn find_by_id(&self, session_id: &SessionId) -> Option<&SessionTracker<State>> {
        self.sessions.iter().find(|s| &s.session_id == session_id)
    }

    /// Get all sessions
    pub fn all(&self) -> &[SessionTracker<State>] {
        &self.sessions
    }

    /// Count of sessions
    pub fn len(&self) -> usize {
        self.sessions.len()
    }

    /// Check if collection is empty
    pub fn is_empty(&self) -> bool {
        self.sessions.is_empty()
    }
}

impl<State> Default for SessionCollection<State> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        llm::{LlmProvider, ModelVersion},
        types::{ModelId, ErrorMessage},
    };

    #[test]
    fn test_session_state_transitions() {
        let session_id = SessionId::generate();
        let user_id = UserId::generate();
        let app_id = ApplicationId::try_new("test-app".to_string()).unwrap();
        let created_at = Timestamp::now();

        // Create session
        let session = SessionTracker::new(session_id.clone(), user_id, app_id, created_at);
        assert_eq!(session.session_id, session_id);

        // Activate session
        let started_at = Timestamp::now();
        let active_session = session.activate(started_at);
        assert_eq!(active_session.request_count(), 0);

        // Add request
        let request_id = RequestId::new();
        let model_version = ModelVersion {
            provider: LlmProvider::Other(
                crate::domain::config_types::ProviderName::try_new("test".to_string()).unwrap()
            ),
            model_id: ModelId::try_new("test-model".to_string()).unwrap(),
        };
        
        let request = TrackedRequest {
            request_id: request_id.clone(),
            model_version,
            received_at: Timestamp::now(),
            status: RequestStatus::Received,
            response_time: None,
        };

        let active_session = active_session.add_request(request).unwrap();
        assert_eq!(active_session.request_count(), 1);

        // Update request status
        let active_session = active_session
            .update_request_status(
                request_id,
                RequestStatus::Completed { response_size_bytes: 1024 },
                Some(Duration::from_millis(250)),
            )
            .unwrap();

        // Complete session
        let completed_at = Timestamp::now();
        let completed_session = active_session.complete(completed_at).unwrap();
        
        let metrics = completed_session.final_metrics();
        assert_eq!(metrics.total_requests, 1);
        assert_eq!(metrics.successful_requests, 1);
        assert_eq!(metrics.failed_requests, 0);
        assert!(completed_session.was_successful());
    }

    #[test]
    fn test_session_failure() {
        let session_id = SessionId::generate();
        let user_id = UserId::generate();
        let app_id = ApplicationId::try_new("test-app".to_string()).unwrap();
        let created_at = Timestamp::now();

        let session = SessionTracker::new(session_id, user_id, app_id, created_at)
            .activate(Timestamp::now());

        let error = ErrorMessage::try_new("Test error".to_string()).unwrap();
        let failed_session = session.fail(Timestamp::now(), error.clone()).unwrap();

        assert_eq!(failed_session.error(), &error);
        assert!(failed_session.partial_metrics().is_none()); // No requests were added
    }

    #[test]
    fn test_duplicate_request_prevention() {
        let session_id = SessionId::generate();
        let user_id = UserId::generate();
        let app_id = ApplicationId::try_new("test-app".to_string()).unwrap();
        let created_at = Timestamp::now();

        let session = SessionTracker::new(session_id, user_id, app_id, created_at)
            .activate(Timestamp::now());

        let request_id = RequestId::new();
        let model_version = ModelVersion {
            provider: LlmProvider::Other(
                crate::domain::config_types::ProviderName::try_new("test".to_string()).unwrap()
            ),
            model_id: ModelId::try_new("test-model".to_string()).unwrap(),
        };
        
        let request = TrackedRequest {
            request_id: request_id.clone(),
            model_version: model_version.clone(),
            received_at: Timestamp::now(),
            status: RequestStatus::Received,
            response_time: None,
        };

        let session = session.add_request(request.clone()).unwrap();
        
        // Adding the same request again should fail
        let result = session.add_request(request);
        assert!(matches!(result, Err(SessionError::DuplicateRequest(_))));
    }

    #[test]
    fn test_session_collection() {
        let mut collection: SessionCollection<Active> = SessionCollection::new();
        assert!(collection.is_empty());

        let session_id = SessionId::generate();
        let user_id = UserId::generate();
        let app_id = ApplicationId::try_new("test-app".to_string()).unwrap();
        let created_at = Timestamp::now();

        let session = SessionTracker::new(session_id.clone(), user_id, app_id, created_at)
            .activate(Timestamp::now());

        collection.add_session(session);
        assert_eq!(collection.len(), 1);
        assert!(collection.find_by_id(&session_id).is_some());
    }
}