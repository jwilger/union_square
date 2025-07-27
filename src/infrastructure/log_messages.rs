//! Log message constants for infrastructure components
//!
//! This module centralizes all log messages used throughout the infrastructure
//! layer to ensure consistency and enable easy internationalization/modification.

/// Application startup and lifecycle messages
pub mod application {
    pub const STARTING: &str = "Starting Union Square application";
    pub const STARTED_SUCCESSFULLY: &str = "Application started successfully";
    pub const CONNECTING_TO_DATABASE: &str = "Connecting to database at {}";
    pub const STARTING_SERVER: &str = "Starting Union Square server on {}:{}";
}

/// Database-related log messages
pub mod database {
    pub const HEALTH_CHECK_FAILED: &str = "Database health check failed";
    pub const CONNECTION_ESTABLISHED: &str = "Database connection established";
    pub const MIGRATION_STARTED: &str = "Running database migrations";
    pub const MIGRATION_COMPLETED: &str = "Database migrations completed successfully";
}

/// Audit processing messages
pub mod audit {
    pub const PROCESSOR_STARTED: &str = "Audit path processor started";
    pub const PROCESSOR_SHUTTING_DOWN: &str = "Audit path processor shutting down";
    pub const PROCESSOR_STOPPED: &str = "Audit path processor stopped";
    pub const PROCESSING_EVENT: &str = "Processing audit event for request {}";
    pub const HANDLING_EVENT: &str = "Handling audit event: {:?}";
    pub const FAILED_DESERIALIZATION: &str = "Failed to deserialize audit event: {}";
    pub const UNHANDLED_EVENT_TYPE: &str = "Unhandled event type";
    pub const ERROR_PROCESSING_EVENT: &str = "Error processing audit event: {}";
}

/// Request/response processing messages
pub mod request_processing {
    pub const REQUEST_RECEIVED: &str = "Request received: {} {}";
    pub const REQUEST_FORWARDED: &str = "Request forwarded to: {}";
    pub const RESPONSE_RECEIVED: &str = "Response received: {} ({}ms)";
    pub const RESPONSE_RETURNED: &str = "Response returned to client ({}ms)";
    pub const ERROR_IN_PHASE: &str = "Error in {:?} phase: {}";
}

/// Performance and monitoring messages
pub mod performance {
    pub const SINGLE_THREADED_PERFORMANCE: &str = "Single-threaded performance ({} writes):";
    pub const CONCURRENT_PERFORMANCE: &str =
        "Concurrent performance test ({} threads, {} writes each):";
    pub const PER_WRITE_TIMING: &str = "  Per write: {:.2}ns";
    pub const PERFORMANCE_WARNING: &str =
        "Performance warning: write took {:.2}ns (expected <100ns)";
}

/// Error messages for infrastructure components
pub mod errors {
    pub const BODY_COLLECTION_ERROR: &str = "Body collection error: {}";
    pub const CONNECTION_ERROR: &str = "Connection error: {}";
    pub const INVALID_HTTP_STATUS: &str = "Invalid HTTP status code '{}' received from upstream";
    pub const INVALID_HTTP_METHOD: &str = "Invalid HTTP method '{}': failed validation";
    pub const INVALID_REQUEST_URI: &str = "Invalid request URI '{}': {}";
    pub const AUTHENTICATION_ERROR: &str = "Authentication error: {}";
    pub const PROVIDER_ERROR: &str = "Provider error: {}";
}

/// Configuration and validation messages
pub mod configuration {
    pub const LOADING_CONFIG: &str = "Loading configuration from environment: {}";
    pub const CONFIG_LOADED: &str = "Configuration loaded successfully";
    pub const CONFIG_DEFAULT_VALUE: &str = "Using default value for {}: {}";
    pub const CONFIG_OVERRIDE: &str = "Configuration override: {} = {}";
}

/// Test-related log messages (for test utilities)
pub mod testing {
    pub const TEST_SETUP: &str = "Setting up test environment";
    pub const TEST_CLEANUP: &str = "Cleaning up test environment";
    pub const MOCK_DATA_CREATED: &str = "Mock data created for test: {}";
    pub const TEST_ASSERTION_FAILED: &str = "Test assertion failed: {}";
    pub const TEST_TIMEOUT: &str = "Test timed out after {} seconds";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_messages_are_not_empty() {
        // Application messages - test actual content lengths
        assert!(application::STARTING.len() > 10);
        assert!(application::STARTED_SUCCESSFULLY.len() > 10);
        assert!(application::CONNECTING_TO_DATABASE.len() > 10);
        assert!(application::STARTING_SERVER.len() > 10);

        // Database messages
        assert!(database::HEALTH_CHECK_FAILED.len() > 10);
        assert!(database::CONNECTION_ESTABLISHED.len() > 10);

        // Audit messages
        assert!(audit::PROCESSOR_STARTED.len() > 10);
        assert!(audit::PROCESSOR_STOPPED.len() > 10);

        // Error messages
        assert!(errors::BODY_COLLECTION_ERROR.len() > 10);
        assert!(errors::CONNECTION_ERROR.len() > 10);
    }

    #[test]
    fn test_messages_contain_format_placeholders() {
        // Verify that messages expecting parameters have format placeholders
        assert!(application::CONNECTING_TO_DATABASE.contains("{}"));
        assert!(application::STARTING_SERVER.contains("{}"));
        assert!(audit::PROCESSING_EVENT.contains("{}"));
        assert!(request_processing::REQUEST_RECEIVED.contains("{}"));
        assert!(errors::BODY_COLLECTION_ERROR.contains("{}"));
    }

    #[test]
    fn test_placeholder_counts_are_consistent() {
        // Verify format string placeholder counts match expected usage
        assert_eq!(application::STARTING_SERVER.matches("{}").count(), 2);
        assert_eq!(
            request_processing::REQUEST_RECEIVED.matches("{}").count(),
            2
        );
        assert_eq!(performance::CONCURRENT_PERFORMANCE.matches("{}").count(), 2);
    }
}
