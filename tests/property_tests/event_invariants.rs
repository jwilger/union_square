//! Property-based tests for domain invariants
//!
//! These tests use property-based testing to verify that domain invariants
//! hold across all possible valid event sequences and system states.

use proptest::prelude::*;
use std::collections::{HashMap, HashSet};
use union_square::domain::{
    events::DomainEvent,
    llm::{ModelVersion, RequestId},
    metrics::Timestamp,
    session::{ApplicationId, SessionId, SessionStatus},
    types::{ErrorMessage, Tag},
    user::{EmailAddress, UserId},
};

// Property test generators
pub mod generators {
    use super::*;
    use chrono::{DateTime, Duration, Utc};
    use proptest::collection::vec;
    use proptest::string::string_regex;

    /// Generate valid session IDs
    pub fn session_id() -> impl Strategy<Value = SessionId> {
        any::<u64>().prop_map(|_| SessionId::generate())
    }

    /// Generate valid user IDs
    pub fn user_id() -> impl Strategy<Value = UserId> {
        any::<u64>().prop_map(|_| UserId::generate())
    }

    /// Generate valid request IDs
    pub fn request_id() -> impl Strategy<Value = RequestId> {
        any::<u64>().prop_map(|_| RequestId::generate())
    }

    /// Generate valid application IDs
    pub fn application_id() -> impl Strategy<Value = ApplicationId> {
        string_regex("[a-z][a-z0-9_-]{0,49}")
            .unwrap()
            .prop_filter_map("Invalid application ID", |s| ApplicationId::try_new(s).ok())
    }

    /// Generate valid email addresses
    pub fn email_address() -> impl Strategy<Value = EmailAddress> {
        string_regex(r"[a-z]+@[a-z]+\.[a-z]+")
            .unwrap()
            .prop_filter_map("Invalid email", |s| EmailAddress::try_new(s).ok())
    }

    /// Generate valid tags
    pub fn tag() -> impl Strategy<Value = Tag> {
        string_regex(r"[a-zA-Z0-9][a-zA-Z0-9:._-]{0,49}")
            .unwrap()
            .prop_filter_map("Invalid tag", |s| Tag::try_new(s).ok())
    }

    /// Generate valid model versions
    pub fn model_version() -> impl Strategy<Value = ModelVersion> {
        prop_oneof![
            Just(ModelVersion::new("gpt-4-turbo-2024-01".to_string())),
            Just(ModelVersion::new("gpt-3.5-turbo".to_string())),
            Just(ModelVersion::new("claude-3-opus-20240229".to_string())),
            Just(ModelVersion::new("claude-3-sonnet-20240229".to_string())),
        ]
    }

    /// Generate timestamps in a reasonable range
    pub fn timestamp() -> impl Strategy<Value = Timestamp> {
        (0i64..1_000_000_000).prop_map(|offset| {
            let base_time = Utc::now();
            Timestamp::from(base_time + Duration::seconds(offset))
        })
    }

    /// Generate session status
    pub fn session_status() -> impl Strategy<Value = SessionStatus> {
        prop_oneof![
            Just(SessionStatus::Active),
            Just(SessionStatus::Completed),
            Just(SessionStatus::Aborted),
        ]
    }

    /// Generate a single domain event
    pub fn domain_event() -> impl Strategy<Value = DomainEvent> {
        prop_oneof![
            // Session events
            (session_id(), user_id(), application_id(), timestamp())
                .prop_map(|(sid, uid, aid, ts)| DomainEvent::SessionStarted {
                    session_id: sid,
                    user_id: uid,
                    application_id: aid,
                    started_at: ts,
                }),
            (session_id(), timestamp(), session_status())
                .prop_map(|(sid, ts, status)| DomainEvent::SessionEnded {
                    session_id: sid,
                    ended_at: ts,
                    final_status: status,
                }),
            (session_id(), tag(), timestamp())
                .prop_map(|(sid, tag, ts)| DomainEvent::SessionTagged {
                    session_id: sid,
                    tag,
                    tagged_at: ts,
                }),
            // Version events
            (model_version(), session_id(), timestamp())
                .prop_map(|(mv, sid, ts)| DomainEvent::VersionFirstSeen {
                    model_version: mv,
                    session_id: sid,
                    first_seen_at: ts,
                }),
        ]
    }

