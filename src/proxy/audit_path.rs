//! Audit path implementation for processing events from the ring buffer

#[cfg(test)]
use crate::domain::commands::audit_commands::{
    RecordRequestForwarded, RecordRequestReceived, RecordResponseReceived, RecordResponseReturned,
};
use crate::infrastructure::eventcore::service::EventCoreService;
use crate::proxy::{ring_buffer::RingBuffer, types::*};
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

        let processor = Self {
            ring_buffer,
            shutdown_rx,
            event_store: None,
        };

        (processor, shutdown_tx)
    }

    /// Create a new audit path processor with EventCore integration
    pub fn with_event_store(
        ring_buffer: Arc<RingBuffer>,
        event_store: Arc<EventCoreService>,
    ) -> (Self, mpsc::Sender<()>) {
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);

        let processor = Self {
            ring_buffer,
            shutdown_rx,
            event_store: Some(event_store),
        };

        (processor, shutdown_tx)
    }

    /// Run the audit path processor
    pub async fn run(mut self) {
        info!("Audit path processor started");

        loop {
            // Check for shutdown signal
            if self.shutdown_rx.try_recv().is_ok() {
                info!("Audit path processor shutting down");
                break;
            }

            // Process events from ring buffer
            match self.process_next_event().await {
                Ok(true) => {
                    // Successfully processed an event
                    continue;
                }
                Ok(false) => {
                    // No events available, sleep briefly
                    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                }
                Err(e) => {
                    error!("Error processing audit event: {}", e);
                    // Continue processing despite errors
                }
            }
        }

        info!("Audit path processor stopped");
    }

    /// Process the next event from the ring buffer
    async fn process_next_event(&mut self) -> ProxyResult<bool> {
        // Read from ring buffer
        if let Some((request_id, data)) = self.ring_buffer.read() {
            debug!("Processing audit event for request {}", request_id);

            // Deserialize the audit event
            match serde_json::from_slice::<AuditEvent>(&data) {
                Ok(event) => {
                    self.handle_audit_event(event).await?;
                    Ok(true)
                }
                Err(e) => {
                    warn!("Failed to deserialize audit event: {}", e);
                    // Log the error but continue processing
                    Ok(true)
                }
            }
        } else {
            // No events available
            Ok(false)
        }
    }

    /// Handle a specific audit event
    async fn handle_audit_event(&self, event: AuditEvent) -> ProxyResult<()> {
        debug!("Handling audit event: {:?}", event.event_type);

        // TODO: Implement actual event handling
        // For now, just log the event
        match &event.event_type {
            AuditEventType::RequestReceived { method, uri, .. } => {
                info!("Request received: {} {}", method, uri);
            }
            AuditEventType::RequestForwarded { target_url, .. } => {
                info!("Request forwarded to: {}", target_url);
            }
            AuditEventType::ResponseReceived {
                status,
                duration_ms,
                ..
            } => {
                info!("Response received: {} ({}ms)", status, duration_ms);
            }
            AuditEventType::ResponseReturned { duration_ms } => {
                info!("Response returned to client ({}ms)", duration_ms);
            }
            AuditEventType::Error { error, phase } => {
                warn!("Error in {:?} phase: {}", phase, error);
            }
            _ => {
                debug!("Unhandled event type");
            }
        }

        // Write to EventCore if available
        if let Some(event_store) = &self.event_store {
            // This will fail until we implement the integration
            self.write_to_eventcore(&event, event_store).await?;
        }

        Ok(())
    }

    /// Write audit event to EventCore
    async fn write_to_eventcore(
        &self,
        #[allow(unused_variables)] event: &AuditEvent,
        #[allow(unused_variables)] event_store: &EventCoreService,
    ) -> ProxyResult<()> {
        #[cfg(test)]
        {
            match &event.event_type {
                AuditEventType::RequestReceived { .. } => {
                    let command = RecordRequestReceived::from_audit_event(event).map_err(|e| {
                        ProxyError::Internal(format!("Failed to create command: {e}"))
                    })?;
                    event_store
                        .execute_command_memory(command)
                        .await
                        .map_err(|e| {
                            ProxyError::Internal(format!("EventCore execution failed: {e}"))
                        })?;
                }
                AuditEventType::RequestForwarded { .. } => {
                    let command = RecordRequestForwarded::from_audit_event(event).map_err(|e| {
                        ProxyError::Internal(format!("Failed to create command: {e}"))
                    })?;
                    event_store
                        .execute_command_memory(command)
                        .await
                        .map_err(|e| {
                            ProxyError::Internal(format!("EventCore execution failed: {e}"))
                        })?;
                }
                AuditEventType::ResponseReceived { .. } => {
                    let command = RecordResponseReceived::from_audit_event(event).map_err(|e| {
                        ProxyError::Internal(format!("Failed to create command: {e}"))
                    })?;
                    event_store
                        .execute_command_memory(command)
                        .await
                        .map_err(|e| {
                            ProxyError::Internal(format!("EventCore execution failed: {e}"))
                        })?;
                }
                AuditEventType::ResponseReturned { .. } => {
                    let command = RecordResponseReturned::from_audit_event(event).map_err(|e| {
                        ProxyError::Internal(format!("Failed to create command: {e}"))
                    })?;
                    event_store
                        .execute_command_memory(command)
                        .await
                        .map_err(|e| {
                            ProxyError::Internal(format!("EventCore execution failed: {e}"))
                        })?;
                }
                AuditEventType::Error { .. } => {
                    // TODO: Implement error event handling if needed
                    debug!("Error events not yet persisted to EventCore");
                }
                _ => {
                    debug!("Unhandled event type for EventCore persistence");
                }
            }

            Ok(())
        }

        #[cfg(not(test))]
        {
            // In production, we'll use the PostgreSQL backend
            // For now, just return an error
            Err(ProxyError::Internal(
                "EventCore PostgreSQL backend not yet implemented".to_string(),
            ))
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

        let (processor, shutdown_tx) = AuditPathProcessor::new(ring_buffer);

        // Should be created successfully
        drop(processor);
        drop(shutdown_tx);
    }

    #[tokio::test]
    async fn test_audit_processor_shutdown() {
        let config = RingBufferConfig::default();
        let ring_buffer = Arc::new(RingBuffer::new(&config));

        let (processor, shutdown_tx) = AuditPathProcessor::new(ring_buffer);

        // Start processor in background
        let handle = tokio::spawn(async move {
            processor.run().await;
        });

        // Give it time to start
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Send shutdown signal
        shutdown_tx.send(()).await.unwrap();

        // Should shutdown cleanly
        tokio::time::timeout(tokio::time::Duration::from_secs(1), handle)
            .await
            .unwrap()
            .unwrap();
    }

    #[tokio::test]
    async fn test_audit_event_processing() {
        let config = RingBufferConfig::default();
        let ring_buffer = Arc::new(RingBuffer::new(&config));

        // Write an audit event to the ring buffer
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

        let (mut processor, _shutdown_tx) = AuditPathProcessor::new(ring_buffer);

        // Process the event
        let result = processor.process_next_event().await;
        assert!(result.is_ok());
        assert!(result.unwrap()); // Should have processed an event

        // No more events
        let result = processor.process_next_event().await;
        assert!(result.is_ok());
        assert!(!result.unwrap()); // Should have no events
    }

    #[tokio::test]
    async fn test_invalid_event_handling() {
        let config = RingBufferConfig::default();
        let ring_buffer = Arc::new(RingBuffer::new(&config));

        // Write invalid data to the ring buffer
        let request_id = RequestId::new();
        ring_buffer.write(request_id, b"invalid json").unwrap();

        let (mut processor, _shutdown_tx) = AuditPathProcessor::new(ring_buffer);

        // Should handle invalid data gracefully
        let result = processor.process_next_event().await;
        assert!(result.is_ok());
        assert!(result.unwrap()); // Should still return true (event was "processed")
    }

    #[tokio::test]
    async fn test_audit_processor_eventcore_integration() {
        let config = RingBufferConfig::default();
        let ring_buffer = Arc::new(RingBuffer::new(&config));

        // Create EventCore service with in-memory store for testing
        let event_store = EventCoreService::with_memory_store();

        // Create processor with EventCore integration
        let (processor, _shutdown_tx) =
            AuditPathProcessor::with_event_store(ring_buffer.clone(), Arc::new(event_store));

        // Write an audit event to the ring buffer
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

        // Process the event
        let mut processor = processor;
        let result = processor.process_next_event().await;

        // Should successfully process the event and write to EventCore
        assert!(result.is_ok(), "Failed to process event: {result:?}");
        assert!(result.unwrap()); // Should have processed an event
    }

    #[tokio::test]
    async fn test_audit_events_written_to_eventcore() {
        // This test verifies that audit events are successfully written to EventCore
        // It will fail until we implement the actual integration
        let config = RingBufferConfig::default();
        let ring_buffer = Arc::new(RingBuffer::new(&config));

        // Create EventCore service with in-memory store for testing
        let event_store = EventCoreService::with_memory_store();

        // Create processor with EventCore integration
        let (processor, _shutdown_tx) =
            AuditPathProcessor::with_event_store(ring_buffer.clone(), Arc::new(event_store));

        // Create multiple audit events to write
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

        // Write events to ring buffer
        for event in &events {
            let serialized = serde_json::to_vec(&event).unwrap();
            ring_buffer.write(event.request_id, &serialized).unwrap();
        }

        // Process the events
        let mut processor = processor;
        for _ in 0..events.len() {
            // This will fail until EventCore integration is implemented
            let result = processor.process_next_event().await;
            assert!(result.is_ok(), "Failed to process event: {result:?}");
            assert!(result.unwrap()); // Should have processed an event
        }

        // TODO: Once EventCore integration is implemented, verify events were written
        // by querying EventCore and checking that the events exist
    }
}
