# Union Square - Product Requirements Document

## Product Overview

Union Square is a proxy server that sits between applications and LLM APIs to capture, organize, and analyze AI model interactions. It enables development teams, customer support, and management to monitor, debug, and optimize their AI-powered applications.

## Target Users

### 1. Customer Support Managers (CSMs)
- **Use Case**: Review conversation sessions when customers report issues
- **Key Actions**:
  - Look up specific customer sessions
  - Review full conversation flows
  - Flag problematic sessions for developer review
  - Add notes about customer complaints or observed issues

### 2. Managers/Executives
- **Use Case**: Monitor performance statistics and optimize AI usage
- **Key Actions**:
  - View cost optimization metrics
  - Track precision/recall metrics with F-scores over time
  - Monitor performance by application component
  - Make data-driven decisions about AI model usage

### 3. Developers
- **Use Case**: Debug issues and ensure system reliability
- **Key Actions**:
  - Review flagged interactions from CSMs
  - Extract test cases from problematic conversations
  - Run tests on-demand or on schedule
  - Integrate test suites with CI/CD pipelines
  - Prevent regression in AI behavior

## Technical Architecture

### Session Identification
- Applications pass session identifiers via custom HTTP headers
- Headers are designed to be ignored by actual LLM providers
- Supports flexible metadata for different application types
- No prescriptive customer definition - adaptable to various use cases

### Supported LLM Providers
- **OpenAI API** (including OpenAI-compatible local models)
- **Anthropic API**
- **AWS Bedrock**
- **Google Vertex AI**

## Core Features

### Data Management
- **Configurable retention periods** per application/environment
- **Isolated data spaces** for each application
- **Multi-environment support** with separate configurations

### Test Case Management
- **In-app test execution** - Run individual tests or full suites from the UI
- **Test case extraction** from flagged conversations
- **CI/CD integration** via API
  - Programmatic test execution
  - Wait for results
  - Statistical regression detection
  - GitHub Actions plugin support

### Test Evaluation Methods
- **Deterministic comparisons** - Exact match requirements
- **LLM-as-judge** - Use LLMs to evaluate response quality
- **Custom model analysis** - Plug in specialized evaluation models
- **Range/gradient scoring** - Accept responses within defined thresholds
- **Statistical deviation detection** - Alert when behavior differs significantly from baseline
  - Baseline established over multiple test runs
  - Configurable sliding window for baseline calculation

### User Interface
- **Hybrid approach** - Server-side rendering with reactive components
- **Technology consideration**: Rust-based (e.g., Leptos for LiveView-like experience)
- **Real-time updates** for monitoring active sessions
- **Responsive design** for various screen sizes

### Search & Discovery
- **Session ID search**
- **Time range filtering**
- **Error status filtering**
- **Full-text search** across conversation content
- **Custom metadata search** - Search by any application-defined metadata fields

### Analytics & Dashboards
- **Hourly aggregations** for metrics
- **Cost tracking** by model, application, and time period
- **Performance metrics** - Response times, error rates
- **F-score tracking** - Precision/recall over time
- **Usage patterns** - Peak times, most active features

### Authentication & Security
- **OIDC (OpenID Connect) client** - Connect to any OIDC provider
- **Bring Your Own Identity Provider** - Customers use their existing auth systems
- **Example providers**: Stytch, Auth0, Okta, Google Workspace, Microsoft Azure AD
- **No vendor lock-in** - Standard OIDC implementation

### Authorization
- **Predefined roles** (initial implementation):
  - **Admin** - Full system access, user management, configuration
  - **Developer** - View sessions, create/run tests, access API
  - **CSM** - View sessions, flag issues, add notes
  - **Viewer** - Read-only access to sessions and dashboards
- **Designed for future expansion** to RBAC/granular permissions

### API Key Management & Resilience
- **Pass-through authentication** - Applications provide their own LLM API keys
- **No key storage** - Union Square never stores provider API keys
- **Transparent proxying** - Requests maintain original authentication headers
- **Circuit breaker pattern** - Applications can bypass Union Square if it's down
- **Not a single point of failure** - Direct provider fallback supported

### Deployment Options
- **Self-hosted** - Open source project
- **Self-hosted features**:
  - **Container-based deployment** - Docker/Kubernetes support
  - **Single binary distribution** - Easy deployment with minimal dependencies
  - **Database-level isolation** - Separate databases per deployment
  - **Complete data separation** - Security and compliance focused

### Performance Metrics Capture
- **Latency tracking** - Request/response times
- **Token usage** - Input/output tokens per request
- **Cost calculation** - Based on model and token usage
- **Rate limiting data** - Track API quota usage
- **Error rates** - Provider errors, timeouts, rate limits

### Low-Latency Design Requirements
- **Minimal proxy overhead** - Target < 5ms added latency
- **Asynchronous recording** - Capture data without blocking requests
- **Write-through design** - Forward requests immediately, record in parallel
- **Efficient buffering** - Batch writes to reduce I/O overhead
- **Zero processing in request path** - All analysis happens asynchronously

