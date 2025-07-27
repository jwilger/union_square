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
        assert_eq!(http::content_types::TEXT_EVENT_STREAM, "text/event-stream");
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
}
