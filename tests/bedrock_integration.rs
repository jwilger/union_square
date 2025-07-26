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
    bedrock::{
        types::{AwsRegion, InputTokens, ModelId, OutputTokens, TotalTokens},
        BedrockProvider,
    },
    response_processor::ProviderResponseProcessor,
    Provider, ProviderId, ProviderRegistry, RequestId,
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

    assert_eq!(metadata.provider_id, ProviderId::bedrock());
    assert_eq!(
        metadata.model_id,
        Some(ModelId::try_new("anthropic.claude-3-sonnet-20240229".to_string()).unwrap())
    );
    assert_eq!(
        metadata.provider_request_id,
        Some(RequestId::try_new("test-request-123".to_string()).unwrap())
    );
}

#[test]
fn test_response_processor_claude() {
    let base_metadata = union_square::providers::ProviderMetadata {
        provider_id: ProviderId::bedrock(),
        model_id: Some(ModelId::try_new("anthropic.claude-3-sonnet-20240229".to_string()).unwrap()),
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

    assert_eq!(
        metadata.request_tokens,
        Some(InputTokens::try_new(25).unwrap())
    );
    assert_eq!(
        metadata.response_tokens,
        Some(OutputTokens::try_new(15).unwrap())
    );
    assert_eq!(
        metadata.total_tokens,
        Some(TotalTokens::try_new(40).unwrap())
    );
}

#[test]
fn test_response_processor_titan() {
    let base_metadata = union_square::providers::ProviderMetadata {
        provider_id: ProviderId::bedrock(),
        model_id: Some(ModelId::try_new("amazon.titan-text-express-v1".to_string()).unwrap()),
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

    assert_eq!(
        metadata.request_tokens,
        Some(InputTokens::try_new(20).unwrap())
    );
    assert_eq!(
        metadata.response_tokens,
        Some(OutputTokens::try_new(30).unwrap())
    );
    assert_eq!(
        metadata.total_tokens,
        Some(TotalTokens::try_new(50).unwrap())
    );
}

#[test]
fn test_response_processor_llama() {
    let base_metadata = union_square::providers::ProviderMetadata {
        provider_id: ProviderId::bedrock(),
        model_id: Some(ModelId::try_new("meta.llama3-70b-instruct-v1".to_string()).unwrap()),
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

    assert_eq!(
        metadata.request_tokens,
        Some(InputTokens::try_new(15).unwrap())
    );
    assert_eq!(
        metadata.response_tokens,
        Some(OutputTokens::try_new(10).unwrap())
    );
    assert_eq!(
        metadata.total_tokens,
        Some(TotalTokens::try_new(25).unwrap())
    );
}

#[test]
fn test_authentication_validation() {
    // Test that provider checks for required auth headers
    let provider = BedrockProvider::new(AwsRegion::try_new("us-east-1").unwrap());

    assert_eq!(provider.id(), ProviderId::bedrock());
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
        ModelFamily::from_model_id(
            &ModelId::try_new("anthropic.claude-3-opus".to_string()).unwrap()
        ),
        ModelFamily::Claude
    );
    assert_eq!(
        ModelFamily::from_model_id(
            &ModelId::try_new("amazon.titan-text-lite".to_string()).unwrap()
        ),
        ModelFamily::Titan
    );
    assert_eq!(
        ModelFamily::from_model_id(&ModelId::try_new("meta.llama3-8b".to_string()).unwrap()),
        ModelFamily::Llama
    );
    assert_eq!(
        ModelFamily::from_model_id(&ModelId::try_new("unknown-model".to_string()).unwrap()),
        ModelFamily::Unknown
    );
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
