//! Test data constants for consistent testing
//!
//! This module centralizes all test data constants used throughout the test suite
//! to ensure consistency, maintainability, and reduce duplication.

/// Email addresses for testing
pub mod emails {
    pub const VALID_EMAIL_1: &str = "test@example.com";
    pub const VALID_EMAIL_2: &str = "user@domain.org";
    pub const VALID_EMAIL_3: &str = "admin@company.net";

    pub const INVALID_EMAIL_1: &str = "invalid-email";
    pub const INVALID_EMAIL_2: &str = "a@b"; // Too short
    pub const EMPTY_EMAIL: &str = "";
}

/// Display names for testing
pub mod display_names {
    pub const VALID_NAME_1: &str = "John Doe";
    pub const VALID_NAME_2: &str = "Test User";
    pub const VALID_NAME_3: &str = "Admin User";

    pub const EMPTY_NAME: &str = "";
    pub const WHITESPACE_NAME: &str = "   ";
}

/// LLM model identifiers for testing
pub mod model_ids {
    pub const GPT_4_TURBO: &str = "gpt-4-turbo-2024-01";
    pub const GPT_35_TURBO: &str = "gpt-3.5-turbo";
    pub const CLAUDE_OPUS: &str = "claude-3-opus-20240229";
    pub const CLAUDE_SONNET: &str = "claude-3-sonnet-20240229";
    pub const TITAN_EXPRESS: &str = "amazon.titan-text-express-v1";
    pub const LLAMA_2: &str = "meta.llama2-13b-chat-v1";
}

/// Provider names for testing
pub mod provider_names {
    pub const CUSTOM_PROVIDER_1: &str = "custom_provider";
    pub const CUSTOM_PROVIDER_2: &str = "test-provider-v2";
    pub const CUSTOM_PROVIDER_3: &str = "internal_ai_service";
}

/// Application identifiers for testing
pub mod application_ids {
    pub const MY_APP: &str = "my-app";
    pub const MY_APPLICATION: &str = "MyApplication";
    pub const APP_123: &str = "app_123";
    pub const SINGLE_CHAR: &str = "a";
}

/// Environment identifiers for testing
pub mod environment_ids {
    pub const PRODUCTION: &str = "production";
    pub const STAGING: &str = "staging";
    pub const DEVELOPMENT: &str = "development";
    pub const QA: &str = "qa";
    pub const DEV_123: &str = "dev-123";
}

/// Tag values for testing
pub mod tags {
    pub const PRODUCTION_TAG: &str = "production";
    pub const API_V2_TAG: &str = "api:v2";
    pub const TEST_CASE_TAG: &str = "test-case_1";
    pub const FEATURE_ENABLED_TAG: &str = "feature.enabled";

    pub const INVALID_DASH_START: &str = "-invalid";
    pub const INVALID_SPACE: &str = "invalid ";
}

/// User agent strings for testing
pub mod user_agents {
    pub const MOZILLA: &str = "Mozilla/5.0";
    pub const CHROME: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36";
    pub const FIREFOX: &str =
        "Mozilla/5.0 (X11; Linux x86_64; rv:91.0) Gecko/20100101 Firefox/91.0";
    pub const SAFARI: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15";
}

/// Prompt templates for testing
pub mod prompts {
    pub const SIMPLE_PROMPT: &str = "Test prompt";
    pub const TEMPLATE_PROMPT: &str = "Hello {name}!";
    pub const COMPLEX_PROMPT: &str = "Generate a response about {topic} with {style} tone";
    pub const EMPTY_PROMPT: &str = "";
}

/// Response text for testing
pub mod responses {
    pub const SIMPLE_RESPONSE: &str = "Test response";
    pub const COMPLEX_RESPONSE: &str =
        "This is a more complex response with multiple sentences. It contains various elements.";
    pub const EMPTY_RESPONSE: &str = "";
}

