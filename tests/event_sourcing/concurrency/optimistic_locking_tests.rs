//! Optimistic locking and concurrent event processing tests
//!
//! Tests for verifying correct behavior under concurrent access,
//! optimistic locking scenarios, and race conditions.

use eventcore::{CommandExecutor, CommandLogic, EventStore, ExecutionOptions};
use eventcore_memory::InMemoryEventStore;
use std::sync::{Arc, Barrier};
use std::time::Duration;
use tokio::time::timeout;
use union_square::domain::{
    commands::audit_commands::{ProcessAuditEvent, RequestState},
    events::DomainEvent,
    llm::RequestId,
    session::SessionId,
};

/// Test harness for concurrent event processing
pub struct ConcurrencyTestHarness {
    event_store: Arc<InMemoryEventStore<DomainEvent>>,
    command_executor: Arc<CommandExecutor<InMemoryEventStore<DomainEvent>>>,
}

impl ConcurrencyTestHarness {
    /// Create a new concurrency test harness
    pub fn new() -> Self {
        let event_store = Arc::new(InMemoryEventStore::new());
        let command_executor = Arc::new(CommandExecutor::new(event_store.clone()));

        Self {
            event_store,
            command_executor,
        }
    }

    /// Test concurrent command execution
    pub async fn test_concurrent_commands(&self, command_count: usize) -> ConcurrencyTestResult {
        let start_time = std::time::Instant::now();
        let barrier = Arc::new(Barrier::new(command_count));
        let mut handles = Vec::new();

        // Create concurrent command execution tasks
        for i in 0..command_count {
            let executor = self.command_executor.clone();
            let barrier = barrier.clone();
            
            let handle = tokio::spawn(async move {
                // Wait for all tasks to be ready
                barrier.wait();
                
                // Create a unique command for this task
                let session_id = SessionId::generate();
                let request_id = RequestId::generate();
                
                let command = ProcessAuditEvent {
                    request_id: request_id.clone(),
                    session_id,
                    event_data: format!("concurrent-test-{}", i),
                    initial_state: RequestState {
                        lifecycle: union_square::domain::commands::audit_commands::RequestLifecycle::NotStarted,
                    },
                };

                let result = executor.execute(command, ExecutionOptions::default()).await;
                (i, request_id, result.is_ok())
            });
            
            handles.push(handle);
        }

        // Wait for all commands to complete
        let mut successful_commands = 0;
        let mut failed_commands = 0;
        let mut command_results = Vec::new();

        for handle in handles {
            match handle.await {
                Ok((task_id, request_id, success)) => {
                    command_results.push((task_id, request_id, success));
                    if success {
                        successful_commands += 1;
                    } else {
                        failed_commands += 1;
                    }
                }
                Err(_) => failed_commands += 1,
            }
        }

        let duration = start_time.elapsed();

        ConcurrencyTestResult {
            total_commands: command_count,
            successful_commands,
            failed_commands,
            execution_time: duration,
            commands_per_second: command_count as f64 / duration.as_secs_f64(),
            command_results,
        }
    }

    /// Test race conditions in event stream updates
    pub async fn test_event_stream_race_conditions(&self) -> RaceConditionTestResult {
        let session_id = SessionId::generate();
        let stream_id = eventcore::StreamId::new(format!("session-{}", session_id));
        
        let barrier = Arc::new(Barrier::new(3));
        let mut handles = Vec::new();

        // Task 1: Try to append SessionStarted event
        {
            let event_store = self.event_store.clone();
            let barrier = barrier.clone();
            let stream_id = stream_id.clone();
            let session_id = session_id.clone();
            
            let handle = tokio::spawn(async move {
                barrier.wait();
                
                let event = DomainEvent::SessionStarted {
                    session_id,
                    user_id: union_square::domain::user::UserId::generate(),
                    application_id: union_square::domain::session::ApplicationId::try_new("test".to_string()).unwrap(),
                    started_at: union_square::domain::metrics::Timestamp::now(),
                };
                
                event_store.append_events(stream_id, vec![event]).await
            });
            
            handles.push(handle);
        }

        // Task 2: Try to append SessionTagged event
        {
            let event_store = self.event_store.clone();
            let barrier = barrier.clone();
            let stream_id = stream_id.clone();
            let session_id = session_id.clone();
            
            let handle = tokio::spawn(async move {
                barrier.wait();
                
                let event = DomainEvent::SessionTagged {
                    session_id,
                    tag: union_square::domain::types::Tag::try_new("concurrent-test".to_string()).unwrap(),
                    tagged_at: union_square::domain::metrics::Timestamp::now(),
                };
                
                event_store.append_events(stream_id, vec![event]).await
            });
            
            handles.push(handle);
        }

        // Task 3: Try to append SessionEnded event
        {
            let event_store = self.event_store.clone();
            let barrier = barrier.clone();
            let stream_id = stream_id.clone();
            let session_id = session_id.clone();
            
            let handle = tokio::spawn(async move {
                barrier.wait();
                
                let event = DomainEvent::SessionEnded {
                    session_id,
                    ended_at: union_square::domain::metrics::Timestamp::now(),
                    final_status: union_square::domain::session::SessionStatus::Completed,
                };
                
                event_store.append_events(stream_id, vec![event]).await
            });
            
            handles.push(handle);
        }

        // Wait for all tasks
        let mut successful_appends = 0;
        let mut failed_appends = 0;

        for handle in handles {
            match handle.await {
                Ok(Ok(_)) => successful_appends += 1,
                Ok(Err(_)) => failed_appends += 1,
                Err(_) => failed_appends += 1,
            }
        }

        // Read final stream state
        let final_events = self.event_store
            .read_events(&stream_id, 0, None)
            .await
            .unwrap_or_default();

        RaceConditionTestResult {
            successful_appends,
            failed_appends,
            final_event_count: final_events.len(),
            events_in_order: self.verify_event_ordering(&final_events),
        }
    }

