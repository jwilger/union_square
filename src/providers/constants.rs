//! Constants for provider interactions
//!
//! This module centralizes all string literals, JSON field names, and other
//! constants used across provider implementations to prevent duplication
//! and make maintenance easier.

/// JSON field names used in model responses across different providers
pub mod json_fields {
    /// Claude model response fields
    pub mod claude {
        pub const USAGE: &str = "usage";
        pub const INPUT_TOKENS: &str = "input_tokens";
        pub const OUTPUT_TOKENS: &str = "output_tokens";
        pub const CONTENT: &str = "content";
        pub const TYPE: &str = "type";
        pub const TEXT: &str = "text";
    }

    /// Titan model response fields
    pub mod titan {
        pub const INPUT_TEXT_TOKEN_COUNT: &str = "inputTextTokenCount";
        pub const RESULTS: &str = "results";
        pub const TOKEN_COUNT: &str = "tokenCount";
        pub const OUTPUT_TEXT: &str = "outputText";
    }

    /// Llama model response fields
    pub mod llama {
        pub const GENERATION_TOKEN_COUNT: &str = "generation_token_count";
        pub const PROMPT_TOKEN_COUNT: &str = "prompt_token_count";
        pub const GENERATION: &str = "generation";
    }

    /// Jurassic model response fields
    pub mod jurassic {
        pub const COMPLETIONS: &str = "completions";
        pub const DATA: &str = "data";
        pub const TOKENS: &str = "tokens";
        pub const GENERATED_TOKENS: &str = "generated_tokens";
    }

    /// Command model response fields
    pub mod command {
        pub const PROMPT_TOKENS: &str = "prompt_tokens";
        pub const COMPLETION_TOKENS: &str = "completion_tokens";
    }

    /// Common fields used across providers
    pub mod common {
        pub const MODEL: &str = "model";
        pub const ERROR: &str = "error";
        pub const MESSAGE: &str = "message";
        pub const ID: &str = "id";
        pub const TYPE: &str = "type";
    }
}

/// Path constants for routing and URL manipulation
pub mod paths {
    /// Bedrock API path components
    pub mod bedrock {
        pub const MODEL_SEGMENT: &str = "model";
        pub const INVOKE_ENDPOINT: &str = "invoke";
        pub const INVOKE_STREAM_ENDPOINT: &str = "invoke-with-response-stream";

        /// Base path pattern for bedrock model endpoints
        pub const MODEL_PATH_PREFIX: &str = "/bedrock/model/";
    }
}

/// HTTP-related constants
pub mod http {
    /// Content type constants
    pub mod content_types {
        pub const APPLICATION_JSON: &str = "application/json";
        pub const APPLICATION_OCTET_STREAM: &str = "application/octet-stream";
        pub const TEXT_EVENT_STREAM: &str = "text/event-stream";
        pub const TEXT_PLAIN: &str = "text/plain";
        pub const TEXT_HTML: &str = "text/html; charset=utf-8";
    }

    /// Header names specific to providers
    pub mod headers {
        /// AWS-specific headers
        pub mod aws {
            pub const AMZ_TARGET: &str = "x-amz-target";
            pub const AMZ_DATE: &str = "x-amz-date";
            pub const AMZ_SECURITY_TOKEN: &str = "x-amz-security-token";
            pub const AMZ_CONTENT_SHA256: &str = "x-amz-content-sha256";
        }
    }
}

/// Error message constants
pub mod error_messages {
    pub const DATABASE_HEALTH_CHECK_FAILED: &str = "Database health check failed";
    pub const INVALID_MODEL_RESPONSE: &str = "Invalid model response format";
    pub const MISSING_TOKEN_USAGE: &str = "Token usage information not found in response";
    pub const AUTHENTICATION_FAILED: &str = "Authentication failed";
    pub const INVALID_REQUEST_FORMAT: &str = "Invalid request format";
    pub const INVALID_ERROR_MESSAGE: &str = "Invalid error message";
    pub const UNKNOWN_FIELD: &str = "unknown_field";
    pub const UNKNOWN_RESOURCE: &str = "unknown_resource";
}

/// SQL constants for database operations
pub mod sql {
    pub const HEALTH_CHECK_QUERY: &str = "SELECT 1 as health_check";
    pub const HEALTH_CHECK_COLUMN: &str = "health_check";
    pub const HEALTH_CHECK_EXPECTED_VALUE: i32 = 1;
}

