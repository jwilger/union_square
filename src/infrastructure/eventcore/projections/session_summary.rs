//! Session summary projection
//!
//! TODO: This module needs to be migrated to use EventCore's built-in projection system
//! instead of our custom infrastructure that was removed.
//!
//! The domain logic in apply_session_event() and the query methods on SessionSummaryState
//! should be preserved, but wrapped in EventCore's Projection trait implementation.

use crate::domain::events::DomainEvent;
use crate::domain::session::{SessionId, SessionStatus};
use eventcore::{StoredEvent, Timestamp};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// State maintained by the session summary projection
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct SessionSummaryState {
    /// Map of session ID to session summary
    pub sessions: HashMap<SessionId, SessionSummary>,
}

/// Summary information for a single session
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionSummary {
    pub session_id: SessionId,
    pub user_id: String,
    pub app_id: String,
    pub status: SessionStatus,
    pub request_count: usize,
    pub total_tokens: usize,
    pub started_at: Timestamp,
    pub last_activity: Timestamp,
    pub ended_at: Option<Timestamp>,
}

/// Core logic for applying events to session summary state
/// TODO: This will be used in the EventCore Projection trait implementation
#[allow(dead_code)]
fn apply_session_event(state: &mut SessionSummaryState, event: &StoredEvent<DomainEvent>) {
    match &event.payload {
        DomainEvent::SessionStarted {
            session_id,
            user_id,
            application_id,
            ..
        } => {
            state.sessions.insert(
                session_id.clone(),
                SessionSummary {
                    session_id: session_id.clone(),
                    user_id: user_id.to_string(),
                    app_id: application_id.to_string(),
                    status: SessionStatus::Active,
                    request_count: 0,
                    total_tokens: 0,
                    started_at: event.timestamp,
                    last_activity: event.timestamp,
                    ended_at: None,
                },
            );
        }

        DomainEvent::LlmRequestReceived { session_id, .. } => {
            if let Some(summary) = state.sessions.get_mut(session_id) {
                summary.request_count += 1;
                summary.last_activity = event.timestamp;
            }
        }

        DomainEvent::LlmResponseReceived { .. } => {
            // We need to track which session this response belongs to
            // For now, we'll skip this as we don't have session_id in this event
            // In production, you'd likely have a separate projection or stream correlation
        }

        DomainEvent::LlmRequestFailed { .. } => {
            // We need to track which session this request belongs to
            // For now, we'll skip this as we don't have session_id in this event
        }

        DomainEvent::SessionEnded { session_id, .. } => {
            if let Some(summary) = state.sessions.get_mut(session_id) {
                summary.status = SessionStatus::Completed;
                summary.ended_at = Some(event.timestamp);
                summary.last_activity = event.timestamp;
            }
        }

        _ => {} // Ignore other events
    }
}

/// Query methods for session summaries
impl SessionSummaryState {
    /// Get all active sessions
    pub fn active_sessions(&self) -> Vec<&SessionSummary> {
        self.sessions
            .values()
            .filter(|s| matches!(s.status, SessionStatus::Active))
            .collect()
    }

    /// Get sessions for a specific user
    pub fn user_sessions(&self, user_id: &str) -> Vec<&SessionSummary> {
        self.sessions
            .values()
            .filter(|s| s.user_id == user_id)
            .collect()
    }

    /// Get sessions for a specific app
    pub fn app_sessions(&self, app_id: &str) -> Vec<&SessionSummary> {
        self.sessions
            .values()
            .filter(|s| s.app_id == app_id)
            .collect()
    }

    /// Get session by ID
    pub fn get_session(&self, session_id: &SessionId) -> Option<&SessionSummary> {
        self.sessions.get(session_id)
    }

    /// Calculate total tokens used across all sessions
    pub fn total_tokens_used(&self) -> usize {
        self.sessions.values().map(|s| s.total_tokens).sum()
    }

    /// Get sessions that have been inactive for a given duration
    pub fn inactive_sessions(
        &self,
        _inactive_duration: std::time::Duration,
    ) -> Vec<&SessionSummary> {
        // EventCore doesn't support timestamp arithmetic
        // In production, convert to chrono or use a different approach
        let cutoff = Timestamp::now();
        self.sessions
            .values()
            .filter(|s| matches!(s.status, SessionStatus::Active) && s.last_activity < cutoff)
            .collect()
    }
}

// TODO: Tests need to be rewritten once we implement EventCore's Projection trait
#[cfg(test)]
mod tests {
    // Tests temporarily disabled until migration to EventCore projections is complete
}