    /// Generate a sequence of events for a single session
    pub fn session_event_sequence() -> impl Strategy<Value = Vec<DomainEvent>> {
        session_id().prop_flat_map(|sid| {
            (
                Just(sid.clone()),
                user_id(),
                application_id(),
                vec(timestamp(), 1..20),
            )
                .prop_map(move |(session_id, user_id, app_id, mut timestamps)| {
                    timestamps.sort_by_key(|ts| ts.as_ref());
                    
                    let mut events = Vec::new();
                    
                    // Always start with SessionStarted
                    events.push(DomainEvent::SessionStarted {
                        session_id: session_id.clone(),
                        user_id: user_id.clone(),
                        application_id: app_id.clone(),
                        started_at: timestamps[0],
                    });
                    
                    // Add random events in the middle
                    for &ts in &timestamps[1..timestamps.len().saturating_sub(1)] {
                        events.push(DomainEvent::SessionTagged {
                            session_id: session_id.clone(),
                            tag: Tag::try_new("test-tag".to_string()).unwrap(),
                            tagged_at: ts,
                        });
                    }
                    
                    // Always end with SessionEnded if we have enough timestamps
                    if timestamps.len() > 1 {
                        events.push(DomainEvent::SessionEnded {
                            session_id,
                            ended_at: *timestamps.last().unwrap(),
                            final_status: SessionStatus::Completed,
                        });
                    }
                    
                    events
                })
        })
    }

    /// Generate multiple concurrent sessions
    pub fn concurrent_sessions() -> impl Strategy<Value = Vec<Vec<DomainEvent>>> {
        vec(session_event_sequence(), 1..5)
    }
}

// Domain invariants to verify
pub struct DomainInvariants;

impl DomainInvariants {
    /// Invariant: Events in a stream must have non-decreasing timestamps
    pub fn timestamps_non_decreasing(events: &[DomainEvent]) -> bool {
        let mut last_timestamp: Option<Timestamp> = None;
        
        for event in events {
            let timestamp = event.occurred_at();
            if let Some(last) = last_timestamp {
                if timestamp < last {
                    return false;
                }
            }
            last_timestamp = Some(timestamp);
        }
        
        true
    }

    /// Invariant: Every session must start before it ends
    pub fn sessions_start_before_end(events: &[DomainEvent]) -> bool {
        let mut session_starts = HashMap::new();
        let mut session_ends = HashMap::new();
        
        for event in events {
            match event {
                DomainEvent::SessionStarted { session_id, started_at, .. } => {
                    session_starts.insert(session_id.clone(), *started_at);
                }
                DomainEvent::SessionEnded { session_id, ended_at, .. } => {
                    session_ends.insert(session_id.clone(), *ended_at);
                }
                _ => {}
            }
        }
        
        for (session_id, end_time) in session_ends {
            if let Some(start_time) = session_starts.get(&session_id) {
                if end_time <= *start_time {
                    return false;
                }
            } else {
                // Session ended without starting
                return false;
            }
        }
        
        true
    }

    /// Invariant: Session events can only occur for started sessions
    pub fn session_events_require_active_session(events: &[DomainEvent]) -> bool {
        let mut active_sessions = HashSet::new();
        
        for event in events {
            match event {
                DomainEvent::SessionStarted { session_id, .. } => {
                    active_sessions.insert(session_id.clone());
                }
                DomainEvent::SessionEnded { session_id, .. } => {
                    active_sessions.remove(session_id);
                }
                DomainEvent::SessionTagged { session_id, .. } => {
                    if !active_sessions.contains(session_id) {
                        return false;
                    }
                }
                DomainEvent::LlmRequestReceived { session_id, .. } => {
                    if !active_sessions.contains(session_id) {
                        return false;
                    }
                }
                _ => {}
            }
        }
        
        true
    }

    /// Invariant: Request lifecycle must follow proper state transitions
    pub fn request_lifecycle_valid(events: &[DomainEvent]) -> bool {
        let mut request_states = HashMap::new();
        
        for event in events {
            match event {
                DomainEvent::LlmRequestReceived { request_id, .. } => {
                    if request_states.contains_key(request_id) {
                        return false; // Duplicate request received
                    }
                    request_states.insert(request_id.clone(), RequestState::Received);
                }
                DomainEvent::LlmRequestStarted { request_id, .. } => {
                    match request_states.get(request_id) {
                        Some(RequestState::Received) => {
                            request_states.insert(request_id.clone(), RequestState::Started);
                        }
                        _ => return false, // Invalid transition
                    }
                }
                DomainEvent::LlmResponseReceived { request_id, .. } => {
                    match request_states.get(request_id) {
                        Some(RequestState::Started) => {
                            request_states.insert(request_id.clone(), RequestState::Completed);
                        }
                        _ => return false, // Invalid transition
                    }
                }
                DomainEvent::LlmRequestFailed { request_id, .. } => {
                    match request_states.get(request_id) {
                        Some(RequestState::Received) | Some(RequestState::Started) => {
                            request_states.insert(request_id.clone(), RequestState::Failed);
                        }
                        _ => return false, // Invalid transition
                    }
                }
                _ => {}
            }
        }
        
        true
    }

    /// Invariant: Model versions must be consistent within version events
    pub fn version_consistency(events: &[DomainEvent]) -> bool {
        let mut version_mentions = HashMap::new();
        
        for event in events {
            match event {
                DomainEvent::VersionFirstSeen { model_version, .. } => {
                    version_mentions.insert(model_version.to_string(), model_version.clone());
                }
                DomainEvent::VersionChanged { from_version, to_version, .. } => {
                    version_mentions.insert(from_version.to_string(), from_version.clone());
                    version_mentions.insert(to_version.to_string(), to_version.clone());
                }
                DomainEvent::LlmRequestReceived { model_version, .. } => {
                    if let Some(stored_version) = version_mentions.get(&model_version.to_string()) {
                        if *stored_version != *model_version {
                            return false; // Inconsistent version representation
                        }
                    } else {
                        version_mentions.insert(model_version.to_string(), model_version.clone());
                    }
                }
                _ => {}
            }
        }
        
        true
    }

