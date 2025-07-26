# 0013. Test Execution Architecture

- Status: accepted
- Deciders: John Wilger, Technical Architecture Team
- Date: 2025-07-15

## Context

Union Square must support extracting test cases from captured sessions and executing them to detect regressions. Requirements include:

1. Multiple evaluation methods (deterministic, LLM-as-judge, custom models)
2. Test execution via UI and API (CI/CD integration)
3. Version comparison (test across different models/providers)
4. Statistical analysis of results over time
5. Efficient execution of large test suites
6. Extensible for future evaluation methods

Test cases vary widely:
- Simple: Exact response matching
- Complex: Semantic similarity evaluation
- Statistical: Behavior within acceptable ranges

## Decision

We will implement a pluggable test execution architecture with multiple evaluation strategies:

### Test Case Structure

```rust
struct TestCase {
    id: TestCaseId,
    name: String,
    description: String,

    // Captured from original session
    source_session_id: SessionId,
    source_request: CapturedRequest,
    source_response: CapturedResponse,

    // Test configuration
    evaluation_method: EvaluationMethod,
    evaluation_config: serde_json::Value,

    // Execution settings
    target_providers: Vec<ProviderId>,  // Which providers to test against
    timeout: Duration,
    retries: u32,

    // Metadata
    tags: Vec<String>,
    created_at: DateTime<Utc>,
    created_by: UserId,
}

enum EvaluationMethod {
    Deterministic,      // Exact match
    LlmJudge,          // Use LLM to evaluate
    Statistical,       // Within statistical bounds
    Custom(String),    // Plugin name
}
```

### Evaluation Strategy Pattern

```rust
trait EvaluationStrategy: Send + Sync {
    /// Unique identifier for this strategy
    fn id(&self) -> &str;

    /// Evaluate a test execution result
    async fn evaluate(
        &self,
        test_case: &TestCase,
        expected: &CapturedResponse,
        actual: &ExecutionResponse,
        config: &serde_json::Value,
    ) -> EvaluationResult;

    /// Validate configuration for this strategy
    fn validate_config(&self, config: &serde_json::Value) -> Result<(), ValidationError>;
}

struct EvaluationResult {
    passed: bool,
    score: f32,  // 0.0 to 1.0
    details: serde_json::Value,
    explanation: Option<String>,
}
```

### Built-in Strategies

1. **Deterministic Evaluation**
   ```rust
   struct DeterministicStrategy {
       // Configuration options
       ignore_fields: Vec<JsonPath>,
       normalize_whitespace: bool,
       ignore_order: bool,
   }
   ```

2. **LLM-as-Judge Evaluation**
   ```rust
   struct LlmJudgeStrategy {
       judge_model: ModelConfig,
       prompt_template: String,
       scoring_rubric: serde_json::Value,
       temperature: f32,
   }
   ```

3. **Statistical Evaluation**
   ```rust
   struct StatisticalStrategy {
       baseline_window: Duration,
       confidence_level: f32,
       metrics: Vec<MetricConfig>,
   }
   ```

### Test Execution Flow

```rust
struct TestExecutor {
    provider_registry: ProviderRegistry,
    strategy_registry: StrategyRegistry,
    result_store: Box<dyn ResultStore>,
}

impl TestExecutor {
    async fn execute_test(&self, test_case: &TestCase) -> Result<TestResult, TestError> {
        // 1. Prepare request from test case
        let request = self.prepare_request(&test_case.source_request);

        // 2. Execute against target providers
        let mut executions = Vec::new();
        for provider in &test_case.target_providers {
            let response = self.execute_request(provider, &request).await;
            executions.push(response);
        }

        // 3. Evaluate results
        let strategy = self.strategy_registry.get(&test_case.evaluation_method)?;
        let mut evaluations = Vec::new();
        for execution in executions {
            let result = strategy.evaluate(
                &test_case,
                &test_case.source_response,
                &execution,
                &test_case.evaluation_config,
            ).await;
            evaluations.push(result);
        }

        // 4. Store results
        let test_result = TestResult {
            test_case_id: test_case.id,
            executions,
            evaluations,
            timestamp: Utc::now(),
        };

        self.result_store.store(&test_result).await?;
        Ok(test_result)
    }
}
```

### Test Suite Management

```rust
struct TestSuite {
    id: TestSuiteId,
    name: String,
    test_cases: Vec<TestCaseId>,
    schedule: Option<Schedule>,  // For automated execution

    // Execution configuration
    parallel_execution: bool,
    max_parallel: usize,
    stop_on_failure: bool,

    // Notification settings
    notify_on: NotificationTriggers,
    notification_channels: Vec<NotificationChannel>,
}
```

### CI/CD Integration API

```rust
// REST API endpoints
POST   /api/v1/test-suites/{id}/execute
GET    /api/v1/test-executions/{id}
GET    /api/v1/test-executions/{id}/wait  // Long polling for CI

// Response includes
{
  "execution_id": "...",
  "status": "completed",
  "passed": 45,
  "failed": 2,
  "duration_ms": 3240,
  "results": [...],
  "statistics": {
    "regression_detected": true,
    "confidence": 0.95
  }
}
```

## Consequences

### Positive

- Extensible evaluation methods
- Supports diverse testing needs
- CI/CD friendly
- Statistical regression detection
- Parallel execution support
- Provider comparison testing

### Negative

- Complex configuration for advanced strategies
- LLM-as-judge adds latency and cost
- Statistical baselines need historical data
- Plugin system adds complexity
- Test data management overhead

### Mitigation Strategies

1. **Strategy Templates**: Pre-configured common patterns
2. **Test Caching**: Cache LLM judge results for identical inputs
3. **Baseline Bootstrapping**: Use initial runs to build baselines
4. **Strategy Validation**: Validate configs before execution
5. **Result Compression**: Compress stored test results

## Alternatives Considered

1. **Simple Diff-Based Testing**
   - Only support exact matching
   - Rejected: Too limiting for AI applications

2. **External Test Runners**
   - Integrate with existing test frameworks
   - Rejected: Poor integration, limited control

3. **Record-Replay Only**
   - Just replay requests without evaluation
   - Rejected: No regression detection

4. **Built-in Strategies Only**
   - No plugin system
   - Rejected: Can't adapt to new evaluation methods

5. **Separate Test Service**
   - Decouple testing from proxy
   - Rejected: Complex deployment, data synchronization

## Related Decisions

- ADR-0007: EventCore as Central Audit Mechanism (test results as events)
- ADR-0011: Provider Abstraction (test execution uses same providers)
- ADR-0010: Tiered Projection Strategy (test results storage)
