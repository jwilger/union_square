//! Audit path implementation for processing events from the ring buffer

use crate::adapters::proxy_audit::convert_audit_event;
use crate::infrastructure::eventcore::service::EventCoreService;
use crate::proxy::{
    audit_steps::{AuditEffect, LogLevel, Observation, ProcessorState, Step},
    ring_buffer::RingBuffer,
    types::*,
};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// Audit path processor that reads from the ring buffer
pub struct AuditPathProcessor {
    ring_buffer: Arc<RingBuffer>,
    shutdown_rx: mpsc::Receiver<()>,
    event_store: Option<Arc<EventCoreService>>,
}

impl AuditPathProcessor {
    /// Create a new audit path processor
    pub fn new(ring_buffer: Arc<RingBuffer>) -> (Self, mpsc::Sender<()>) {
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
        (
            Self {
                ring_buffer,
                shutdown_rx,
                event_store: None,
            },
            shutdown_tx,
        )
    }

    /// Create a new audit path processor with EventCore integration
    pub fn with_event_store(
        ring_buffer: Arc<RingBuffer>,
        event_store: Arc<EventCoreService>,
    ) -> (Self, mpsc::Sender<()>) {
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
        (
            Self {
                ring_buffer,
                shutdown_rx,
                event_store: Some(event_store),
            },
            shutdown_tx,
        )
    }

    /// Run the audit path processor
    pub async fn run(mut self) {
        info!("Audit path processor started");
        let mut state = ProcessorState::default();

        'outer: loop {
            // Check for shutdown signal
            if self.shutdown_rx.try_recv().is_ok() {
                let (new_state, step) =
                    crate::proxy::audit_steps::step(state, Observation::ShutdownRequested);
                state = new_state;
                if matches!(step, Step::Stop) {
                    info!("Audit path processor shutting down");
                    break;
                }
            }

            // Read from ring buffer
            let read_result = self.ring_buffer.read();
            let mut observation = Observation::RingBufferRead(read_result);

            loop {
                let (new_state, step) = crate::proxy::audit_steps::step(state, observation);
                state = new_state;

                match step {
                    Step::Stop => {
                        info!("Audit path processor stopped");
                        break 'outer;
                    }
                    Step::Continue => {
                        // Go back to outer loop to read ring buffer again
                        continue 'outer;
                    }
                    Step::Effect(effect) => {
                        observation = Self::perform_effect(effect, &self.event_store).await;
                    }
                }
            }
        }

