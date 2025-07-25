# ADR-0014: Privacy and Compliance Architecture

- Status: accepted
- Deciders: John Wilger, Technical Architecture Team
- Date: 2025-07-15

## Context

Union Square processes sensitive data including customer conversations, PII, and potentially regulated information (PHI, financial data). We must support:

1. GDPR compliance (right to deletion, data portability)
2. HIPAA compliance for healthcare customers
3. SOC2 requirements
4. PII detection and handling
5. Do-not-record capabilities
6. Data residency requirements
7. Audit trails for all data access

Key challenge: Implement privacy controls without impacting the <5ms latency requirement.

## Decision

We will implement a multi-layered privacy architecture with controls at different stages:

### Privacy Control Layers

1. **Request-Time Controls (Hot Path)**
   - Minimal overhead checks only
   - Do-not-record header processing
   - Basic rate limiting

2. **Async Privacy Processing (Audit Path)**
   - PII detection and masking
   - Compliance rule evaluation
   - Data classification

3. **Storage Controls**
   - Encryption at rest
   - Retention policies
   - Access controls

### Do-Not-Record Implementation

```rust
// Checked in hot path - must be fast
fn should_record(headers: &HeaderMap) -> bool {
    !headers.contains_key("x-unionsquare-do-not-record")
}

// If do-not-record is set:
// 1. Forward request normally
// 2. Skip ring buffer write
// 3. Log metadata only (no content)
// 4. Record DNR event for audit
```

### PII Detection and Masking

Implemented in audit path using pluggable detectors:

```rust
trait PiiDetector: Send + Sync {
    fn detect(&self, text: &str) -> Vec<PiiMatch>;
}

struct PiiMatch {
    start: usize,
    end: usize,
    pii_type: PiiType,
    confidence: f32,
}

enum PiiType {
    Email,
    PhoneNumber,
    SocialSecurityNumber,
    CreditCardNumber,
    IpAddress,
    Name,
    Address,
    Custom(String),
}

// Configurable masking strategies
enum MaskingStrategy {
    Replace(String),      // Replace with fixed string
    Hash,                // One-way hash
    Tokenize,           // Reversible tokenization
    Partial,            // Show partial (last 4 digits)
    Remove,             // Complete removal
}
```

### Data Classification

```rust
struct DataClassification {
    sensitivity: SensitivityLevel,
    regulations: Vec<Regulation>,
    retention_days: u32,
    allowed_regions: Vec<Region>,
    access_roles: Vec<Role>,
}

enum SensitivityLevel {
    Public,
    Internal,
    Confidential,
    Restricted,
}

enum Regulation {
    None,
    Gdpr,
    Hipaa,
    Pci,
    Sox,
    Custom(String),
}
```

### Retention and Deletion

EventCore-based retention with automated cleanup:

```rust
struct RetentionPolicy {
    application_id: ApplicationId,
    environment: Environment,

    // Base retention
    default_retention_days: u32,

    // Overrides by classification
    overrides: HashMap<SensitivityLevel, u32>,

    // GDPR deletion support
    deletion_grace_period_days: u32,
}

// Deletion types
enum DeletionType {
    Expired,          // Normal retention expiry
    UserRequested,    // GDPR right to deletion
    Regulatory,       // Compliance requirement
    Emergency,        // Security incident
}
```

### Audit Trail

Every data access is logged:

```rust
struct DataAccessEvent {
    accessor: UserId,
    timestamp: DateTime<Utc>,
    resource_type: ResourceType,
    resource_id: String,
    action: AccessAction,
    justification: Option<String>,
    ip_address: IpAddr,
    session_id: Uuid,
}

enum AccessAction {
    View,
    Export,
    Modify,
    Delete,
    Share,
}
```

### Encryption Architecture

```toml
[encryption]
# Data at rest
at_rest_algorithm = "AES-256-GCM"
key_derivation = "Argon2id"
key_rotation_days = 90

# Application-level encryption for sensitive fields
field_encryption_enabled = true
field_encryption_keys = "vault://encryption-keys"

# TLS configuration
tls_min_version = "1.3"
cipher_suites = ["TLS_AES_256_GCM_SHA384", "TLS_CHACHA20_POLY1305_SHA256"]
```

### Privacy Configuration

```toml
[privacy]
pii_detection_enabled = true
pii_detection_providers = ["built-in", "custom-medical"]

[privacy.masking]
email = { strategy = "partial", show_domain = true }
phone = { strategy = "partial", show_last = 4 }
ssn = { strategy = "tokenize" }
credit_card = { strategy = "partial", show_last = 4 }

[privacy.do_not_record]
honor_header = true
log_metadata = true
audit_events = true
```

### HIPAA Compliance Features

- Encryption in transit and at rest
- Access controls with role-based permissions
- Comprehensive audit logging
- Automatic session timeouts
- Data integrity checks
- Backup and disaster recovery
- Business Associate Agreements (BAA) support

### GDPR Compliance Features

- Right to access (data export API)
- Right to deletion (cascade through all projections)
- Right to rectification (update captured data)
- Privacy by design (minimal data collection)
- Consent management (via application metadata)
- Data portability (standard export formats)

## Consequences

### Positive

- Comprehensive privacy controls
- Flexible configuration per application
- Audit trail for compliance
- Minimal hot path impact
- Extensible detection system
- Strong encryption throughout

### Negative

- Complex configuration management
- PII detection adds latency to audit path
- Storage overhead from encryption
- Key management complexity
- Testing compliance is difficult

### Mitigation Strategies

1. **Performance**: Run PII detection in parallel workers
2. **Configuration**: Provide compliance templates (HIPAA, GDPR)
3. **Key Management**: Integrate with standard KMS solutions
4. **Testing**: Compliance test suites and scanners
5. **Documentation**: Clear compliance guides

## Alternatives Considered

1. **All Privacy Checks in Hot Path**
   - Complete filtering before forwarding
   - Rejected: Would violate latency requirements

2. **External Privacy Service**
   - Delegate to specialized service
   - Rejected: Additional dependency, latency

3. **No PII Detection**
   - Leave to applications
   - Rejected: Compliance requires proxy-level controls

4. **Fixed Retention Only**
   - No configurable retention
   - Rejected: Different regulations require flexibility

5. **Encryption Only at Database**
   - Rely on database encryption
   - Rejected: Need field-level encryption for some data

## Implementation Notes

- Use tried-and-tested crypto libraries (ring, sodiumoxide)
- Regular security audits required
- PII detection models need regular updates
- Consider regional deployment for data residency
- Document compliance mappings clearly

## Related Decisions

- ADR-0008: Dual-path Architecture (privacy processing in audit path)
- ADR-0007: EventCore as Central Audit Mechanism (audit events)
- ADR-0006: Authentication and Authorization (access controls)