### Alerting & Notifications
- **Webhook support** - HTTP callbacks for events
- **Configurable alerts**:
  - Error rate thresholds
  - Latency spikes
  - Cost overruns
  - Failed test cases
- **Integration options** - Slack, PagerDuty, email

### Compliance & Security
- **GDPR compliance**:
  - Right to deletion
  - Data export capabilities
  - Consent management
  - Data minimization
- **HIPAA compliance**:
  - Encryption at rest and in transit
  - Audit logging
  - Access controls
  - PHI handling procedures
- **SOC2 compliance**:
  - Security controls
  - Availability monitoring
  - Processing integrity
  - Confidentiality measures
- **Additional security features**:
  - PII detection and masking options
  - Configurable data retention
  - Audit trails for all access

### Data Export
- **Pluggable export system** - Support for custom export formats
- **Built-in formats**:
  - **JSON** - Default export format
- **Plugin architecture** for additional formats:
  - CSV, Parquet, Avro
  - Analytics tool formats (Datadog, Splunk, etc.)
  - Custom formats via plugin API
- **Export triggers**:
  - Manual export via UI/API
  - Scheduled exports
  - Webhook-triggered exports

### Model & API Version Management
- **Version tracking** - Capture model/API version with each request
- **Test configuration**:
  - Run tests against multiple models/versions
  - Compare results across providers
  - A/B testing different models
- **Default behavior** - Tests use original model/version from capture
- **Model migration support** - Test compatibility when switching providers
- **Version comparison reports** - Side-by-side performance analysis

### API Endpoint Structure
- **Provider-specific endpoints** - Mirror each provider's API paths
- **Drop-in replacement** - Just change base URL, keep all paths identical
- **Example endpoints**:
  - OpenAI: `/openai/v1/chat/completions`
  - Anthropic: `/anthropic/v1/messages`
  - Bedrock: `/bedrock/model/invoke`
  - Vertex AI: `/vertex-ai/v1/projects/{project}/locations/{location}/endpoints/{endpoint}`
- **Benefits**:
  - Simple failover between Union Square and direct provider
  - No special headers or routing logic needed
  - Preserves provider-specific API behavior

### Streaming Response Support
- **Pass-through streaming** - SSE/streaming responses flow directly to caller
- **Capture strategy**:
  - Record that response was streamed
  - Capture complete final response
  - Minimal latency impact on stream
- **Replay support** - Can replay as streamed or complete response

### Caching
- **Configurable caching** - Per-application default settings
- **Request-level override** - Headers to control cache behavior
- **Cache transparency**:
  - Clear indication when cached response is used
  - Cache metadata in session logs
  - Cache hit/miss metrics
- **Cache key options** - Based on request content, headers, or custom rules

### Privacy Controls
- **Do-not-record header** - Skip recording for sensitive requests
- **Configurable privacy rules** - Per-application privacy settings
- **PII detection** - Optional automatic detection and masking
- **Audit trail** - Log when privacy controls are activated

### Request Sampling
- **Configurable sampling rate** - Per-application percentage
- **Sampling strategies**:
  - Random sampling
  - Error-focused sampling (100% errors, X% success)
  - Cost-based sampling (sample expensive requests more)
- **Sampling metadata** - Track what was sampled vs skipped

### Developer Tools
- **Request replay** - Re-send captured requests for debugging
- **Replay options**:
  - Use original or different model/provider
  - Modify parameters before replay
  - Compare replay with original response
- **Bulk replay** - Re-run test suites or specific sessions

### Error Handling
- **Pass-through approach** - All provider errors returned as-is
- **Error recording** - Capture all errors for analysis
- **Error types tracked**:
  - Rate limit errors
  - Authentication failures
  - Model availability issues
  - Network timeouts
- **No automatic retries** - Applications maintain control

### Health Monitoring
- **Health check endpoints** - Verify Union Square availability
- **Checks performed**:
  - Database connectivity
  - Provider endpoint reachability
  - Cache system status
  - Queue processing health
- **Integration support** - Compatible with k8s probes, load balancers

## Summary

Union Square is designed to be a transparent, high-performance proxy for LLM API calls that enables teams to:

1. **Monitor and debug** AI-powered applications with full conversation capture
2. **Extract and run test cases** to prevent regressions
3. **Optimize costs and performance** through detailed analytics
4. **Maintain compliance** with comprehensive security and privacy controls
5. **Scale confidently** with minimal latency overhead and failure resilience

The system prioritizes being a true drop-in replacement for direct LLM API calls, ensuring that applications can adopt Union Square without code changes and can bypass it if needed. With its focus on self-hosted deployment, pluggable architecture, and comprehensive feature set, Union Square provides the observability and testing infrastructure that AI-powered applications need to operate reliably in production.