/// Pattern strings for testing
pub mod patterns {
    pub const EXPECTED_OUTPUT: &str = "expected output";
    pub const SUCCESS_PATTERN: &str = "success";
    pub const ERROR_PATTERN: &str = "error";
    pub const JSON_PATTERN: &str = r#"{"status":"ok"}"#;
}

/// Finish reasons for testing
pub mod finish_reasons {
    pub const STOP: &str = "stop";
    pub const LENGTH: &str = "length";
    pub const CONTENT_FILTER: &str = "content_filter";
    pub const MAX_TOKENS: &str = "max_tokens";
}

/// Change reasons for testing
pub mod change_reasons {
    pub const PERFORMANCE_UPGRADE: &str = "Performance upgrade";
    pub const BUG_FIX: &str = "Critical bug fix";
    pub const FEATURE_UPDATE: &str = "New feature release";
    pub const SECURITY_PATCH: &str = "Security vulnerability patch";
}

/// URL strings for testing
pub mod urls {
    pub const EXAMPLE_COM: &str = "https://example.com/test";
    pub const API_ENDPOINT: &str = "https://api.example.com/v1/endpoint";
    pub const USERS_ENDPOINT: &str = "/users/123?param=value";
    pub const HEALTH_CHECK: &str = "/health";
}

/// Error messages for testing
pub mod error_messages {
    pub const TEST_ERROR: &str = "Test error";
    pub const NETWORK_ERROR: &str = "Network connection failed";
    pub const VALIDATION_ERROR: &str = "Validation failed";
    pub const TIMEOUT_ERROR: &str = "Request timed out";
}

/// HTTP status codes for testing (as strings)
pub mod http_status {
    pub const OK: &str = "200";
    pub const NOT_FOUND: &str = "404";
    pub const INTERNAL_ERROR: &str = "500";
    pub const BAD_REQUEST: &str = "400";
}

/// Numeric constants for testing
pub mod numeric {
    /// Token counts
    pub const TOKENS_150: u32 = 150;
    pub const TOKENS_10: u32 = 10;
    pub const TOKENS_5: u32 = 5;

    /// Latency values in milliseconds
    pub const LATENCY_1200_MS: u64 = 1200;
    pub const LATENCY_500_MS: u64 = 500;
    pub const LATENCY_100_MS: u64 = 100;

    /// Temperature values
    pub const TEMPERATURE_07: f64 = 0.7;
    pub const TEMPERATURE_09: f64 = 0.9;
    pub const TEMPERATURE_01: f64 = 0.1;

    /// Max tokens
    pub const MAX_TOKENS_1000: u32 = 1000;
    pub const MAX_TOKENS_2000: u32 = 2000;
    pub const MAX_TOKENS_4000: u32 = 4000;

    /// Port numbers
    pub const PORT_8080: u16 = 8080;
    pub const PORT_5432: u16 = 5432;
    pub const PORT_3000: u16 = 3000;

    /// Connection limits
    pub const MAX_CONNECTIONS_10: u32 = 10;
    pub const MAX_CONNECTIONS_20: u32 = 20;

    /// Batch sizes
    pub const BATCH_SIZE_100: u32 = 100;
    pub const BATCH_SIZE_50: u32 = 50;

    /// Intervals
    pub const FLUSH_INTERVAL_1000_MS: u32 = 1000;
    pub const FLUSH_INTERVAL_500_MS: u32 = 500;
}

/// JSON parameter keys
pub mod json_keys {
    pub const TEMPERATURE: &str = "temperature";
    pub const MAX_TOKENS: &str = "max_tokens";
    pub const TOP_P: &str = "top_p";
    pub const FREQUENCY_PENALTY: &str = "frequency_penalty";
    pub const PRESENCE_PENALTY: &str = "presence_penalty";
}

/// Regex patterns for property testing
pub mod regex_patterns {
    pub const EMAIL_PATTERN: &str = r"[a-z]+@[a-z]+\.[a-z]+";
    pub const NAME_PATTERN: &str = r"[a-zA-Z ]{1,100}";
    pub const IPV4_PATTERN: &str = r"(25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.(25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.(25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.(25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)";
    pub const USER_AGENT_PATTERN: &str = r"[a-zA-Z0-9 /;.()]+";
    pub const TAG_PATTERN: &str = r"[a-zA-Z0-9][a-zA-Z0-9:._-]*";
}

