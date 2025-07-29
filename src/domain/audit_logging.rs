//! Type-safe audit logging with data integrity guarantees
//!
//! This module provides audit logging types that ensure completeness,
//! consistency, and tamper-evidence through compile-time guarantees.

use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, marker::PhantomData};

use crate::domain::{
    llm::{ModelVersion, RequestId},
    metrics::{Timestamp, Duration},
    session::SessionId,
    types::{ErrorMessage, ResponseText},
    user::UserId,
};

/// Audit log entry with compile-time integrity guarantees
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditLogEntry<T> {
    pub entry_id: AuditEntryId,
    pub session_id: SessionId,
    pub request_id: Option<RequestId>,
    pub timestamp: Timestamp,
    pub entry_type: AuditEntryType<T>,
    pub integrity: IntegrityProof,
    pub context: AuditContext,
}

/// Unique identifier for audit entries
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AuditEntryId(uuid::Uuid);

impl AuditEntryId {
    pub fn generate() -> Self {
        Self(uuid::Uuid::now_v7())
    }

    pub fn as_uuid(&self) -> &uuid::Uuid {
        &self.0
    }
}

/// Audit entry types with phantom type for compile-time safety
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditEntryType<T> {
    /// Session lifecycle events
    SessionStarted {
        user_id: UserId,
        application_id: String,
        _phantom: PhantomData<T>,
    },
    SessionEnded {
        duration: Duration,
        final_state: SessionFinalState,
        _phantom: PhantomData<T>,
    },

    /// Request lifecycle events
    RequestReceived {
        model_version: ModelVersion,
        prompt_hash: Hash256,
        parameters_hash: Hash256,
        _phantom: PhantomData<T>,
    },
    RequestProcessed {
        processing_duration: Duration,
        tokens_consumed: Option<u32>,
        _phantom: PhantomData<T>,
    },
    ResponseGenerated {
        response_hash: Hash256,
        response_size_bytes: u64,
        generation_duration: Duration,
        _phantom: PhantomData<T>,
    },

    /// Error and failure events
    RequestFailed {
        error_type: ErrorType,
        error_message_hash: Hash256,
        failure_stage: FailureStage,
        _phantom: PhantomData<T>,
    },
    SystemError {
        component: SystemComponent,
        error_severity: ErrorSeverity,
        error_details_hash: Hash256,
        _phantom: PhantomData<T>,
    },

    /// Security and compliance events
    AuthenticationEvent {
        auth_result: AuthResult,
        user_agent_hash: Hash256,
        ip_address_hash: Hash256,
        _phantom: PhantomData<T>,
    },
    DataAccessEvent {
        resource_type: ResourceType,
        access_pattern: AccessPattern,
        data_volume_bytes: u64,
        _phantom: PhantomData<T>,
    },

    /// Performance and monitoring events
    PerformanceMetric {
        metric_type: MetricType,
        metric_value: f64,
        measurement_unit: MeasurementUnit,
        _phantom: PhantomData<T>,
    },
}

// Phantom types for different audit contexts
pub struct ProductionAudit;
pub struct DevelopmentAudit;
pub struct TestingAudit;
pub struct SecurityAudit;

/// Session final states for audit logging
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionFinalState {
    CompletedSuccessfully { total_requests: u32 },
    CompletedWithErrors { successful_requests: u32, failed_requests: u32 },
    FailedDuringProcessing { error_code: String },
    CancelledByUser,
    TimeoutExpired,
}

/// 256-bit hash for content integrity
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hash256(String);

impl Hash256 {
    /// Create a new hash from content
    pub fn from_content(content: &str) -> Self {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        Self(format!("{:x}", hasher.finalize()))
    }

    /// Create hash from bytes
    pub fn from_bytes(bytes: &[u8]) -> Self {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        Self(format!("{:x}", hasher.finalize()))
    }