        info!("Audit path processor stopped");
        info!("  events_processed={}", state.events_processed);
        info!(
            "  deserialization_failures={}",
            state.deserialization_failures
        );
        info!("  conversion_failures={}", state.conversion_failures);
        info!("  persist_failures={}", state.persist_failures);
    }

    /// Perform a single audit effect and return the observation
    async fn perform_effect(
        effect: AuditEffect,
        event_store: &Option<Arc<EventCoreService>>,
    ) -> Observation {
        match effect {
            AuditEffect::Deserialize { data } => {
                let result = serde_json::from_slice::<AuditEvent>(&data).map_err(|e| e.to_string());
                Observation::Deserialized(result)
            }
            AuditEffect::ConvertToDomain { event } => {
                let result = convert_audit_event(&event).map_err(|e| e.to_string());
                Observation::Converted(result)
            }
            AuditEffect::Persist { command } => {
                let result = if let Some(store) = event_store {
                    store
                        .execute_command(command)
                        .await
                        .map_err(|e| e.to_string())
                } else {
                    warn!("No event store configured; skipping persistence");
                    Err("No event store configured".to_string())
                };
                Observation::Persisted(result)
            }
            AuditEffect::Log { level, message } => {
                match level {
                    LogLevel::Debug => debug!("{message}"),
                    LogLevel::Info => info!("{message}"),
                    LogLevel::Warn => warn!("{message}"),
                    LogLevel::Error => error!("{message}"),
                }
                Observation::LogComplete
            }
            AuditEffect::Sleep { duration } => {
                tokio::time::sleep(duration).await;
                Observation::SleepComplete
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::eventcore::service::EventCoreService;
    use crate::proxy::types::{RequestId, RingBufferConfig, SessionId};

    #[tokio::test]
    async fn test_audit_processor_creation() {
        let config = RingBufferConfig::default();
        let ring_buffer = Arc::new(RingBuffer::new(&config));
        let (processor, _shutdown_tx) = AuditPathProcessor::new(ring_buffer);
        drop(processor);
    }

    #[tokio::test]
    async fn test_audit_processor_shutdown() {
        let config = RingBufferConfig::default();
        let ring_buffer = Arc::new(RingBuffer::new(&config));
        let (processor, shutdown_tx) = AuditPathProcessor::new(ring_buffer);

        let handle = tokio::spawn(async move {
            processor.run().await;
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        shutdown_tx.send(()).await.unwrap();

        tokio::time::timeout(tokio::time::Duration::from_secs(1), handle)
            .await
            .unwrap()
            .unwrap();
    }

    #[tokio::test]
    async fn test_audit_event_processing() {
        let config = RingBufferConfig::default();
        let ring_buffer = Arc::new(RingBuffer::new(&config));

        let event = AuditEvent {
            request_id: RequestId::new(),
            session_id: SessionId::new(),
            timestamp: chrono::Utc::now(),
            event_type: AuditEventType::RequestReceived {
                method: HttpMethod::try_new(METHOD_GET.to_string()).unwrap(),
                uri: RequestUri::try_new("/test".to_string()).unwrap(),
                headers: Headers::new(),
                body_size: BodySize::from(0),
            },
        };

        let serialized = serde_json::to_vec(&event).unwrap();
        ring_buffer.write(event.request_id, &serialized).unwrap();

        let (processor, shutdown_tx) = AuditPathProcessor::new(ring_buffer.clone());

        let handle = tokio::spawn(async move {
            processor.run().await;
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        shutdown_tx.send(()).await.unwrap();
        tokio::time::timeout(tokio::time::Duration::from_secs(1), handle)
            .await
            .unwrap()
            .unwrap();
    }

    #[tokio::test]
    async fn test_invalid_event_handling() {
        let config = RingBufferConfig::default();
        let ring_buffer = Arc::new(RingBuffer::new(&config));
        let request_id = RequestId::new();
        ring_buffer.write(request_id, b"invalid json").unwrap();

        let (processor, shutdown_tx) = AuditPathProcessor::new(ring_buffer.clone());

        let handle = tokio::spawn(async move {
            processor.run().await;
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        shutdown_tx.send(()).await.unwrap();
        tokio::time::timeout(tokio::time::Duration::from_secs(1), handle)
            .await
            .unwrap()
            .unwrap();
    }

    #[tokio::test]
    async fn test_audit_processor_eventcore_integration() {
        let config = RingBufferConfig::default();
        let ring_buffer = Arc::new(RingBuffer::new(&config));

        let event_store = Arc::new(EventCoreService::with_memory_store());
        let (processor, shutdown_tx) =
            AuditPathProcessor::with_event_store(ring_buffer.clone(), Arc::clone(&event_store));

        let event = AuditEvent {
            request_id: RequestId::new(),
            session_id: SessionId::new(),
            timestamp: chrono::Utc::now(),
            event_type: AuditEventType::RequestReceived {
                method: HttpMethod::try_new(METHOD_GET.to_string()).unwrap(),
                uri: RequestUri::try_new("/test".to_string()).unwrap(),
                headers: Headers::new(),
                body_size: BodySize::from(100),
            },
        };

        let serialized = serde_json::to_vec(&event).unwrap();
        ring_buffer.write(event.request_id, &serialized).unwrap();

        let handle = tokio::spawn(async move {
            processor.run().await;
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        shutdown_tx.send(()).await.unwrap();
        tokio::time::timeout(tokio::time::Duration::from_secs(1), handle)
            .await
            .unwrap()
            .unwrap();

        let domain_session_id = crate::domain::session::SessionId::new(*event.session_id.as_ref());
        let session_stream =
            crate::domain::commands::audit_commands::RecordAuditEvent::session_stream_id(
                &domain_session_id,
            )
            .unwrap();

        let events = event_store
            .read_stream::<crate::domain::events::DomainEvent>(session_stream)
            .await
            .unwrap();

        assert!(!events.is_empty(), "Expected events to be persisted");
    }

    #[tokio::test]
    async fn test_audit_events_written_to_eventcore() {
        let config = RingBufferConfig::default();
        let ring_buffer = Arc::new(RingBuffer::new(&config));

        let event_store = Arc::new(EventCoreService::with_memory_store());
        let (processor, shutdown_tx) =
            AuditPathProcessor::with_event_store(ring_buffer.clone(), Arc::clone(&event_store));

        let events = vec![
            AuditEvent {
                request_id: RequestId::new(),
                session_id: SessionId::new(),
                timestamp: chrono::Utc::now(),
                event_type: AuditEventType::RequestReceived {
                    method: HttpMethod::try_new(METHOD_POST.to_string()).unwrap(),
                    uri: RequestUri::try_new("/api/chat".to_string()).unwrap(),
                    headers: Headers::new(),
                    body_size: BodySize::from(1024),
                },
            },
            AuditEvent {
                request_id: RequestId::new(),
                session_id: SessionId::new(),
                timestamp: chrono::Utc::now(),
                event_type: AuditEventType::RequestForwarded {
                    target_url: TargetUrl::try_new("https://api.openai.com/v1/chat".to_string())
                        .unwrap(),
                    start_time: chrono::Utc::now(),
                },
            },
        ];

        for event in &events {
            let serialized = serde_json::to_vec(&event).unwrap();
            ring_buffer.write(event.request_id, &serialized).unwrap();
        }

        let handle = tokio::spawn(async move {
            processor.run().await;
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        shutdown_tx.send(()).await.unwrap();
        tokio::time::timeout(tokio::time::Duration::from_secs(1), handle)
            .await
            .unwrap()
            .unwrap();

        let domain_session_id =
            crate::domain::session::SessionId::new(*events[0].session_id.as_ref());
        let session_stream =
            crate::domain::commands::audit_commands::RecordAuditEvent::session_stream_id(
                &domain_session_id,
            )
            .unwrap();

        let persisted = event_store
            .read_stream::<crate::domain::events::DomainEvent>(session_stream)
            .await
            .unwrap();

        assert!(!persisted.is_empty(), "Expected events to be persisted");
    }

    #[tokio::test]
    #[ignore = "requires database connection"]
    async fn test_audit_path_persists_to_postgres() {
        use crate::config::Settings;

        let settings = Settings::new().unwrap();
        let db_url = settings.database_url();

        let config =
            crate::infrastructure::eventcore::EventCoreConfig::try_new(&db_url, 10, 30).unwrap();
        let service = Arc::new(EventCoreService::new(config).await.unwrap());
        service.migrate().await.unwrap();

        let ring_buffer = Arc::new(RingBuffer::new(&RingBufferConfig::default()));
        let (processor, shutdown_tx) =
            AuditPathProcessor::with_event_store(ring_buffer.clone(), Arc::clone(&service));

        let event = AuditEvent {
            request_id: RequestId::new(),
            session_id: SessionId::new(),
            timestamp: chrono::Utc::now(),
            event_type: AuditEventType::RequestReceived {
                method: HttpMethod::try_new(METHOD_POST.to_string()).unwrap(),
                uri: RequestUri::try_new("/v1/chat/completions".to_string()).unwrap(),
                headers: Headers::new(),
                body_size: BodySize::from(256),
            },
        };

        let serialized = serde_json::to_vec(&event).unwrap();
        ring_buffer.write(event.request_id, &serialized).unwrap();

        let handle = tokio::spawn(async move {
            processor.run().await;
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        shutdown_tx.send(()).await.unwrap();
        tokio::time::timeout(tokio::time::Duration::from_secs(2), handle)
            .await
            .unwrap()
            .unwrap();

        let domain_session_id = crate::domain::session::SessionId::new(*event.session_id.as_ref());
        let session_stream =
            crate::domain::commands::audit_commands::RecordAuditEvent::session_stream_id(
                &domain_session_id,
            )
            .unwrap();

        let events = service
            .read_stream::<crate::domain::events::DomainEvent>(session_stream)
            .await
            .unwrap();

        assert!(
            !events.is_empty(),
            "Expected events to be persisted to PostgreSQL"
        );
    }
}
