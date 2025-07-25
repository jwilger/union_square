//! Audit path implementation for processing events from the ring buffer

use crate::proxy::{ring_buffer::RingBuffer, types::*};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// Audit path processor that reads from the ring buffer
pub struct AuditPathProcessor {
    ring_buffer: Arc<RingBuffer>,
    shutdown_rx: mpsc::Receiver<()>,
}

impl AuditPathProcessor {
    /// Create a new audit path processor
    pub fn new(ring_buffer: Arc<RingBuffer>) -> (Self, mpsc::Sender<()>) {
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);

        let processor = Self {
            ring_buffer,
            shutdown_rx,
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

        // TODO: Write to EventCore for persistent storage

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proxy::types::{RequestId, RingBufferConfig, SessionId};
    use uuid::Uuid;

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
            request_id: unsafe { RequestId::new_unchecked(Uuid::now_v7()) },
            session_id: unsafe { SessionId::new_unchecked(Uuid::now_v7()) },
            timestamp: chrono::Utc::now(),
            event_type: AuditEventType::RequestReceived {
                method: "GET".to_string(),
                uri: "/test".to_string(),
                headers: vec![],
                body_size: 0,
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
        let request_id = unsafe { RequestId::new_unchecked(Uuid::now_v7()) };
        ring_buffer.write(request_id, b"invalid json").unwrap();

        let (mut processor, _shutdown_tx) = AuditPathProcessor::new(ring_buffer);

        // Should handle invalid data gracefully
        let result = processor.process_next_event().await;
        assert!(result.is_ok());
        assert!(result.unwrap()); // Should still return true (event was "processed")
    }
}