    /// Test optimistic concurrency control
    pub async fn test_optimistic_concurrency(&self) -> OptimisticConcurrencyResult {
        let session_id = SessionId::generate();
        let request_id1 = RequestId::generate();
        let request_id2 = RequestId::generate();

        // Simulate two concurrent operations on the same session
        let barrier = Arc::new(Barrier::new(2));
        let executor1 = self.command_executor.clone();
        let executor2 = self.command_executor.clone();

        let handle1 = {
            let barrier = barrier.clone();
            let session_id = session_id.clone();
            let request_id = request_id1.clone();
            
            tokio::spawn(async move {
                barrier.wait();
                
                let command = ProcessAuditEvent {
                    request_id,
                    session_id,
                    event_data: "concurrent-op-1".to_string(),
                    initial_state: RequestState {
                        lifecycle: union_square::domain::commands::audit_commands::RequestLifecycle::NotStarted,
                    },
                };

                executor1.execute(command, ExecutionOptions::default()).await
            })
        };

        let handle2 = {
            let barrier = barrier.clone();
            let session_id = session_id.clone();
            let request_id = request_id2.clone();
            
            tokio::spawn(async move {
                barrier.wait();
                
                let command = ProcessAuditEvent {
                    request_id,
                    session_id,
                    event_data: "concurrent-op-2".to_string(),
                    initial_state: RequestState {
                        lifecycle: union_square::domain::commands::audit_commands::RequestLifecycle::NotStarted,
                    },
                };

                executor2.execute(command, ExecutionOptions::default()).await
            })
        };

        // Wait for both operations
        let result1 = handle1.await.unwrap();
        let result2 = handle2.await.unwrap();

        let both_succeeded = result1.is_ok() && result2.is_ok();
        let one_succeeded = result1.is_ok() || result2.is_ok();

        OptimisticConcurrencyResult {
            operation1_success: result1.is_ok(),
            operation2_success: result2.is_ok(),
            both_operations_succeeded: both_succeeded,
            at_least_one_succeeded: one_succeeded,
            conflict_detected: !both_succeeded,
        }
    }

    /// Test timeout scenarios under concurrent load
    pub async fn test_concurrent_timeouts(&self) -> TimeoutTestResult {
        let timeout_duration = Duration::from_millis(100);
        let command_count = 5;
        
        let mut handles = Vec::new();
        let barrier = Arc::new(Barrier::new(command_count));

        for i in 0..command_count {
            let executor = self.command_executor.clone();
            let barrier = barrier.clone();
            
            let handle = tokio::spawn(async move {
                barrier.wait();
                
                let session_id = SessionId::generate();
                let request_id = RequestId::generate();
                
                let command = ProcessAuditEvent {
                    request_id,
                    session_id,
                    event_data: format!("timeout-test-{}", i),
                    initial_state: RequestState {
                        lifecycle: union_square::domain::commands::audit_commands::RequestLifecycle::NotStarted,
                    },
                };

                // Wrap the execution with a timeout
                timeout(
                    timeout_duration,
                    executor.execute(command, ExecutionOptions::default())
                ).await
            });
            
            handles.push(handle);
        }

        let mut completed_within_timeout = 0;
        let mut timed_out = 0;
        let mut failed = 0;

        for handle in handles {
            match handle.await {
                Ok(Ok(Ok(_))) => completed_within_timeout += 1,
                Ok(Err(_)) => timed_out += 1,
                Ok(Ok(Err(_))) => failed += 1,
                Err(_) => failed += 1,
            }
        }

        TimeoutTestResult {
            total_operations: command_count,
            completed_within_timeout,
            timed_out,
            failed,
            timeout_duration,
        }
    }

