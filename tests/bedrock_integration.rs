//! Integration tests for AWS Bedrock provider
//!
//! These tests verify the Bedrock provider integration including:
//! - Provider registration and routing
//! - URL transformation
//! - Authentication validation
//! - Metadata extraction
//! - Response processing

use axum::body::Body;
use hyper::{Request, Response, StatusCode};
use std::sync::Arc;
use union_square::providers::{
    bedrock::{types::AwsRegion, BedrockProvider},
    response_processor::ProviderResponseProcessor,
    Provider, ProviderRegistry,
};

/// Create a test request with required SigV4 headers
fn create_test_request(path: &str, body: serde_json::Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(path)
        .header("content-type", "application/json")
        .header("authorization", "AWS4-HMAC-SHA256 Credential=AKIAIOSFODNN7EXAMPLE/20250127/us-east-1/bedrock/aws4_request, SignedHeaders=content-type;host;x-amz-date, Signature=example")
        .header("x-amz-date", "20250127T120000Z")
        .header("x-amz-content-sha256", "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")
        .body(Body::from(body.to_string()))
        .unwrap()
}

#[test]
fn test_provider_registration() {
    let mut registry = ProviderRegistry::new();
    let bedrock_provider = Arc::new(BedrockProvider::new(
        AwsRegion::try_new("us-east-1").unwrap(),
    ));
    registry.register(bedrock_provider);

    // Test routing to Bedrock
    assert!(registry.route("/bedrock/model/test/invoke").is_some());
    assert!(registry.route("/bedrock/anything").is_some());

    // Test non-Bedrock paths
    assert!(registry.route("/openai/v1/chat").is_none());
    assert!(registry.route("/anthropic/v1/messages").is_none());
}

#[test]
fn test_url_transformation() {
    let provider = BedrockProvider::new(AwsRegion::try_new("us-west-2").unwrap());

    // Test basic transformation
    let input_uri: hyper::Uri = "/bedrock/model/test/invoke".parse().unwrap();
    let output_uri = provider.transform_url(&input_uri).unwrap();
    assert_eq!(
        output_uri.to_string(),
        "https://bedrock-runtime.us-west-2.amazonaws.com/model/test/invoke"
    );

    // Test streaming endpoint
    let input_uri: hyper::Uri = "/bedrock/model/claude/invoke-with-response-stream"
        .parse()
        .unwrap();
    let output_uri = provider.transform_url(&input_uri).unwrap();
    assert_eq!(
        output_uri.to_string(),
        "https://bedrock-runtime.us-west-2.amazonaws.com/model/claude/invoke-with-response-stream"
    );
}

#[test]
fn test_provider_metadata_extraction() {
    let provider = BedrockProvider::new(AwsRegion::try_new("us-east-1").unwrap());

    // Create mock request and response
    let request = create_test_request(
        "/bedrock/model/anthropic.claude-3-sonnet-20240229/invoke",
        serde_json::json!({
            "messages": [{
                "role": "user",
                "content": "Hello!"
            }]
        }),
    );

    let response = Response::builder()
        .status(StatusCode::OK)
        .header("x-amzn-requestid", "test-request-123")
        .header("content-type", "application/json")
        .body(Body::empty())
        .unwrap();

    let metadata = provider.extract_metadata(&request, &response);

    assert_eq!(metadata.provider_id, "bedrock");
    assert_eq!(
        metadata.model_id,
        Some("anthropic.claude-3-sonnet-20240229".to_string())
    );
    assert_eq!(
        metadata.provider_request_id,
        Some("test-request-123".to_string())
    );
}

#[test]
fn test_response_processor_claude() {
    let base_metadata = union_square::providers::ProviderMetadata {
        provider_id: "bedrock".to_string(),
        model_id: Some("anthropic.claude-3-sonnet-20240229".to_string()),
        ..Default::default()
    };

    let processor = ProviderResponseProcessor::new(base_metadata);

    let response_body = serde_json::json!({
        "id": "msg_123",
        "type": "message",
        "role": "assistant",
        "content": [{
            "type": "text",
            "text": "Hello from Claude!"
        }],
        "usage": {
            "input_tokens": 25,
            "output_tokens": 15
        }
    });

    let body_bytes = response_body.to_string().into_bytes();
    let metadata = processor.process_complete_body(&body_bytes);

    assert_eq!(metadata.request_tokens, Some(25));
    assert_eq!(metadata.response_tokens, Some(15));
    assert_eq!(metadata.total_tokens, Some(40));
    assert!(metadata.cost_estimate.is_some());

    // Verify cost calculation (Claude 3 Sonnet: $0.003/1K input, $0.015/1K output)
    let expected_cost = (25.0 / 1000.0) * 0.003 + (15.0 / 1000.0) * 0.015;
    assert!((metadata.cost_estimate.unwrap() - expected_cost).abs() < f64::EPSILON);
}

