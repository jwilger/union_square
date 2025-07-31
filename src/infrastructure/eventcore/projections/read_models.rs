//! Read models for common queries
//!
//! This module defines read model structs that represent denormalized views
//! of the event stream, optimized for specific query patterns.

use crate::domain::{
    llm::{ModelVersion, RequestId},
    metrics::{FScore, Precision, Recall, SampleCount, Timestamp},
    session::{ApplicationId, SessionId, SessionStatus},
    user::UserId,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::Duration;

/// Read model for session details with aggregated request information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionSummary {
    pub session_id: SessionId,
    pub user_id: UserId,
    pub application_id: ApplicationId,
    pub started_at: Timestamp,
    pub ended_at: Option<Timestamp>,
    pub status: SessionStatus,
    pub total_requests: usize,
    pub successful_requests: usize,
    pub failed_requests: usize,
    pub models_used: HashSet<ModelVersion>,
    pub average_response_time: Option<Duration>,
}

impl SessionSummary {
    /// Create a new session summary
    pub fn new(
        session_id: SessionId,
        user_id: UserId,
        application_id: ApplicationId,
        started_at: Timestamp,
    ) -> Self {
        Self {
            session_id,
            user_id,
            application_id,
            started_at,
            ended_at: None,
            status: SessionStatus::Active,
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            models_used: HashSet::new(),
            average_response_time: None,
        }
    }

    /// Update the summary with a new request
    pub fn add_request(&mut self, _request_id: &RequestId, model_version: &ModelVersion) {
        self.total_requests += 1;
        self.models_used.insert(model_version.clone());
    }

    /// Mark a request as completed
    pub fn complete_request(&mut self, _request_id: &RequestId, response_time: Duration) {
        self.successful_requests += 1;

        // Update average response time
        if let Some(current_avg) = self.average_response_time {
            // When successful_requests is 1, we just have the first response time
            if self.successful_requests == 1 {
                self.average_response_time = Some(response_time);
            } else {
                // Calculate new average: (old_avg * (n-1) + new_value) / n
                let prev_count = self.successful_requests - 1;
                let total_time = current_avg * prev_count as u32 + response_time;
                self.average_response_time = Some(total_time / self.successful_requests as u32);
            }
        } else {
            self.average_response_time = Some(response_time);
        }
    }

    /// Mark a request as failed
    pub fn fail_request(&mut self, _request_id: &RequestId) {
        self.failed_requests += 1;
    }

    /// End the session
    pub fn end_session(&mut self, ended_at: Timestamp, status: SessionStatus) {
        self.ended_at = Some(ended_at);
        self.status = status;
    }
}

/// Read model for user activity across all sessions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserActivityModel {
    pub user_id: UserId,
    pub total_sessions: usize,
    pub active_sessions: usize,
    pub total_requests: usize,
    pub first_activity: Timestamp,
    pub last_activity: Timestamp,
    pub applications_used: HashMap<ApplicationId, ApplicationUsage>,
    pub model_preferences: HashMap<ModelVersion, ModelUsage>,
}

/// Usage statistics for an application
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApplicationUsage {
    pub session_count: usize,
    pub request_count: usize,
    pub last_used: Timestamp,
}

/// Usage statistics for a model version
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelUsage {
    pub request_count: usize,
    pub last_used: Timestamp,
    pub average_response_time: Option<Duration>,
}

impl UserActivityModel {
    /// Create a new user activity model
    pub fn new(user_id: UserId) -> Self {
        let now = Timestamp::now();
        Self {
            user_id,
            total_sessions: 0,
            active_sessions: 0,
            total_requests: 0,
            first_activity: now,
            last_activity: now,
            applications_used: HashMap::new(),
            model_preferences: HashMap::new(),
        }
    }

    /// Update the model with a new session
    pub fn add_session(
        &mut self,
        _session_id: &SessionId,
        application_id: &ApplicationId,
        started_at: Timestamp,
    ) {
        self.total_sessions += 1;
        self.active_sessions += 1;
        self.last_activity = started_at;

        // Update application usage
        let app_usage = self
            .applications_used
            .entry(application_id.clone())
            .or_insert(ApplicationUsage {
                session_count: 0,
                request_count: 0,
                last_used: started_at,
            });
        app_usage.session_count += 1;
        app_usage.last_used = started_at;
    }

    /// Update the model with a new request
    pub fn add_request(&mut self, application_id: &ApplicationId, model_version: &ModelVersion) {
        self.total_requests += 1;
        self.last_activity = Timestamp::now();

        // Update application usage
        if let Some(app_usage) = self.applications_used.get_mut(application_id) {
            app_usage.request_count += 1;
            app_usage.last_used = self.last_activity;
        }

        // Update model preferences
        let model_usage = self
            .model_preferences
            .entry(model_version.clone())
            .or_insert(ModelUsage {
                request_count: 0,
                last_used: self.last_activity,
                average_response_time: None,
            });
        model_usage.request_count += 1;
        model_usage.last_used = self.last_activity;
    }
}