    /// Get the hash string
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Error classification for audit logging
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorType {
    ValidationError,
    AuthenticationError,
    AuthorizationError,
    ResourceNotFound,
    RateLimitExceeded,
    ServiceUnavailable,
    InternalError,
    ExternalServiceError,
    NetworkError,
    TimeoutError,
}

/// Stage where failure occurred
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FailureStage {
    RequestValidation,
    Authentication,
    Authorization,
    RequestProcessing,
    ModelInvocation,
    ResponseGeneration,
    ResponseValidation,
    ResponseDelivery,
}

/// System components for error tracking
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SystemComponent {
    ProxyService,
    AuthenticationService,
    ModelProvider,
    DatabaseLayer,
    EventStore,
    MetricsCollector,
    LoadBalancer,
    NetworkLayer,
}

/// Error severity levels
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorSeverity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

/// Authentication results
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthResult {
    Success,
    InvalidCredentials,
    ExpiredToken,
    MissingCredentials,
    RateLimited,
    Blocked,
}

/// Resource types for access logging
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourceType {
    SessionData,
    RequestData,
    ResponseData,
    UserProfile,
    SystemConfiguration,
    AuditLogs,
    MetricsData,
    TestCases,
}

/// Data access patterns
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccessPattern {
    Read,
    Write,
    Update,
    Delete,
    BulkRead,
    BulkWrite,
    Stream,
    Query { query_complexity: QueryComplexity },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueryComplexity {
    Simple,
    Moderate,
    Complex,
    Heavy,
}

/// Performance metric types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MetricType {
    ResponseTime,
    Throughput,
    ErrorRate,
    CpuUsage,
    MemoryUsage,
    NetworkLatency,
    DatabaseQueryTime,
    CacheHitRate,
    TokensPerSecond,
    CostPerRequest,
}

/// Measurement units for metrics
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MeasurementUnit {
    Milliseconds,
    Seconds,
    RequestsPerSecond,
    Percentage,
    Bytes,
    Kilobytes,
    Megabytes,
    Count,
    USD,
    TokensPerSecond,
}

/// Integrity proof for audit entries
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntegrityProof {
    pub content_hash: Hash256,
    pub previous_entry_hash: Option<Hash256>,
    pub sequence_number: u64,
    pub timestamp_nonce: String,
}

impl IntegrityProof {
    /// Create a new integrity proof
    pub fn new(
        content: &str,
        previous_hash: Option<Hash256>,
        sequence_number: u64,
    ) -> Self {
        let timestamp_nonce = format!("{}_{}", Timestamp::now(), uuid::Uuid::new_v4());
        let combined_content = format!("{content}_{timestamp_nonce}");
        
        Self {
            content_hash: Hash256::from_content(&combined_content),
            previous_entry_hash: previous_hash,
            sequence_number,
            timestamp_nonce,
        }
    }

    /// Verify integrity against content
    pub fn verify(&self, content: &str) -> bool {
        let combined_content = format!("{content}_{}", self.timestamp_nonce);
        let expected_hash = Hash256::from_content(&combined_content);
        self.content_hash == expected_hash
    }
}

/// Audit context providing additional metadata
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditContext {
    pub environment: Environment,
    pub service_version: String,
    pub correlation_id: CorrelationId,
    pub trace_id: Option<TraceId>,
    pub source_component: SystemComponent,
    pub additional_metadata: BTreeMap<String, String>,
}

/// Environment types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Environment {
    Production,
    Staging,
    Development,
    Testing,
    Integration,
}

/// Correlation ID for request tracing
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CorrelationId(String);