    /// Invariant: F-scores must be within valid range [0.0, 1.0]
    pub fn f_scores_in_valid_range(events: &[DomainEvent]) -> bool {
        for event in events {
            match event {
                DomainEvent::FScoreCalculated { f_score, precision, recall, .. } => {
                    let f_val = f_score.as_ref();
                    if !(0.0..=1.0).contains(f_val) {
                        return false;
                    }
                    
                    if let Some(p) = precision {
                        let p_val = p.as_ref();
                        if !(0.0..=1.0).contains(p_val) {
                            return false;
                        }
                    }
                    
                    if let Some(r) = recall {
                        let r_val = r.as_ref();
                        if !(0.0..=1.0).contains(r_val) {
                            return false;
                        }
                    }
                }
                DomainEvent::ApplicationFScoreCalculated { f_score, precision, recall, .. } => {
                    let f_val = f_score.as_ref();
                    if !(0.0..=1.0).contains(f_val) {
                        return false;
                    }
                    
                    if let Some(p) = precision {
                        let p_val = p.as_ref();
                        if !(0.0..=1.0).contains(p_val) {
                            return false;
                        }
                    }
                    
                    if let Some(r) = recall {
                        let r_val = r.as_ref();
                        if !(0.0..=1.0).contains(r_val) {
                            return false;
                        }
                    }
                }
                _ => {}
            }
        }
        
        true
    }
}

#[derive(Debug, Clone, PartialEq)]
enum RequestState {
    Received,
    Started,
    Completed,
    Failed,
}

// Property tests
#[cfg(test)]
mod property_tests {
    use super::*;
    use crate::tests::event_sourcing::fixtures::stream_builders::ScenarioBuilder;

    proptest! {
        #[test]
        fn prop_timestamps_non_decreasing(events in generators::session_event_sequence()) {
            prop_assert!(DomainInvariants::timestamps_non_decreasing(&events));
        }

        #[test]
        fn prop_sessions_start_before_end(events in generators::session_event_sequence()) {
            prop_assert!(DomainInvariants::sessions_start_before_end(&events));
        }

        #[test]
        fn prop_session_events_require_active_session(events in generators::session_event_sequence()) {
            prop_assert!(DomainInvariants::session_events_require_active_session(&events));
        }

        #[test]
        fn prop_version_consistency(events in generators::concurrent_sessions()) {
            for session_events in events {
                prop_assert!(DomainInvariants::version_consistency(&session_events));
            }
        }

        #[test]
        fn prop_single_event_invariants(event in generators::domain_event()) {
            let events = vec![event];
            prop_assert!(DomainInvariants::timestamps_non_decreasing(&events));
            prop_assert!(DomainInvariants::version_consistency(&events));
            prop_assert!(DomainInvariants::f_scores_in_valid_range(&events));
        }
    }

    // Test invariants against known scenarios
    #[test]
    fn test_invariants_against_typical_session() {
        let events = ScenarioBuilder::typical_user_session();
        
        assert!(DomainInvariants::timestamps_non_decreasing(&events));
        assert!(DomainInvariants::sessions_start_before_end(&events));
        assert!(DomainInvariants::session_events_require_active_session(&events));
        assert!(DomainInvariants::version_consistency(&events));
        assert!(DomainInvariants::f_scores_in_valid_range(&events));
    }

    #[test]
    fn test_invariants_against_concurrent_sessions() {
        let sessions = ScenarioBuilder::concurrent_sessions(3);
        
        for (_session_id, events) in sessions {
            assert!(DomainInvariants::timestamps_non_decreasing(&events));
            assert!(DomainInvariants::sessions_start_before_end(&events));
            assert!(DomainInvariants::session_events_require_active_session(&events));
            assert!(DomainInvariants::version_consistency(&events));
            assert!(DomainInvariants::f_scores_in_valid_range(&events));
        }
    }

    #[test]
    fn test_invariants_against_complex_scenario() {
        let events = ScenarioBuilder::long_running_session_with_model_evolution();
        
        assert!(DomainInvariants::timestamps_non_decreasing(&events));
        assert!(DomainInvariants::sessions_start_before_end(&events));
        assert!(DomainInvariants::session_events_require_active_session(&events));
        assert!(DomainInvariants::version_consistency(&events));
        assert!(DomainInvariants::f_scores_in_valid_range(&events));
    }

    #[test]
    fn test_request_lifecycle_invariant() {
        let events = ScenarioBuilder::session_with_failures();
        
        // This should pass even with failures
        assert!(DomainInvariants::request_lifecycle_valid(&events));
    }
}