/// Read model for version performance metrics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VersionPerformanceModel {
    pub model_version: ModelVersion,
    pub total_requests: usize,
    pub successful_requests: usize,
    pub failed_requests: usize,
    pub average_response_time: Option<Duration>,
    pub f_scores: Vec<FScoreEntry>,
    pub sessions_used_in: HashSet<SessionId>,
    pub applications_used_in: HashSet<ApplicationId>,
}

/// F-score entry with metadata
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FScoreEntry {
    pub f_score: FScore,
    pub precision: Option<Precision>,
    pub recall: Option<Recall>,
    pub sample_count: SampleCount,
    pub calculated_at: Timestamp,
    pub session_id: SessionId,
}

impl VersionPerformanceModel {
    /// Create a new version performance model
    pub fn new(model_version: ModelVersion) -> Self {
        Self {
            model_version,
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            average_response_time: None,
            f_scores: Vec::new(),
            sessions_used_in: HashSet::new(),
            applications_used_in: HashSet::new(),
        }
    }

    /// Add performance metrics
    pub fn add_performance_metrics(
        &mut self,
        session_id: SessionId,
        f_score: FScore,
        precision: Option<Precision>,
        recall: Option<Recall>,
        sample_count: SampleCount,
        calculated_at: Timestamp,
    ) {
        self.f_scores.push(FScoreEntry {
            f_score,
            precision,
            recall,
            sample_count,
            calculated_at,
            session_id: session_id.clone(),
        });
        self.sessions_used_in.insert(session_id);
    }
}

/// Read model for application-level metrics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApplicationMetricsModel {
    pub application_id: ApplicationId,
    pub total_sessions: usize,
    pub active_sessions: usize,
    pub total_requests: usize,
    pub unique_users: HashSet<UserId>,
    pub model_versions: HashMap<ModelVersion, VersionMetrics>,
    pub session_durations: Vec<Duration>,
    /// Track session start times for duration calculation
    #[serde(skip)]
    session_start_times: HashMap<SessionId, Timestamp>,
}

/// Metrics for a specific version within an application
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VersionMetrics {
    pub request_count: usize,
    pub average_response_time: Option<Duration>,
    pub f_scores: Vec<FScore>,
}

impl ApplicationMetricsModel {
    /// Create a new application metrics model
    pub fn new(application_id: ApplicationId) -> Self {
        Self {
            application_id,
            total_sessions: 0,
            active_sessions: 0,
            total_requests: 0,
            unique_users: HashSet::new(),
            model_versions: HashMap::new(),
            session_durations: Vec::new(),
            session_start_times: HashMap::new(),
        }
    }

    /// Add a new session
    pub fn add_session(&mut self, session_id: &SessionId, user_id: &UserId, started_at: Timestamp) {
        self.total_sessions += 1;
        self.active_sessions += 1;
        self.unique_users.insert(user_id.clone());
        self.session_start_times
            .insert(session_id.clone(), started_at);
    }

    /// End a session
    pub fn end_session(&mut self, session_id: &SessionId, ended_at: Timestamp) {
        self.active_sessions = self.active_sessions.saturating_sub(1);

        // Calculate actual duration if we have the start time
        if let Some(started_at) = self.session_start_times.remove(session_id) {
            let started_dt = started_at.into_datetime();
            let ended_dt = ended_at.into_datetime();
            let duration = ended_dt.signed_duration_since(started_dt);
            if duration.num_milliseconds() > 0 {
                self.session_durations
                    .push(std::time::Duration::from_millis(
                        duration.num_milliseconds() as u64,
                    ));
            }
        }
    }

