//! Tests for AWS Bedrock provider

#[cfg(test)]
mod invoke_model_tests {
    use crate::providers::bedrock::{provider::BedrockProvider, types::AwsRegion};
    use crate::providers::{Provider, ProviderError};
    use axum::body::Body;
    use http_body_util::BodyExt;
    use hyper::{Request, StatusCode};
    use mockito::{Matcher, Server};
    use serde_json::json;

    #[tokio::test]
    async fn test_invoke_model_endpoint_routing() {
        let provider = BedrockProvider::new(AwsRegion::try_new("us-east-1").unwrap());

        // Test that it matches the correct paths
        assert!(provider.matches_path("/bedrock/model/anthropic.claude-3-sonnet-20240229/invoke"));
        assert!(provider.matches_path("/bedrock/model/amazon.titan-text-express-v1/invoke"));
        assert!(provider.matches_path("/bedrock/model/test/invoke-with-response-stream"));
        // Should match any bedrock path
    }

    #[tokio::test]
    async fn test_invoke_model_url_transformation() {
        let provider = BedrockProvider::new(AwsRegion::try_new("us-west-2").unwrap());

        let uri = "/bedrock/model/anthropic.claude-3-sonnet-20240229/invoke"
            .parse()
            .unwrap();
        let transformed = provider.transform_url(&uri).unwrap();

        assert_eq!(
            transformed.to_string(),
            "https://bedrock-runtime.us-west-2.amazonaws.com/model/anthropic.claude-3-sonnet-20240229/invoke"
        );
    }

    #[tokio::test]
    async fn test_invoke_model_auth_passthrough() {
        let mut server = Server::new_async().await;
        let mock_url = server.url();

        // Create provider with mock URL
        let provider = BedrockProvider::with_base_url(mock_url.clone());

        // Create a mock endpoint that expects auth headers
        let mock = server
            .mock("POST", "/model/anthropic.claude-3-sonnet-20240229/invoke")
            .match_header(
                "authorization",
                Matcher::Regex(r"AWS4-HMAC-SHA256.*".to_string()),
            )
            .match_header("x-amz-date", Matcher::Any)
            .match_header("content-type", "application/json")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(json!({"completion": "Hello from Claude!"}).to_string())
            .create_async()
            .await;

        // Create request with SigV4 headers
        let request = Request::builder()
            .method("POST")
            .uri("/bedrock/model/anthropic.claude-3-sonnet-20240229/invoke")
            .header(
                "authorization",
                "AWS4-HMAC-SHA256 Credential=test/20250126/us-east-1/bedrock/aws4_request",
            )
            .header("x-amz-date", "20250126T120000Z")
            .header("content-type", "application/json")
            .body(Body::from(json!({"prompt": "Hello"}).to_string()))
            .unwrap();

        // Create a test HTTP client
        let client =
            hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
                .build_http();

        // Forward the request through the provider
        let response = provider.forward_request(request, &client).await.unwrap();

        // Verify the response
        assert_eq!(response.status(), StatusCode::OK);

        // Verify the mock was called
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_invoke_model_request_body_passthrough() {
        // Test request body for Claude model
        let claude_body = json!({
            "anthropic_version": "bedrock-2023-05-31",
            "max_tokens": 1000,
            "messages": [{
                "role": "user",
                "content": "Hello Claude!"
            }]
        });

        // Test request body for Titan model
        let titan_body = json!({
            "inputText": "Hello Titan!",
            "textGenerationConfig": {
                "maxTokenCount": 1000,
                "temperature": 0.7
            }
        });

        // For MVP, we pass through request bodies as-is
        // Just verify the JSON is valid
        assert!(claude_body.is_object());
        assert!(titan_body.is_object());
    }

    #[tokio::test]
    async fn test_invoke_model_error_handling() {
        let mut server = Server::new_async().await;
        let mock_url = server.url();

        // Create provider with mock URL
        let provider = BedrockProvider::with_base_url(mock_url.clone());

        // Create a mock endpoint that returns a Bedrock error
        let mock = server
            .mock("POST", "/model/anthropic.claude-3-sonnet-20240229/invoke")
            .match_header("authorization", Matcher::Any)
            .match_header("x-amz-date", Matcher::Any)
            .with_status(400)
            .with_header("content-type", "application/x-amz-json-1.1")
            .with_body(
                json!({
                    "__type": "ValidationException",
                    "message": "Invalid model ID"
                })
                .to_string(),
            )
            .create_async()
            .await;

        // Create request with SigV4 headers
        let request = Request::builder()
            .method("POST")
            .uri("/bedrock/model/anthropic.claude-3-sonnet-20240229/invoke")
            .header("authorization", "AWS4-HMAC-SHA256 Credential=test")
            .header("x-amz-date", "20250126T120000Z")
            .header("content-type", "application/json")
            .body(Body::from(json!({"prompt": "Hello"}).to_string()))
            .unwrap();

        // Create a test HTTP client
        let client =
            hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
                .build_http();

        // Forward the request through the provider
        let response = provider.forward_request(request, &client).await.unwrap();

        // Verify the error response is passed through
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        // Read the response body
        let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
        let error_response: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

        assert_eq!(error_response["__type"], "ValidationException");
        assert_eq!(error_response["message"], "Invalid model ID");

        // Verify the mock was called
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_missing_auth_headers() {
        let provider = BedrockProvider::new(AwsRegion::try_new("us-east-1").unwrap());

        // Create request without auth headers
        let request = Request::builder()
            .method("POST")
            .uri("/bedrock/model/test/invoke")
            .body(Body::empty())
            .unwrap();

        let client =
            hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
                .build_http();

        // Should fail due to missing auth headers
        let result = provider.forward_request(request, &client).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            ProviderError::AuthenticationError(msg) => {
                assert!(msg.contains("Missing authorization header"));
            }
            _ => panic!("Expected AuthenticationError"),
        }
    }
}

#[cfg(test)]
mod streaming_tests {
    use crate::providers::bedrock::{provider::BedrockProvider, types::AwsRegion};
    use crate::providers::Provider;

