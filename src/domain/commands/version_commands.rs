//! EventCore commands for version tracking
//!
//! These commands represent the core operations for tracking model versions
//! throughout the system. They are designed to work with EventCore but
//! currently use a simplified implementation until the full EventCore
//! integration is complete.

use serde::{Deserialize, Serialize};

use crate::domain::{
    events::DomainEvent,
    llm::ModelVersion,
    session::SessionId,
    version::{TrackedVersion, VersionChangeId},
};

/// Command to record the usage of a model version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordVersionUsage {
    pub session_id: SessionId,
    pub model_version: ModelVersion,
}

impl RecordVersionUsage {
    pub fn new(session_id: SessionId, model_version: ModelVersion) -> Self {
        Self {
            session_id,
            model_version,
        }
    }

    /// Execute the command and return resulting events
    pub fn execute(&self, state: &VersionState) -> Vec<DomainEvent> {
        let mut events = Vec::new();

        // Check if this is the first time we've seen this version
        let is_first_seen = !state
            .tracked_versions
            .iter()
            .any(|v| v.version == self.model_version);

        if is_first_seen {
            events.push(DomainEvent::VersionFirstSeen {
                model_version: self.model_version.clone(),
                session_id: self.session_id.clone(),
                first_seen_at: chrono::Utc::now(),
            });
        }

        // Always record usage
        events.push(DomainEvent::VersionUsageRecorded {
            model_version: self.model_version.clone(),
            session_id: self.session_id.clone(),
            recorded_at: chrono::Utc::now(),
        });

        events
    }
}

/// State for version tracking
#[derive(Debug, Default, Clone)]
pub struct VersionState {
    pub tracked_versions: Vec<TrackedVersion>,
}

impl VersionState {
    /// Apply an event to update the state
    pub fn apply(&mut self, event: &DomainEvent) {
        match event {
            DomainEvent::VersionFirstSeen { model_version, .. } => {
                let tracked = TrackedVersion::new(model_version.clone());
                self.tracked_versions.push(tracked);
            }
            DomainEvent::VersionUsageRecorded { model_version, .. } => {
                // Find and update the tracked version
                if let Some(tracked) = self
                    .tracked_versions
                    .iter_mut()
                    .find(|v| v.version == *model_version)
                {
                    tracked.record_usage();
                }
            }
            DomainEvent::VersionDeactivated { model_version, .. } => {
                if let Some(tracked) = self
                    .tracked_versions
                    .iter_mut()
                    .find(|v| v.version == *model_version)
                {
                    tracked.deactivate();
                }
            }
            _ => {} // Ignore other events
        }
    }
}

/// Command to record a version change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordVersionChange {
    pub session_id: SessionId,
    pub from_version: ModelVersion,
    pub to_version: ModelVersion,
    pub reason: Option<String>,
}

impl RecordVersionChange {
    pub fn new(
        session_id: SessionId,
        from_version: ModelVersion,
        to_version: ModelVersion,
        reason: Option<String>,
    ) -> Self {
        Self {
            session_id,
            from_version,
            to_version,
            reason,
        }
    }

    /// Execute the command and return resulting events
    pub fn execute(&self) -> Vec<DomainEvent> {
        let change_type = self.from_version.compare(&self.to_version);
        let change_id = VersionChangeId::generate();

        vec![DomainEvent::VersionChanged {
            change_id,
            session_id: self.session_id.clone(),
            from_version: self.from_version.clone(),
            to_version: self.to_version.clone(),
            change_type,
            reason: self.reason.clone(),
            changed_at: chrono::Utc::now(),
        }]
    }
}

/// Command to deactivate a version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeactivateVersion {
    pub model_version: ModelVersion,
    pub reason: Option<String>,
}

impl DeactivateVersion {
    pub fn new(model_version: ModelVersion, reason: Option<String>) -> Self {
        Self {
            model_version,
            reason,
        }
    }

    /// Execute the command and return resulting events
    pub fn execute(&self, state: &VersionState) -> Result<Vec<DomainEvent>, String> {
        // Check if version exists and is active
        let version_exists = state
            .tracked_versions
            .iter()
            .any(|v| v.version == self.model_version && v.is_active);

        if !version_exists {
            return Err("Version not found or already deactivated".into());
        }

        Ok(vec![DomainEvent::VersionDeactivated {
            model_version: self.model_version.clone(),
            reason: self.reason.clone(),
            deactivated_at: chrono::Utc::now(),
        }])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_version_usage_first_seen() {
        let session_id = SessionId::generate();
        let model_version = ModelVersion {
            provider: crate::domain::llm::LlmProvider::OpenAI,
            model_id: "gpt-4-turbo".to_string(),
        };

        let command = RecordVersionUsage::new(session_id, model_version);
        let state = VersionState::default();
        let events = command.execute(&state);

        assert_eq!(events.len(), 2); // VersionFirstSeen and VersionUsageRecorded
        assert!(matches!(events[0], DomainEvent::VersionFirstSeen { .. }));
        assert!(matches!(
            events[1],
            DomainEvent::VersionUsageRecorded { .. }
        ));
    }

    #[test]
    fn test_record_version_usage_existing() {
        let session_id = SessionId::generate();
        let model_version = ModelVersion {
            provider: crate::domain::llm::LlmProvider::OpenAI,
            model_id: "gpt-4-turbo".to_string(),
        };

        // Create state with existing version
        let mut state = VersionState::default();
        state.apply(&DomainEvent::VersionFirstSeen {
            model_version: model_version.clone(),
            session_id: session_id.clone(),
            first_seen_at: chrono::Utc::now(),
        });

        let command = RecordVersionUsage::new(session_id, model_version);
        let events = command.execute(&state);

        assert_eq!(events.len(), 1); // Only VersionUsageRecorded
        assert!(matches!(
            events[0],
            DomainEvent::VersionUsageRecorded { .. }
        ));
    }

    #[test]
    fn test_record_version_change() {
        let session_id = SessionId::generate();
        let from_version = ModelVersion {
            provider: crate::domain::llm::LlmProvider::OpenAI,
            model_id: "gpt-3.5-turbo".to_string(),
        };
        let to_version = ModelVersion {
            provider: crate::domain::llm::LlmProvider::OpenAI,
            model_id: "gpt-4-turbo".to_string(),
        };

        let command = RecordVersionChange::new(
            session_id,
            from_version,
            to_version,
            Some("Performance upgrade".to_string()),
        );
        let events = command.execute();

        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], DomainEvent::VersionChanged { .. }));
    }

    #[test]
    fn test_deactivate_version() {
        let session_id = SessionId::generate();
        let model_version = ModelVersion {
            provider: crate::domain::llm::LlmProvider::OpenAI,
            model_id: "gpt-4-deprecated".to_string(),
        };

        // Create state with existing active version
        let mut state = VersionState::default();
        state.apply(&DomainEvent::VersionFirstSeen {
            model_version: model_version.clone(),
            session_id,
            first_seen_at: chrono::Utc::now(),
        });

        let command = DeactivateVersion::new(model_version, Some("Model deprecated".to_string()));
        let result = command.execute(&state);

        assert!(result.is_ok());
        let events = result.unwrap();
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], DomainEvent::VersionDeactivated { .. }));
    }

    #[test]
    fn test_deactivate_nonexistent_version() {
        let model_version = ModelVersion {
            provider: crate::domain::llm::LlmProvider::OpenAI,
            model_id: "gpt-4-nonexistent".to_string(),
        };

        let state = VersionState::default();
        let command = DeactivateVersion::new(model_version, None);
        let result = command.execute(&state);

        assert!(result.is_err());
    }
}