/// Configuration defaults and limits
pub mod limits {
    /// Token count limits for different models
    pub const MAX_CLAUDE_TOKENS: u32 = 200_000;
    pub const MAX_TITAN_TOKENS: u32 = 8_000;
    pub const MAX_LLAMA_TOKENS: u32 = 4_096;

    /// Request/response size limits
    pub const MAX_REQUEST_SIZE: usize = 10 * 1024 * 1024; // 10MB
    pub const MAX_RESPONSE_SIZE: usize = 10 * 1024 * 1024; // 10MB
}

/// Model detection patterns for identifying model families
pub mod model_patterns {
    /// Claude model detection
    pub const CLAUDE: &str = "claude";

    /// Titan model detection
    pub const TITAN: &str = "titan";

    /// Llama model detection
    pub const LLAMA: &str = "llama";

    /// Jurassic model detection patterns
    pub const JURASSIC_J2: &str = "j2";
    pub const JURASSIC_FULL: &str = "jurassic";

    /// Command model detection
    pub const COMMAND: &str = "command";

    /// Stable Diffusion detection
    pub const STABLE: &str = "stable";
}

/// Provider identification constants
pub mod provider_ids {
    /// Bedrock provider identifier
    pub const BEDROCK: &str = "bedrock";

    /// OpenAI provider identifier
    pub const OPENAI: &str = "openai";

    /// Anthropic provider identifier
    pub const ANTHROPIC: &str = "anthropic";

    /// Google provider identifier
    pub const GOOGLE: &str = "google";

    /// Azure provider identifier
    pub const AZURE: &str = "azure";
}

/// Configuration file paths
pub mod config_paths {
    /// Default configuration file
    pub const DEFAULT: &str = "config/default";

    /// Local overrides configuration
    pub const LOCAL: &str = "config/local";

    /// Environment variable prefix
    pub const ENV_PREFIX: &str = "UNION_SQUARE";

    /// Environment variable separator
    pub const ENV_SEPARATOR: &str = "__";
}

/// Database migration paths
pub mod migration_paths {
    /// Default migrations directory
    pub const MIGRATIONS_DIR: &str = "./migrations";
}

/// Environment names
pub mod environments {
    /// Production environment
    pub const PRODUCTION: &str = "production";

    /// Staging environment
    pub const STAGING: &str = "staging";

    /// Development environment (default)
    pub const DEVELOPMENT: &str = "development";

    /// QA/Testing environment
    pub const QA: &str = "qa";
}

/// Default configuration values
pub mod config_defaults {
    /// Default application host
    pub const APP_HOST: &str = "0.0.0.0";

    /// Default application port
    pub const APP_PORT: i64 = 8080;

    /// Default database host
    pub const DB_HOST: &str = "localhost";

    /// Default database port
    pub const DB_PORT: i64 = 5432;

    /// Default database username
    pub const DB_USERNAME: &str = "postgres";

    /// Default database password
    pub const DB_PASSWORD: &str = "password";

    /// Default database name
    pub const DB_NAME: &str = "union_square";

    /// Default max database connections
    pub const DB_MAX_CONNECTIONS: i64 = 10;

    /// Default EventCore batch size
    pub const EVENTCORE_BATCH_SIZE: i64 = 100;

    /// Default EventCore flush interval
    pub const EVENTCORE_FLUSH_INTERVAL: i64 = 1000;

    /// Default log level
    pub const LOG_LEVEL: &str = "info";

    /// Default log format
    pub const LOG_FORMAT: &str = "json";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_field_constants_have_expected_values() {
        // Test that field names match expected API conventions
        assert_eq!(json_fields::claude::USAGE, "usage");
        assert_eq!(json_fields::claude::INPUT_TOKENS, "input_tokens");
        assert_eq!(json_fields::claude::OUTPUT_TOKENS, "output_tokens");

        assert_eq!(
            json_fields::titan::INPUT_TEXT_TOKEN_COUNT,
            "inputTextTokenCount"
        );
        assert_eq!(json_fields::titan::RESULTS, "results");

        assert_eq!(json_fields::common::MODEL, "model");
        assert_eq!(json_fields::common::ERROR, "error");
    }

    #[test]
    fn test_path_constants_have_expected_values() {
        assert_eq!(paths::bedrock::MODEL_SEGMENT, "model");
        assert_eq!(paths::bedrock::INVOKE_ENDPOINT, "invoke");
        assert_eq!(
            paths::bedrock::INVOKE_STREAM_ENDPOINT,
            "invoke-with-response-stream"
        );
        assert!(paths::bedrock::MODEL_PATH_PREFIX.starts_with('/'));
        assert!(paths::bedrock::MODEL_PATH_PREFIX.ends_with('/'));
    }