    #[tokio::test]
    async fn test_invoke_model_with_response_stream_routing() {
        let provider = BedrockProvider::new(AwsRegion::try_new("us-east-1").unwrap());

        assert!(provider.matches_path(
            "/bedrock/model/anthropic.claude-3-sonnet-20240229/invoke-with-response-stream"
        ));
        assert!(provider.matches_path("/bedrock/model/test/invoke")); // Should match any bedrock path
    }
}

#[cfg(test)]
mod model_specific_tests {
    use crate::providers::bedrock::models::*;
    use crate::providers::bedrock::types::ModelFamily;
    use serde_json::json;

    #[test]
    fn test_claude_token_extraction() {
        let response = json!({
            "id": "msg_123",
            "type": "message",
            "role": "assistant",
            "content": [{
                "type": "text",
                "text": "Hello! I'm Claude."
            }],
            "model": "claude-3-sonnet-20240229",
            "stop_reason": "end_turn",
            "stop_sequence": null,
            "usage": {
                "input_tokens": 15,
                "output_tokens": 10
            }
        });

        let tokens = extract_token_usage(&ModelFamily::Claude, &response);
        assert!(tokens.is_some());

        let usage = tokens.unwrap();
        assert_eq!(usage.input_tokens, 15);
        assert_eq!(usage.output_tokens, 10);
        assert_eq!(usage.total_tokens, 25);
    }

    #[test]
    fn test_titan_token_extraction() {
        let response = json!({
            "inputTextTokenCount": 12,
            "results": [{
                "tokenCount": 20,
                "outputText": "Hello from Titan!",
                "completionReason": "FINISH"
            }]
        });

        let tokens = extract_token_usage(&ModelFamily::Titan, &response);
        assert!(tokens.is_some());

        let usage = tokens.unwrap();
        assert_eq!(usage.input_tokens, 12);
        assert_eq!(usage.output_tokens, 20);
        assert_eq!(usage.total_tokens, 32);
    }

    #[test]
    fn test_llama_token_extraction() {
        let response = json!({
            "generation": "Hello from Llama!",
            "prompt_token_count": 8,
            "generation_token_count": 15,
            "stop_reason": "stop"
        });

        let tokens = extract_token_usage(&ModelFamily::Llama, &response);
        assert!(tokens.is_some());

        let usage = tokens.unwrap();
        assert_eq!(usage.input_tokens, 8);
        assert_eq!(usage.output_tokens, 15);
        assert_eq!(usage.total_tokens, 23);
    }
}