/// Configuration default values
pub mod config_defaults {
    pub const DEFAULT_HOST: &str = "0.0.0.0";
    pub const DEFAULT_DB_HOST: &str = "localhost";
    pub const DEFAULT_DB_USERNAME: &str = "postgres";
    pub const DEFAULT_DB_PASSWORD: &str = "password";
    pub const DEFAULT_DB_NAME: &str = "union_square";
    pub const DEFAULT_ENVIRONMENT: &str = "development";
    pub const DEFAULT_LOG_LEVEL: &str = "info";
    pub const DEFAULT_LOG_FORMAT: &str = "json";
}

/// File paths for testing
pub mod file_paths {
    pub const CONFIG_DEFAULT: &str = "config/default";
    pub const CONFIG_LOCAL: &str = "config/local";
    pub const MIGRATIONS_DIR: &str = "./migrations";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_constants_are_not_empty() {
        // Email constants - these are compile-time constants so we test lengths instead
        assert_eq!(emails::VALID_EMAIL_1.len(), 16); // "test@example.com"
        assert_eq!(emails::INVALID_EMAIL_1.len(), 13); // "invalid-email"

        // Display name constants
        assert_eq!(display_names::VALID_NAME_1.len(), 8); // "John Doe"

        // Model ID constants
        assert_eq!(model_ids::GPT_4_TURBO.len(), 19); // "gpt-4-turbo-2024-01"
        assert_eq!(model_ids::CLAUDE_OPUS.len(), 22); // "claude-3-opus-20240229"

        // Environment constants
        assert_eq!(environment_ids::PRODUCTION.len(), 10); // "production"
        assert_eq!(environment_ids::STAGING.len(), 7); // "staging"

        // URL constants
        assert_eq!(urls::EXAMPLE_COM.len(), 24); // "https://example.com/test"
        assert_eq!(urls::API_ENDPOINT.len(), 35); // "https://api.example.com/v1/endpoint"

        // Regex patterns
        assert!(regex_patterns::EMAIL_PATTERN.len() > 10);
        assert!(regex_patterns::IPV4_PATTERN.len() > 20);

        // Config defaults
        assert_eq!(config_defaults::DEFAULT_HOST.len(), 7); // "0.0.0.0"
        assert_eq!(config_defaults::DEFAULT_DB_HOST.len(), 9); // "localhost"
    }

    #[test]
    fn test_numeric_constants_are_reasonable() {
        // Test specific expected values instead of ranges
        assert_eq!(numeric::TOKENS_150, 150);
        assert_eq!(numeric::LATENCY_1200_MS, 1200);
        assert_eq!(numeric::TEMPERATURE_07, 0.7);
        assert_eq!(numeric::PORT_8080, 8080); // Non-privileged port
        assert_eq!(numeric::MAX_CONNECTIONS_10, 10);
    }

    #[test]
    fn test_email_test_data_is_valid_format() {
        assert!(emails::VALID_EMAIL_1.contains('@'));
        assert!(emails::VALID_EMAIL_2.contains('@'));
        assert!(!emails::INVALID_EMAIL_1.contains('@'));
    }

    #[test]
    fn test_model_ids_follow_naming_conventions() {
        // GPT models should contain "gpt"
        assert!(model_ids::GPT_4_TURBO.contains("gpt"));
        assert!(model_ids::GPT_35_TURBO.contains("gpt"));

        // Claude models should contain "claude"
        assert!(model_ids::CLAUDE_OPUS.contains("claude"));
        assert!(model_ids::CLAUDE_SONNET.contains("claude"));

        // Check version patterns
        assert!(model_ids::GPT_4_TURBO.contains("2024"));
        assert!(model_ids::CLAUDE_OPUS.contains("20240229"));
    }
}