#[test]
fn test_response_processor_titan() {
    let base_metadata = union_square::providers::ProviderMetadata {
        provider_id: "bedrock".to_string(),
        model_id: Some("amazon.titan-text-express-v1".to_string()),
        ..Default::default()
    };

    let processor = ProviderResponseProcessor::new(base_metadata);

    let response_body = serde_json::json!({
        "inputTextTokenCount": 20,
        "results": [{
            "tokenCount": 30,
            "outputText": "Hello from Titan!",
            "completionReason": "FINISH"
        }]
    });

    let body_bytes = response_body.to_string().into_bytes();
    let metadata = processor.process_complete_body(&body_bytes);

    assert_eq!(metadata.request_tokens, Some(20));
    assert_eq!(metadata.response_tokens, Some(30));
    assert_eq!(metadata.total_tokens, Some(50));

    // Verify Titan Express cost ($0.0008/1K input, $0.0016/1K output)
    let expected_cost = (20.0 / 1000.0) * 0.0008 + (30.0 / 1000.0) * 0.0016;
    assert!((metadata.cost_estimate.unwrap() - expected_cost).abs() < f64::EPSILON);
}

#[test]
fn test_response_processor_llama() {
    let base_metadata = union_square::providers::ProviderMetadata {
        provider_id: "bedrock".to_string(),
        model_id: Some("meta.llama3-70b-instruct-v1".to_string()),
        ..Default::default()
    };

    let processor = ProviderResponseProcessor::new(base_metadata);

    let response_body = serde_json::json!({
        "generation": "Hello from Llama!",
        "prompt_token_count": 15,
        "generation_token_count": 10,
        "stop_reason": "stop"
    });

    let body_bytes = response_body.to_string().into_bytes();
    let metadata = processor.process_complete_body(&body_bytes);

    assert_eq!(metadata.request_tokens, Some(15));
    assert_eq!(metadata.response_tokens, Some(10));
    assert_eq!(metadata.total_tokens, Some(25));

    // Verify Llama 70B cost ($0.00265/1K input, $0.0035/1K output)
    let expected_cost = (15.0 / 1000.0) * 0.00265 + (10.0 / 1000.0) * 0.0035;
    assert!((metadata.cost_estimate.unwrap() - expected_cost).abs() < f64::EPSILON);
}

#[test]
fn test_authentication_validation() {
    // Test that provider checks for required auth headers
    let provider = BedrockProvider::new(AwsRegion::try_new("us-east-1").unwrap());

    assert_eq!(provider.id(), "bedrock");
    assert!(provider.matches_path("/bedrock/test"));

    // Note: Actual authentication validation happens in forward_request
    // which requires async context and would be tested with mocked HTTP client
}

#[test]
fn test_invalid_path_handling() {
    let provider = BedrockProvider::new(AwsRegion::try_new("us-east-1").unwrap());

    // Test path without /bedrock prefix
    let input_uri: hyper::Uri = "/model/test/invoke".parse().unwrap();
    let result = provider.transform_url(&input_uri);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Missing /bedrock prefix"));
}

#[test]
fn test_model_family_detection() {
    use union_square::providers::bedrock::types::ModelFamily;

    assert_eq!(
        ModelFamily::from_model_id("anthropic.claude-3-opus"),
        ModelFamily::Claude
    );
    assert_eq!(
        ModelFamily::from_model_id("amazon.titan-text-lite"),
        ModelFamily::Titan
    );
    assert_eq!(
        ModelFamily::from_model_id("meta.llama3-8b"),
        ModelFamily::Llama
    );
    assert_eq!(
        ModelFamily::from_model_id("unknown-model"),
        ModelFamily::Unknown
    );
}

#[test]
fn test_model_pricing_lookup() {
    use union_square::providers::bedrock::types::ModelPricing;

    // Test known models have pricing
    assert!(ModelPricing::for_model("anthropic.claude-3-opus-20240229").is_some());
    assert!(ModelPricing::for_model("amazon.titan-text-express-v1").is_some());
    assert!(ModelPricing::for_model("meta.llama3-70b-instruct-v1").is_some());

    // Test unknown model has no pricing
    assert!(ModelPricing::for_model("unknown-model-v1").is_none());
}

// Integration test scenarios that would require async context and mocked HTTP:
//
// 1. **Full Request Flow**: Test complete request forwarding with mocked HTTP client
//    - Verify SigV4 headers are preserved
//    - Verify request body is passed through unchanged
//    - Verify response headers and body are returned
//
// 2. **Streaming Response**: Test streaming endpoint with chunked responses
//    - Verify chunks are passed through without buffering
//    - Verify streaming metadata is extracted correctly
//
// 3. **Error Handling**: Test various AWS error responses
//    - ValidationException (400)
//    - ThrottlingException (429)
//    - AccessDeniedException (403)
//    - ModelNotReadyException (503)
//
// 4. **Concurrent Requests**: Test provider handles multiple concurrent requests
//    - Verify no state pollution between requests
//    - Verify proper resource cleanup
//
// 5. **Large Payloads**: Test handling of large request/response bodies
//    - Verify no memory issues with streaming
//    - Verify performance meets requirements