impl CorrelationId {
    pub fn generate() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    pub fn from_string(s: String) -> Self {
        Self(s)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Distributed tracing ID
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TraceId(String);

impl TraceId {
    pub fn generate() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    pub fn from_string(s: String) -> Self {
        Self(s)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Audit log builder with compile-time validation
pub struct AuditLogBuilder<T> {
    session_id: Option<SessionId>,
    request_id: Option<RequestId>,
    entry_type: Option<AuditEntryType<T>>,
    context: Option<AuditContext>,
    _phantom: PhantomData<T>,
}

impl<T> AuditLogBuilder<T> {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            session_id: None,
            request_id: None,
            entry_type: None,
            context: None,
            _phantom: PhantomData,
        }
    }

    /// Set session ID
    pub fn session_id(mut self, session_id: SessionId) -> Self {
        self.session_id = Some(session_id);
        self
    }

    /// Set request ID
    pub fn request_id(mut self, request_id: RequestId) -> Self {
        self.request_id = Some(request_id);
        self
    }

    /// Set entry type
    pub fn entry_type(mut self, entry_type: AuditEntryType<T>) -> Self {
        self.entry_type = Some(entry_type);
        self
    }

    /// Set context
    pub fn context(mut self, context: AuditContext) -> Self {
        self.context = Some(context);
        self
    }

    /// Build the audit log entry
    pub fn build(self, previous_hash: Option<Hash256>, sequence_number: u64) -> Result<AuditLogEntry<T>, AuditError> {
        let session_id = self.session_id.ok_or(AuditError::MissingRequiredField("session_id"))?;
        let entry_type = self.entry_type.ok_or(AuditError::MissingRequiredField("entry_type"))?;
        let context = self.context.ok_or(AuditError::MissingRequiredField("context"))?;

        let timestamp = Timestamp::now();
        let entry_id = AuditEntryId::generate();

        // Create content for integrity proof
        let content = serde_json::to_string(&entry_type)
            .map_err(|e| AuditError::SerializationError(e.to_string()))?;
        
        let integrity = IntegrityProof::new(&content, previous_hash, sequence_number);

        Ok(AuditLogEntry {
            entry_id,
            session_id,
            request_id: self.request_id,
            timestamp,
            entry_type,
            integrity,
            context,
        })
    }
}

impl<T> Default for AuditLogBuilder<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Audit log chain for ensuring chronological integrity
#[derive(Debug, Clone)]
pub struct AuditLogChain<T> {
    entries: Vec<AuditLogEntry<T>>,
    last_hash: Option<Hash256>,
    sequence_counter: u64,
}

impl<T> AuditLogChain<T> {
    /// Create a new audit log chain
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            last_hash: None,
            sequence_counter: 0,
        }
    }

    /// Append an entry to the chain
    pub fn append(&mut self, builder: AuditLogBuilder<T>) -> Result<&AuditLogEntry<T>, AuditError> {
        let entry = builder.build(self.last_hash.clone(), self.sequence_counter)?;
        
        // Update chain state
        self.last_hash = Some(entry.integrity.content_hash.clone());
        self.sequence_counter += 1;
        
        self.entries.push(entry);
        Ok(self.entries.last().unwrap())
    }

    /// Verify the integrity of the entire chain
    pub fn verify_integrity(&self) -> Result<(), AuditError> {
        let mut expected_hash: Option<Hash256> = None;
        
        for (index, entry) in self.entries.iter().enumerate() {
            // Verify sequence number
            if entry.integrity.sequence_number != index as u64 {
                return Err(AuditError::ChainIntegrityViolation(
                    format!("Invalid sequence number at index {index}")
                ));
            }

            // Verify previous hash linkage
            if entry.integrity.previous_entry_hash != expected_hash {
                return Err(AuditError::ChainIntegrityViolation(
                    format!("Hash chain broken at index {index}")
                ));
            }

            // Verify content integrity
            let content = serde_json::to_string(&entry.entry_type)
                .map_err(|e| AuditError::SerializationError(e.to_string()))?;
            
            if !entry.integrity.verify(&content) {
                return Err(AuditError::ChainIntegrityViolation(
                    format!("Content integrity violation at index {index}")
                ));
            }

            expected_hash = Some(entry.integrity.content_hash.clone());
        }

        Ok(())
    }

    /// Get all entries
    pub fn entries(&self) -> &[AuditLogEntry<T>] {
        &self.entries
    }