    #[test]
    fn test_sql_constants_have_expected_values() {
        assert_eq!(sql::HEALTH_CHECK_QUERY, "SELECT 1 as health_check");
        assert_eq!(sql::HEALTH_CHECK_COLUMN, "health_check");
        assert_eq!(sql::HEALTH_CHECK_EXPECTED_VALUE, 1);
    }

    #[test]
    fn test_http_constants_have_expected_values() {
        assert_eq!(http::content_types::APPLICATION_JSON, "application/json");
        assert_eq!(
            http::content_types::APPLICATION_OCTET_STREAM,
            "application/octet-stream"
        );
        assert_eq!(http::content_types::TEXT_EVENT_STREAM, "text/event-stream");
        assert_eq!(http::content_types::TEXT_PLAIN, "text/plain");
        assert_eq!(http::content_types::TEXT_HTML, "text/html; charset=utf-8");
        assert_eq!(http::headers::aws::AMZ_TARGET, "x-amz-target");
    }

    #[test]
    fn test_limits_have_expected_values() {
        // Test specific values to document expected limits
        assert_eq!(limits::MAX_CLAUDE_TOKENS, 200_000);
        assert_eq!(limits::MAX_TITAN_TOKENS, 8_000);
        assert_eq!(limits::MAX_LLAMA_TOKENS, 4_096);
        assert_eq!(limits::MAX_REQUEST_SIZE, limits::MAX_RESPONSE_SIZE);
        assert_eq!(limits::MAX_REQUEST_SIZE, 10 * 1024 * 1024); // 10MB
    }

    #[test]
    fn test_model_patterns_are_defined() {
        // Verify model detection patterns
        assert_eq!(model_patterns::CLAUDE, "claude");
        assert_eq!(model_patterns::TITAN, "titan");
        assert_eq!(model_patterns::LLAMA, "llama");
        assert_eq!(model_patterns::JURASSIC_J2, "j2");
        assert_eq!(model_patterns::JURASSIC_FULL, "jurassic");
        assert_eq!(model_patterns::COMMAND, "command");
        assert_eq!(model_patterns::STABLE, "stable");
    }

    #[test]
    fn test_provider_ids_are_defined() {
        // Verify provider identifiers
        assert_eq!(provider_ids::BEDROCK, "bedrock");
        assert_eq!(provider_ids::OPENAI, "openai");
        assert_eq!(provider_ids::ANTHROPIC, "anthropic");
        assert_eq!(provider_ids::GOOGLE, "google");
        assert_eq!(provider_ids::AZURE, "azure");
    }

    #[test]
    fn test_config_paths_are_defined() {
        // Verify configuration paths
        assert_eq!(config_paths::DEFAULT, "config/default");
        assert_eq!(config_paths::LOCAL, "config/local");
        assert_eq!(config_paths::ENV_PREFIX, "UNION_SQUARE");
        assert_eq!(config_paths::ENV_SEPARATOR, "__");
    }

    #[test]
    fn test_environments_are_defined() {
        // Verify environment names
        assert_eq!(environments::PRODUCTION, "production");
        assert_eq!(environments::STAGING, "staging");
        assert_eq!(environments::DEVELOPMENT, "development");
        assert_eq!(environments::QA, "qa");
    }

    #[test]
    fn test_config_defaults_are_reasonable() {
        // Verify default configuration values
        assert_eq!(config_defaults::APP_HOST, "0.0.0.0");
        assert_eq!(config_defaults::APP_PORT, 8080);
        assert_eq!(config_defaults::DB_HOST, "localhost");
        assert_eq!(config_defaults::DB_PORT, 5432);
        assert_eq!(config_defaults::DB_USERNAME, "postgres");
        assert_eq!(config_defaults::DB_PASSWORD, "password");
        assert_eq!(config_defaults::DB_NAME, "union_square");
        assert_eq!(config_defaults::DB_MAX_CONNECTIONS, 10);
        assert_eq!(config_defaults::EVENTCORE_BATCH_SIZE, 100);
        assert_eq!(config_defaults::EVENTCORE_FLUSH_INTERVAL, 1000);
        assert_eq!(config_defaults::LOG_LEVEL, "info");
        assert_eq!(config_defaults::LOG_FORMAT, "json");
    }
}