    /// Calculate average session duration
    pub fn average_session_duration(&self) -> Option<Duration> {
        if self.session_durations.is_empty() {
            None
        } else {
            let total_duration: Duration = self.session_durations.iter().sum();
            Some(total_duration / self.session_durations.len() as u32)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::llm::LlmProvider;
    use crate::domain::types::ModelId;

    #[test]
    fn test_session_summary_new() {
        let session_id = SessionId::generate();
        let user_id = UserId::generate();
        let app_id = ApplicationId::try_new("test-app".to_string()).unwrap();
        let started_at = Timestamp::now();

        let summary = SessionSummary::new(
            session_id.clone(),
            user_id.clone(),
            app_id.clone(),
            started_at,
        );

        assert_eq!(summary.session_id, session_id);
        assert_eq!(summary.user_id, user_id);
        assert_eq!(summary.application_id, app_id);
        assert_eq!(summary.started_at, started_at);
        assert_eq!(summary.total_requests, 0);
        assert!(summary.ended_at.is_none());
    }

    #[test]
    fn test_session_summary_add_request() {
        let session_id = SessionId::generate();
        let user_id = UserId::generate();
        let app_id = ApplicationId::try_new("test-app".to_string()).unwrap();
        let started_at = Timestamp::now();

        let mut summary = SessionSummary::new(session_id, user_id, app_id, started_at);

        let request_id = RequestId::generate();
        let model_version = ModelVersion {
            provider: LlmProvider::Anthropic,
            model_id: ModelId::try_new("claude-3".to_string()).unwrap(),
        };

        summary.add_request(&request_id, &model_version);

        assert_eq!(summary.total_requests, 1);
        assert!(summary.models_used.contains(&model_version));
    }

    #[test]
    fn test_average_response_time_calculation() {
        let session_id = SessionId::generate();
        let user_id = UserId::generate();
        let app_id = ApplicationId::try_new("test-app".to_string()).unwrap();
        let started_at = Timestamp::now();

        let mut summary = SessionSummary::new(session_id, user_id, app_id, started_at);
        let request_id = RequestId::generate();

        // First request - should not divide by zero
        summary.complete_request(&request_id, Duration::from_millis(100));
        assert_eq!(summary.successful_requests, 1);
        assert_eq!(
            summary.average_response_time,
            Some(Duration::from_millis(100))
        );

        // Second request - should average correctly
        let request_id2 = RequestId::generate();
        summary.complete_request(&request_id2, Duration::from_millis(200));
        assert_eq!(summary.successful_requests, 2);
        assert_eq!(
            summary.average_response_time,
            Some(Duration::from_millis(150))
        );

        // Third request - verify averaging continues to work
        let request_id3 = RequestId::generate();
        summary.complete_request(&request_id3, Duration::from_millis(300));
        assert_eq!(summary.successful_requests, 3);
        assert_eq!(
            summary.average_response_time,
            Some(Duration::from_millis(200))
        );
    }

    #[test]
    fn test_user_activity_model() {
        let user_id = UserId::generate();
        let mut model = UserActivityModel::new(user_id.clone());

        assert_eq!(model.user_id, user_id);
        assert_eq!(model.total_sessions, 0);
        assert_eq!(model.total_requests, 0);

        let session_id = SessionId::generate();
        let app_id = ApplicationId::try_new("test-app".to_string()).unwrap();
        let started_at = Timestamp::now();

        model.add_session(&session_id, &app_id, started_at);
        assert_eq!(model.total_sessions, 1);
        assert!(model.applications_used.contains_key(&app_id));
    }

    #[test]
    fn test_version_performance_model() {
        let model_version = ModelVersion {
            provider: LlmProvider::OpenAI,
            model_id: ModelId::try_new("gpt-4".to_string()).unwrap(),
        };

        let mut perf_model = VersionPerformanceModel::new(model_version.clone());
        assert_eq!(perf_model.model_version, model_version);
        assert_eq!(perf_model.total_requests, 0);

        let session_id = SessionId::generate();
        let f_score = FScore::try_new(0.85).unwrap();
        let precision = Some(Precision::try_new(0.90).unwrap());
        let recall = Some(Recall::try_new(0.80).unwrap());
        let sample_count = SampleCount::try_new(100).unwrap();
        let calculated_at = Timestamp::now();

        perf_model.add_performance_metrics(
            session_id.clone(),
            f_score,
            precision,
            recall,
            sample_count,
            calculated_at,
        );

        assert_eq!(perf_model.f_scores.len(), 1);
        assert!(perf_model.sessions_used_in.contains(&session_id));
    }

    #[test]
    fn test_application_metrics_model() {
        let app_id = ApplicationId::try_new("my-app".to_string()).unwrap();
        let mut model = ApplicationMetricsModel::new(app_id.clone());

        assert_eq!(model.application_id, app_id);
        assert_eq!(model.total_sessions, 0);

        let session_id = SessionId::generate();
        let user_id = UserId::generate();
        let started_at = Timestamp::now();

        model.add_session(&session_id, &user_id, started_at);
        assert_eq!(model.total_sessions, 1);
        assert!(model.unique_users.contains(&user_id));

        // End session after 5 minutes
        let ended_dt = started_at.into_datetime() + chrono::Duration::seconds(300);
        let ended_at = Timestamp::try_new(ended_dt).unwrap();
        model.end_session(&session_id, ended_at);
        assert_eq!(model.active_sessions, 0);
        assert_eq!(model.session_durations.len(), 1);
        assert_eq!(model.session_durations[0], Duration::from_secs(300));

        let avg_duration = model.average_session_duration();
        assert!(avg_duration.is_some());
        assert_eq!(avg_duration.unwrap(), Duration::from_secs(300));
    }
}