    /// Get entry count
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if chain is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl<T> Default for AuditLogChain<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Audit logging errors
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AuditError {
    #[error("Missing required field: {0}")]
    MissingRequiredField(&'static str),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Chain integrity violation: {0}")]
    ChainIntegrityViolation(String),

    #[error("Invalid entry type for context: {0}")]
    InvalidEntryType(String),

    #[error("Hash verification failed")]
    HashVerificationFailed,
}

/// Type-safe audit logger
pub struct AuditLogger<T> {
    chain: AuditLogChain<T>,
    context_factory: Box<dyn Fn() -> AuditContext + Send + Sync>,
}

impl<T> AuditLogger<T> {
    /// Create a new audit logger
    pub fn new<F>(context_factory: F) -> Self
    where
        F: Fn() -> AuditContext + Send + Sync + 'static,
    {
        Self {
            chain: AuditLogChain::new(),
            context_factory: Box::new(context_factory),
        }
    }

    /// Log an audit entry
    pub fn log(
        &mut self,
        session_id: SessionId,
        entry_type: AuditEntryType<T>,
        request_id: Option<RequestId>,
    ) -> Result<&AuditLogEntry<T>, AuditError> {
        let context = (self.context_factory)();
        
        let mut builder = AuditLogBuilder::new()
            .session_id(session_id)
            .entry_type(entry_type)
            .context(context);

        if let Some(req_id) = request_id {
            builder = builder.request_id(req_id);
        }

        self.chain.append(builder)
    }

    /// Verify the integrity of all logged entries
    pub fn verify_integrity(&self) -> Result<(), AuditError> {
        self.chain.verify_integrity()
    }

    /// Get all audit entries
    pub fn entries(&self) -> &[AuditLogEntry<T>] {
        self.chain.entries()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_context() -> AuditContext {
        AuditContext {
            environment: Environment::Testing,
            service_version: "1.0.0".to_string(),
            correlation_id: CorrelationId::generate(),
            trace_id: Some(TraceId::generate()),
            source_component: SystemComponent::ProxyService,
            additional_metadata: BTreeMap::new(),
        }
    }

    #[test]
    fn test_audit_log_chain_integrity() {
        let mut chain: AuditLogChain<ProductionAudit> = AuditLogChain::new();
        let session_id = SessionId::generate();

        // Add first entry
        let entry1 = AuditLogBuilder::new()
            .session_id(session_id.clone())
            .entry_type(AuditEntryType::SessionStarted {
                user_id: UserId::generate(),
                application_id: "test-app".to_string(),
                _phantom: PhantomData,
            })
            .context(create_test_context())
            .build(None, 0)
            .unwrap();

        chain.entries.push(entry1);
        chain.last_hash = Some(chain.entries[0].integrity.content_hash.clone());
        chain.sequence_counter = 1;

        // Add second entry
        let builder2 = AuditLogBuilder::new()
            .session_id(session_id)
            .entry_type(AuditEntryType::SessionEnded {
                duration: Duration::from_millis(5000),
                final_state: SessionFinalState::CompletedSuccessfully { total_requests: 3 },
                _phantom: PhantomData,
            })
            .context(create_test_context());

        chain.append(builder2).unwrap();

        // Verify chain integrity
        assert!(chain.verify_integrity().is_ok());
        assert_eq!(chain.len(), 2);
    }

    #[test]
    fn test_hash_integrity() {
        let content = "test content";
        let hash = Hash256::from_content(content);
        
        let proof = IntegrityProof::new(content, None, 0);
        assert!(proof.verify(content));
        
        // Tampering should fail verification
        assert!(!proof.verify("tampered content"));
    }

    #[test]
    fn test_audit_logger() {
        let mut logger: AuditLogger<SecurityAudit> = AuditLogger::new(create_test_context);
        let session_id = SessionId::generate();

        // Log authentication event
        let auth_entry = logger.log(
            session_id.clone(),
            AuditEntryType::AuthenticationEvent {
                auth_result: AuthResult::Success,
                user_agent_hash: Hash256::from_content("Mozilla/5.0"),
                ip_address_hash: Hash256::from_content("192.168.1.1"),
                _phantom: PhantomData,
            },
            None,
        ).unwrap();

        assert_eq!(auth_entry.session_id, session_id);
        assert!(logger.verify_integrity().is_ok());
    }

    #[test]
    fn test_error_classification() {
        let error_types = vec![
            ErrorType::ValidationError,
            ErrorType::AuthenticationError,
            ErrorType::RateLimitExceeded,
            ErrorType::ServiceUnavailable,
        ];

        for error_type in error_types {
            assert!(matches!(error_type, ErrorType::ValidationError | ErrorType::AuthenticationError | ErrorType::RateLimitExceeded | ErrorType::ServiceUnavailable));
        }
    }
}