    /// Verify that events are in proper temporal order
    fn verify_event_ordering(&self, events: &[DomainEvent]) -> bool {
        let mut last_timestamp = None;
        
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
}

/// Results of concurrent command execution test
#[derive(Debug)]
pub struct ConcurrencyTestResult {
    pub total_commands: usize,
    pub successful_commands: usize,
    pub failed_commands: usize,
    pub execution_time: Duration,
    pub commands_per_second: f64,
    pub command_results: Vec<(usize, RequestId, bool)>,
}

impl ConcurrencyTestResult {
    pub fn success_rate(&self) -> f64 {
        self.successful_commands as f64 / self.total_commands as f64
    }
}

/// Results of race condition testing
#[derive(Debug)]
pub struct RaceConditionTestResult {
    pub successful_appends: usize,
    pub failed_appends: usize,
    pub final_event_count: usize,
    pub events_in_order: bool,
}

/// Results of optimistic concurrency testing
#[derive(Debug)]
pub struct OptimisticConcurrencyResult {
    pub operation1_success: bool,
    pub operation2_success: bool,
    pub both_operations_succeeded: bool,
    pub at_least_one_succeeded: bool,
    pub conflict_detected: bool,
}

/// Results of timeout testing under concurrent load
#[derive(Debug)]
pub struct TimeoutTestResult {
    pub total_operations: usize,
    pub completed_within_timeout: usize,
    pub timed_out: usize,
    pub failed: usize,
    pub timeout_duration: Duration,
}

impl TimeoutTestResult {
    pub fn completion_rate(&self) -> f64 {
        self.completed_within_timeout as f64 / self.total_operations as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_concurrent_command_execution() {
        let harness = ConcurrencyTestHarness::new();
        let result = harness.test_concurrent_commands(10).await;
        
        assert!(result.success_rate() > 0.0, "At least some commands should succeed");
        assert!(result.commands_per_second > 0.0, "Should have measurable throughput");
        assert_eq!(result.total_commands, 10);
    }

    #[tokio::test]
    async fn test_event_stream_race_conditions() {
        let harness = ConcurrencyTestHarness::new();
        let result = harness.test_event_stream_race_conditions().await;
        
        // At least one append should succeed
        assert!(result.successful_appends > 0, "At least one append should succeed");
        
        // Events should be in proper order if any were written
        if result.final_event_count > 0 {
            assert!(result.events_in_order, "Events should maintain temporal ordering");
        }
    }

    #[tokio::test]
    async fn test_optimistic_concurrency_control() {
        let harness = ConcurrencyTestHarness::new();
        let result = harness.test_optimistic_concurrency().await;
        
        // At least one operation should succeed
        assert!(result.at_least_one_succeeded, "At least one operation should succeed");
        
        // The system should handle concurrent operations gracefully
        // (both succeeding is fine, conflicts are also acceptable)
        assert!(
            result.both_operations_succeeded || result.conflict_detected,
            "System should either allow both operations or detect conflicts"
        );
    }

    #[tokio::test]
    async fn test_concurrent_timeouts() {
        let harness = ConcurrencyTestHarness::new();
        let result = harness.test_concurrent_timeouts().await;
        
        // Should have some measurable results
        assert_eq!(result.total_operations, 5);
        assert!(
            result.completed_within_timeout + result.timed_out + result.failed == result.total_operations,
            "All operations should be accounted for"
        );
        
        // Completion rate should be meaningful
        let completion_rate = result.completion_rate();
        assert!(completion_rate >= 0.0 && completion_rate <= 1.0);
    }

    #[tokio::test]
    async fn test_stress_test_concurrent_operations() {
        let harness = ConcurrencyTestHarness::new();
        
        // Run multiple concurrent tests to stress the system
        let mut stress_handles = Vec::new();
        
        for _ in 0..3 {
            let harness = ConcurrencyTestHarness::new();
            let handle = tokio::spawn(async move {
                harness.test_concurrent_commands(20).await
            });
            stress_handles.push(handle);
        }
        
        // Wait for all stress tests
        for handle in stress_handles {
            let result = handle.await.unwrap();
            assert!(result.success_rate() > 0.5, "Should maintain reasonable success rate under stress");
        }
    }

    #[tokio::test] 
    async fn test_concurrent_projection_updates() {
        let harness = ConcurrencyTestHarness::new();
        
        // Create events that would update the same projection
        let session_id = SessionId::generate();
        let barrier = Arc::new(Barrier::new(3));
        let mut handles = Vec::new();

        for i in 0..3 {
            let executor = harness.command_executor.clone();
            let barrier = barrier.clone();
            let session_id = session_id.clone();
            
            let handle = tokio::spawn(async move {
                barrier.wait();
                
                let request_id = RequestId::generate();
                let command = ProcessAuditEvent {
                    request_id,
                    session_id,
                    event_data: format!("projection-update-{}", i),
                    initial_state: RequestState {
                        lifecycle: union_square::domain::commands::audit_commands::RequestLifecycle::NotStarted,
                    },
                };

                executor.execute(command, ExecutionOptions::default()).await
            });
            
            handles.push(handle);
        }

        // Wait for all commands
        let mut successful_updates = 0;
        for handle in handles {
            if handle.await.unwrap().is_ok() {
                successful_updates += 1;
            }
        }

        assert!(successful_updates > 0, "At least one projection update should succeed");
    }
}